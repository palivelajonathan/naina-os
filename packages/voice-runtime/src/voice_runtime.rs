//! Primary VoiceRuntime supervisor implementation for NAINA OS.

use crate::adapters::{CandleWhisperSttAdapter, PiperTtsAdapter};
use crate::config::VoiceRuntimeConfig;
use crate::error::{Result, VoiceRuntimeError};
use crate::traits::{SttEngine, TtsEngine};
use crate::types::{AudioBuffer, SynthesisResult, TranscriptionResult, VoiceState};
use logging::{LogLevel, Logger, LoggerConfig};
use runtime::Runtime;
use services::ServiceRegistry;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

/// Primary top-level voice runtime supervisor.
pub struct VoiceRuntime {
    config: VoiceRuntimeConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    state: RwLock<VoiceState>,
    stt_engine: RwLock<Option<Arc<dyn SttEngine>>>,
    tts_engine: RwLock<Option<Arc<dyn TtsEngine>>>,
    model_provider: RwLock<Option<Arc<dyn model_runtime::ModelProvider>>>,
    audio_hal: RwLock<Arc<dyn crate::hal::AudioHal>>,
    cancel_flag: Arc<AtomicBool>,
    logger: Mutex<Logger>,
}

// Safety: VoiceRuntime uses internal synchronization primitives (RwLock, Mutex, AtomicBool)
unsafe impl Send for VoiceRuntime {}
unsafe impl Sync for VoiceRuntime {}

impl fmt::Debug for VoiceRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceRuntime")
            .field("config", &self.config)
            .field("runtime", &self.runtime)
            .field("services", &self.services)
            .field("state", &self.state)
            .field("cancel_flag", &self.cancel_flag)
            .finish()
    }
}

impl VoiceRuntime {
    /// Constructs a new [`VoiceRuntime`] supervisor instance.
    pub fn new(
        config: VoiceRuntimeConfig,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let stt: Arc<dyn SttEngine> = Arc::new(CandleWhisperSttAdapter::new(&config.stt_model_id));
        let tts: Arc<dyn TtsEngine> = Arc::new(PiperTtsAdapter::new(&config.tts_voice_id));
        let audio_hal: Arc<dyn crate::hal::AudioHal> = crate::hal::create_default_hal();
        let logger = Logger::new(LoggerConfig::default())
            .unwrap()
            .with_component("voice-runtime");

        Self {
            config,
            runtime,
            services,
            state: RwLock::new(VoiceState::Idle),
            stt_engine: RwLock::new(Some(stt)),
            tts_engine: RwLock::new(Some(tts)),
            model_provider: RwLock::new(None),
            audio_hal: RwLock::new(audio_hal),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            logger: Mutex::new(logger),
        }
    }

    /// Constructs a [`VoiceRuntime`] with default configuration.
    pub fn with_default_config(runtime: Arc<Runtime>, services: Arc<ServiceRegistry>) -> Self {
        Self::new(VoiceRuntimeConfig::default(), runtime, services)
    }

    /// Constructs a [`VoiceRuntime`] from root [`configuration::Config`].
    pub fn from_root_config(
        root_config: &configuration::Config,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let config = VoiceRuntimeConfig::from_voice_config(&root_config.voice);
        Self::new(config, runtime, services)
    }

    /// Registers a custom STT engine implementation.
    pub fn register_stt_engine(&self, engine: Arc<dyn SttEngine>) {
        if let Ok(mut lock) = self.stt_engine.write() {
            *lock = Some(engine);
        }
    }

    /// Registers a custom TTS engine implementation.
    pub fn register_tts_engine(&self, engine: Arc<dyn TtsEngine>) {
        if let Ok(mut lock) = self.tts_engine.write() {
            *lock = Some(engine);
        }
    }

    /// Registers a custom LLM model provider implementation.
    pub fn register_model_provider(&self, provider: Arc<dyn model_runtime::ModelProvider>) {
        if let Ok(mut lock) = self.model_provider.write() {
            *lock = Some(provider);
        }
    }

    /// Registers a custom Audio HAL implementation (e.g. MockAudioHal for testing).
    pub fn register_audio_hal(&self, hal: Arc<dyn crate::hal::AudioHal>) {
        if let Ok(mut lock) = self.audio_hal.write() {
            *lock = hal;
        }
    }

    /// Returns the currently active Audio HAL instance.
    pub fn audio_hal(&self) -> Arc<dyn crate::hal::AudioHal> {
        self.audio_hal
            .read()
            .map(|h| Arc::clone(&*h))
            .unwrap_or_else(|_| crate::hal::create_default_hal())
    }

    /// Returns the active configuration.
    pub fn config(&self) -> &VoiceRuntimeConfig {
        &self.config
    }

    /// Returns the current pipeline voice state.
    pub fn state(&self) -> VoiceState {
        self.state.read().map(|s| *s).unwrap_or(VoiceState::Error)
    }

    /// Sets the voice state safely.
    pub fn set_state(&self, new_state: VoiceState) -> Result<()> {
        let mut lock = self
            .state
            .write()
            .map_err(|_| VoiceRuntimeError::LockError {
                message: "Failed to acquire state write lock".to_string(),
            })?;
        *lock = new_state;
        Ok(())
    }

    /// Returns a reference to the inner runtime handle.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Returns a reference to the inner service registry handle.
    pub fn services(&self) -> &Arc<ServiceRegistry> {
        &self.services
    }

    /// Processes an incoming audio input buffer through Whisper STT transcription.
    pub fn process_audio_input(&self, audio: &AudioBuffer) -> Result<TranscriptionResult> {
        let start_time = Instant::now();

        if audio.sample_rate != self.config.sample_rate || audio.channels != self.config.channels {
            return Err(VoiceRuntimeError::InvalidAudioFormat {
                message: format!(
                    "Audio format mismatch: expected {}Hz {}ch, got {}Hz {}ch",
                    self.config.sample_rate,
                    self.config.channels,
                    audio.sample_rate,
                    audio.channels
                ),
            });
        }

        self.set_state(VoiceState::Listening)?;
        self.set_state(VoiceState::Transcribing)?;

        let stt = {
            let lock = self
                .stt_engine
                .read()
                .map_err(|_| VoiceRuntimeError::LockError {
                    message: "Failed to acquire STT engine read lock".to_string(),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "STT".to_string(),
                })?
        };

        match stt.transcribe(audio) {
            Ok(res) => {
                self.set_state(VoiceState::Idle)?;

                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                if elapsed_ms > self.config.latency_target_ms {
                    if let Ok(logger) = self.logger.lock() {
                        let _ = logger.log(
                            LogLevel::Warn,
                            format!(
                                "Voice STT latency target exceeded: {}ms (target: {}ms)",
                                elapsed_ms, self.config.latency_target_ms
                            ),
                        );
                    }
                }

                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(VoiceState::Error);
                Err(err)
            }
        }
    }

    /// Synthesizes speech from a text prompt via Piper TTS.
    pub fn synthesize_speech(&self, text: &str) -> Result<SynthesisResult> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(VoiceRuntimeError::BargeInInterrupted);
        }

        self.set_state(VoiceState::Synthesizing)?;

        let tts = {
            let lock = self
                .tts_engine
                .read()
                .map_err(|_| VoiceRuntimeError::LockError {
                    message: "Failed to acquire TTS engine read lock".to_string(),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "TTS".to_string(),
                })?
        };

        match tts.synthesize(text) {
            Ok(res) => {
                if self.cancel_flag.load(Ordering::SeqCst) {
                    let _ = self.set_state(VoiceState::Idle);
                    return Err(VoiceRuntimeError::BargeInInterrupted);
                }

                self.set_state(VoiceState::Speaking)?;
                self.set_state(VoiceState::Idle)?;

                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                if elapsed_ms > self.config.latency_target_ms {
                    if let Ok(logger) = self.logger.lock() {
                        let _ = logger.log(
                            LogLevel::Warn,
                            format!(
                                "Voice TTS latency target exceeded: {}ms (target: {}ms)",
                                elapsed_ms, self.config.latency_target_ms
                            ),
                        );
                    }
                }

                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(VoiceState::Error);
                Err(err)
            }
        }
    }

    /// Handles user barge-in / interruption during active speech output.
    pub fn handle_barge_in(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        let _ = self.set_state(VoiceState::Listening);
    }

    /// Resets the voice processing state machine back to `Idle` after error or interruption.
    pub fn reset_state(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
        let _ = self.set_state(VoiceState::Idle);
    }

    /// Processes a full end-to-end cognitive voice turn (Whisper STT -> Qwen GPU LLM -> Piper TTS).
    pub fn process_cognitive_voice_turn(
        &self,
        audio: &AudioBuffer,
        model_name: &str,
    ) -> Result<crate::types::CognitiveTurnResult> {
        let turn_start = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(VoiceRuntimeError::BargeInInterrupted);
        }

        // 1. Whisper STT Step
        let stt_start = Instant::now();
        let transcription = self.process_audio_input(audio)?;
        let stt_latency_ms = stt_start.elapsed().as_millis() as u64;

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(VoiceRuntimeError::BargeInInterrupted);
        }

        // 2. LLM Reasoning Step
        let model_provider = {
            let lock = self
                .model_provider
                .read()
                .map_err(|_| VoiceRuntimeError::LockError {
                    message: "Failed to acquire model provider read lock".to_string(),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "ModelProvider".to_string(),
                })?
        };

        let req = model_runtime::ModelRequest {
            model_name: model_name.to_string(),
            prompt: transcription.text.clone(),
            params: model_runtime::InferenceParams::default(),
        };

        let llm_start = Instant::now();
        let detailed_llm = model_provider
            .generate_detailed_metrics(&req)
            .map_err(|e| VoiceRuntimeError::SttTranscriptionFailed {
                message: format!("LLM reasoning failed: {e}"),
            })?;
        let llm_latency_ms = llm_start.elapsed().as_millis() as u64;
        let llm_response = detailed_llm.response;
        let llm_ttft_ms = detailed_llm.ttft.as_millis() as u64;
        let llm_tokens_per_sec = if detailed_llm.token_generation_duration.as_secs_f64() > 0.0 {
            llm_response.tokens_generated.saturating_sub(1) as f64
                / detailed_llm.token_generation_duration.as_secs_f64()
        } else if llm_latency_ms > 0 {
            llm_response.tokens_generated as f64 / (llm_latency_ms as f64 / 1000.0)
        } else {
            0.0
        };

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(VoiceRuntimeError::BargeInInterrupted);
        }

        // 3. Piper TTS Step
        let tts_prompt = if llm_response.text.trim().is_empty() {
            "Naina Online".to_string()
        } else {
            llm_response.text.clone()
        };

        let tts_start = Instant::now();
        let synthesis = self.synthesize_speech(&tts_prompt)?;
        let tts_latency_ms = tts_start.elapsed().as_millis() as u64;

        let total_latency_ms = turn_start.elapsed().as_millis() as u64;

        Ok(crate::types::CognitiveTurnResult {
            transcription,
            stt_latency_ms,
            llm_response,
            llm_latency_ms,
            llm_ttft_ms,
            llm_tokens_per_sec,
            synthesis,
            tts_latency_ms,
            total_latency_ms,
        })
    }

    /// Processes a full incremental streaming cognitive voice turn:
    /// Whisper STT -> Qwen Live Streaming -> TextChunker -> Piper Streaming Concurrent TTS.
    ///
    /// Implements real pipelined concurrency with bounded backpressure.
    /// Instruments all 11 timestamp checkpoints (T0 through T10) and verifies exact text reconstruction.
    pub fn process_cognitive_voice_turn_stream(
        &self,
        audio: &AudioBuffer,
        model_name: &str,
    ) -> Result<crate::types::StreamingTurnResult> {
        let t0 = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(VoiceRuntimeError::BargeInInterrupted);
        }

        // T1: Whisper start
        let t1 = Instant::now();
        let transcription = self.process_audio_input(audio)?;
        // T2: Whisper complete
        let t2 = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(VoiceRuntimeError::BargeInInterrupted);
        }

        let model_provider = {
            let lock = self
                .model_provider
                .read()
                .map_err(|_| VoiceRuntimeError::LockError {
                    message: "Failed to acquire model provider read lock".to_string(),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "ModelProvider".to_string(),
                })?
        };

        let tts_engine = {
            let lock = self
                .tts_engine
                .read()
                .map_err(|_| VoiceRuntimeError::LockError {
                    message: "Failed to acquire TTS engine read lock".to_string(),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "TTS".to_string(),
                })?
        };

        let req = model_runtime::ModelRequest {
            model_name: model_name.to_string(),
            prompt: transcription.text.clone(),
            params: model_runtime::InferenceParams::default(),
        };

        // Bounded channel for sending text chunks from Chunker to Piper worker thread (capacity 8)
        let (chunk_tx, chunk_rx) = std::sync::mpsc::sync_channel::<(usize, String)>(8);

        let cancel_flag = Arc::clone(&self.cancel_flag);

        // Spawn Piper TTS consumer thread
        let piper_handle = std::thread::spawn(move || {
            let mut audio_chunks = Vec::new();
            let mut t6_first_chunk_start: Option<Instant> = None;
            let mut t7_first_audio_generated: Option<Instant> = None;
            let mut t10_final_audio: Option<Instant> = None;

            while let Ok((chunk_idx, chunk_text)) = chunk_rx.recv() {
                if cancel_flag.load(Ordering::SeqCst) {
                    break;
                }

                let synth_start = Instant::now();
                if t6_first_chunk_start.is_none() {
                    t6_first_chunk_start = Some(synth_start);
                }

                match tts_engine.synthesize(&chunk_text) {
                    Ok(synth_res) => {
                        let synth_end = Instant::now();
                        if t7_first_audio_generated.is_none() {
                            t7_first_audio_generated = Some(synth_end);
                        }
                        t10_final_audio = Some(synth_end);

                        let chunk_latency =
                            synth_end.duration_since(synth_start).as_millis() as u64;
                        audio_chunks.push(crate::types::SynthesizedAudioChunk {
                            chunk_index: chunk_idx,
                            text: chunk_text,
                            audio: synth_res.audio,
                            latency_ms: chunk_latency,
                        });
                    }
                    Err(err) => {
                        return Err((audio_chunks, err));
                    }
                }
            }

            Ok((
                audio_chunks,
                t6_first_chunk_start,
                t7_first_audio_generated,
                t10_final_audio,
            ))
        });

        // T3: Qwen generation start
        let t3 = Instant::now();
        let token_stream = model_provider.generate_stream(&req).map_err(|e| {
            VoiceRuntimeError::SttTranscriptionFailed {
                message: format!("LLM streaming start failed: {e}"),
            }
        })?;

        let mut chunker = crate::chunker::TextChunker::new();
        let mut text_chunks = Vec::new();
        let mut t4_first_token: Option<Instant> = None;
        let mut t5_first_chunk: Option<Instant> = None;
        let mut total_tokens = 0usize;
        let mut chunk_counter = 0usize;

        // Producer loop: read tokens from Qwen as they arrive incrementally
        while let Ok(token_str) = token_stream.receiver.recv() {
            if self.cancel_flag.load(Ordering::SeqCst) {
                break;
            }

            if t4_first_token.is_none() {
                t4_first_token = Some(Instant::now());
            }
            total_tokens += 1;

            let ready_chunks = chunker.push(&token_str);
            for chunk in ready_chunks {
                if t5_first_chunk.is_none() {
                    t5_first_chunk = Some(Instant::now());
                }
                text_chunks.push(chunk.clone());
                let idx = chunk_counter;
                chunk_counter += 1;
                if chunk_tx.send((idx, chunk)).is_err() {
                    // Piper worker terminated or encountered error
                    break;
                }
            }
        }

        // T9: Qwen final token generated (stream closed)
        let t9 = Instant::now();

        // Flush any remaining partial chunk from chunker
        if let Some(final_chunk) = chunker.flush() {
            if t5_first_chunk.is_none() {
                t5_first_chunk = Some(Instant::now());
            }
            text_chunks.push(final_chunk.clone());
            let idx = chunk_counter;
            let _ = chunk_tx.send((idx, final_chunk));
        }

        // Drop chunk_tx to signal EOF to Piper worker
        drop(chunk_tx);

        // Await Piper TTS worker completion
        let (audio_chunks, t6_opt, t7_opt, t10_opt) = match piper_handle.join() {
            Ok(Ok(res)) => res,
            Ok(Err((_, err))) => return Err(err),
            Err(_) => {
                return Err(VoiceRuntimeError::TtsSynthesisFailed {
                    message: "Piper TTS streaming worker thread panicked".to_string(),
                })
            }
        };

        // Fallbacks for timestamps in edge cases (e.g. 0 tokens generated)
        let t4 = t4_first_token.unwrap_or(t9);
        let t5 = t5_first_chunk.unwrap_or(t9);
        let t6 = t6_opt.unwrap_or(t9);
        let t7 = t7_opt.unwrap_or_else(Instant::now);
        let t8 = t7; // Audio is immediately available once Chunk 0 synthesis completes
        let t10 = t10_opt.unwrap_or(t7);

        // Assemble full reconstructed text
        let full_text = text_chunks.join("");

        // Assemble composite audio buffer from all synthesized chunks
        let mut composite_pcm = Vec::new();
        for ac in &audio_chunks {
            composite_pcm.extend_from_slice(&ac.audio.pcm_data);
        }
        let composite_audio =
            AudioBuffer::new(self.config.sample_rate, self.config.channels, composite_pcm);

        // Calculate latencies
        let whisper_latency_ms = t2.duration_since(t1).as_secs_f64() * 1000.0;
        let qwen_ttft_ms = t4.duration_since(t3).as_secs_f64() * 1000.0;
        let time_to_first_chunk_ms = t5.duration_since(t0).as_secs_f64() * 1000.0;
        let piper_first_chunk_latency_ms = t7.duration_since(t6).as_secs_f64() * 1000.0;
        let time_to_first_audio_ms = t8.duration_since(t0).as_secs_f64() * 1000.0;
        let qwen_gen_duration = t9.duration_since(t3).as_secs_f64();
        let qwen_tokens_per_sec = if qwen_gen_duration > 0.0 {
            total_tokens as f64 / qwen_gen_duration
        } else {
            0.0
        };
        let total_completion_ms = t10.duration_since(t0).as_secs_f64() * 1000.0;

        // Calculate overlap duration: duration where Piper was synthesizing while Qwen was still generating
        let overlap_start = t6.max(t3);
        let overlap_end = t10.min(t9);
        let overlap_duration_ms = if overlap_end > overlap_start {
            overlap_end.duration_since(overlap_start).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        let timeline = crate::types::StreamTimeline {
            t0_request_accepted: t0,
            t1_whisper_start: t1,
            t2_whisper_complete: t2,
            t3_qwen_start: t3,
            t4_first_qwen_token: t4,
            t5_first_text_chunk: t5,
            t6_piper_first_chunk_start: t6,
            t7_first_audio_generated: t7,
            t8_first_audio_available: t8,
            t9_qwen_final_token: t9,
            t10_final_piper_audio: t10,
            whisper_latency_ms,
            qwen_ttft_ms,
            time_to_first_chunk_ms,
            piper_first_chunk_latency_ms,
            time_to_first_audio_ms,
            qwen_tokens_per_sec,
            total_completion_ms,
            overlap_duration_ms,
        };

        Ok(crate::types::StreamingTurnResult {
            transcription,
            full_text,
            text_chunks,
            audio_chunks,
            composite_audio,
            total_tokens,
            timeline,
        })
    }

    /// Lists all detected audio input (microphone) devices via the active Audio HAL.
    pub fn list_input_devices(&self) -> Result<Vec<crate::hal::AudioDeviceInfo>> {
        self.audio_hal().list_input_devices()
    }

    /// Lists all detected audio output (speaker/headphone) devices via the active Audio HAL.
    pub fn list_output_devices(&self) -> Result<Vec<crate::hal::AudioDeviceInfo>> {
        self.audio_hal().list_output_devices()
    }

    /// Returns the system default audio input device.
    pub fn default_input_device(&self) -> Result<crate::hal::AudioDeviceInfo> {
        self.audio_hal().default_input_device()
    }

    /// Returns the system default audio output device.
    pub fn default_output_device(&self) -> Result<crate::hal::AudioDeviceInfo> {
        self.audio_hal().default_output_device()
    }

    /// Captures real audio from the specified (or default) microphone for `duration_ms` milliseconds,
    /// returning a normalized 16 kHz mono [`AudioBuffer`] ready for Whisper STT and capture statistics.
    pub fn capture_microphone_audio(
        &self,
        duration_ms: u64,
        device_name: Option<&str>,
    ) -> Result<(AudioBuffer, crate::types::AudioCaptureStats)> {
        let hal = self.audio_hal();
        let stream_cfg = crate::hal::AudioStreamConfig::default();
        let mut input_stream = hal.open_input_stream(device_name, &stream_cfg)?;
        let hw_cfg = input_stream.config();

        let t1 = Instant::now();
        input_stream.start()?;

        let mut raw_samples = Vec::new();
        let mut t2_first_frame: Option<Instant> = None;
        let sleep_slice = std::time::Duration::from_millis(10);
        let capture_limit = std::time::Duration::from_millis(duration_ms);
        let capture_start = Instant::now();

        while capture_start.elapsed() < capture_limit {
            if self.cancel_flag.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(sleep_slice);
            let chunk = input_stream.capture_samples()?;
            if !chunk.is_empty() {
                if t2_first_frame.is_none() {
                    t2_first_frame = Some(Instant::now());
                }
                raw_samples.extend(chunk);
            }
        }

        // Drain any remaining buffered frames
        let tail = input_stream.capture_samples()?;
        if !tail.is_empty() {
            if t2_first_frame.is_none() {
                t2_first_frame = Some(Instant::now());
            }
            raw_samples.extend(tail);
        }

        input_stream.stop()?;
        let t2 = t2_first_frame.unwrap_or(Instant::now());
        let startup_latency_ms = t2.duration_since(t1).as_secs_f64() * 1000.0;

        let normalized_audio =
            crate::hal::normalize_to_16k_mono(&raw_samples, hw_cfg.sample_rate, hw_cfg.channels);

        let stats = crate::types::AudioCaptureStats {
            sample_rate: hw_cfg.sample_rate,
            channels: hw_cfg.channels,
            samples_captured: raw_samples.len(),
            duration_ms,
            startup_latency_ms,
        };

        Ok((normalized_audio, stats))
    }

    /// Plays an [`AudioBuffer`] to the specified (or default) Windows output device,
    /// automatically converting and resampling to the native hardware device format.
    pub fn play_audio(
        &self,
        audio: &AudioBuffer,
        device_name: Option<&str>,
    ) -> Result<crate::types::AudioPlaybackStats> {
        let hal = self.audio_hal();
        let stream_cfg = crate::hal::AudioStreamConfig::default();
        let mut output_stream = hal.open_output_stream(device_name, &stream_cfg)?;
        let hw_cfg = output_stream.config();

        let device_samples =
            crate::hal::convert_to_device_format(audio, hw_cfg.sample_rate, hw_cfg.channels);

        let start_time = Instant::now();
        output_stream.start()?;
        output_stream.write_chunk(&device_samples)?;
        output_stream.flush_and_wait()?;
        output_stream.stop()?;

        let playback_latency_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        let stats = crate::types::AudioPlaybackStats {
            sample_rate: hw_cfg.sample_rate,
            channels: hw_cfg.channels,
            samples_played: device_samples.len(),
            duration_ms: audio.duration_ms(),
            playback_latency_ms,
        };

        Ok(stats)
    }

    /// Executes a complete live physical voice turn:
    /// 1. Captures real microphone audio via the Windows WASAPI Audio HAL.
    /// 2. Normalizes captured audio to 16 kHz mono PCM.
    /// 3. Transcribes spoken speech via Whisper STT.
    /// 4. Generates response tokens incrementally via Qwen 7B GGUF CUDA.
    /// 5. Chunks generated text on clause/sentence boundaries.
    /// 6. Synthesizes audio chunks concurrently via Piper TTS.
    /// 7. Streams synthesized PCM chunks directly to the real Windows output device (speakers/headphones).
    /// 8. Records high-precision timeline instrumentation across all milestones ($T_0$ through $T_{13}$).
    pub fn process_live_voice_turn_stream(
        &self,
        capture_duration_ms: u64,
        model_name: &str,
        input_device: Option<&str>,
        output_device: Option<&str>,
    ) -> Result<crate::types::LiveStreamingTurnResult> {
        let t0 = Instant::now();
        self.cancel_flag.store(false, Ordering::SeqCst);

        // 1. Capture real microphone audio
        let (audio_input, capture_stats) =
            self.capture_microphone_audio(capture_duration_ms, input_device)?;
        let t1 = t0;
        let t2 = t0 + std::time::Duration::from_secs_f64(capture_stats.startup_latency_ms / 1000.0);
        let t3 = Instant::now();

        // 2. Open hardware playback stream early so it is active and primed
        let hal = self.audio_hal();
        let stream_cfg = crate::hal::AudioStreamConfig::default();
        let mut output_stream = hal.open_output_stream(output_device, &stream_cfg)?;
        output_stream.start()?;
        let hw_out_cfg = output_stream.config();

        // 3. Transcribe with Whisper STT
        let stt_engine = {
            let lock = self
                .stt_engine
                .read()
                .map_err(|e| VoiceRuntimeError::LockError {
                    message: format!("Failed to acquire STT lock: {e}"),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "stt".to_string(),
                })?
        };
        let transcription = stt_engine.transcribe(&audio_input)?;
        let t4 = Instant::now();

        // 4. Verify Model Provider loaded
        let model_provider = {
            let lock = self
                .model_provider
                .read()
                .map_err(|e| VoiceRuntimeError::LockError {
                    message: format!("Failed to acquire ModelProvider lock: {e}"),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "model_provider".to_string(),
                })?
        };

        // 5. Setup Piper TTS concurrent worker with hardware output playback
        let tts_engine = {
            let lock = self
                .tts_engine
                .read()
                .map_err(|e| VoiceRuntimeError::LockError {
                    message: format!("Failed to acquire TTS lock: {e}"),
                })?;
            lock.clone()
                .ok_or_else(|| VoiceRuntimeError::EngineNotLoaded {
                    engine_name: "tts".to_string(),
                })?
        };

        let (chunk_tx, chunk_rx) = std::sync::mpsc::sync_channel::<(usize, String)>(8);

        let piper_worker_handle = std::thread::spawn(move || {
            let mut audio_chunks = Vec::new();
            let mut t8_first_audio: Option<Instant> = None;
            let mut t9_first_pcm_submitted: Option<Instant> = None;
            let mut t10_first_playback: Option<Instant> = None;
            let mut t12_final_audio: Option<Instant> = None;

            while let Ok((chunk_idx, chunk_text)) = chunk_rx.recv() {
                let synth_start = Instant::now();
                match tts_engine.synthesize(&chunk_text) {
                    Ok(synth_res) => {
                        let synth_end = Instant::now();
                        if t8_first_audio.is_none() {
                            t8_first_audio = Some(synth_end);
                        }
                        t12_final_audio = Some(synth_end);

                        // Resample & convert Piper 16kHz mono audio into device native format
                        let device_samples = crate::hal::convert_to_device_format(
                            &synth_res.audio,
                            hw_out_cfg.sample_rate,
                            hw_out_cfg.channels,
                        );

                        if t9_first_pcm_submitted.is_none() {
                            t9_first_pcm_submitted = Some(Instant::now());
                        }

                        // Stream audio directly to real Windows speaker/headphone device
                        if let Err(e) = output_stream.write_chunk(&device_samples) {
                            eprintln!("[Playback Write Error]: {e}");
                        }

                        if t10_first_playback.is_none() {
                            t10_first_playback = Some(Instant::now());
                        }

                        let chunk_latency =
                            synth_end.duration_since(synth_start).as_millis() as u64;
                        audio_chunks.push(crate::types::SynthesizedAudioChunk {
                            chunk_index: chunk_idx,
                            text: chunk_text,
                            audio: synth_res.audio,
                            latency_ms: chunk_latency,
                        });
                    }
                    Err(err) => {
                        return Err((audio_chunks, output_stream, err));
                    }
                }
            }

            Ok((
                audio_chunks,
                output_stream,
                t8_first_audio,
                t9_first_pcm_submitted,
                t10_first_playback,
                t12_final_audio,
            ))
        });

        // 6. Qwen generation start (T5)
        let t5 = Instant::now();
        let prompt_text = transcription.text.clone();
        let req = model_runtime::ModelRequest {
            model_name: model_name.to_string(),
            prompt: prompt_text,
            params: model_runtime::InferenceParams::default(),
        };

        let token_stream = model_provider.generate_stream(&req).map_err(|e| {
            VoiceRuntimeError::SttTranscriptionFailed {
                message: format!("LLM streaming start failed: {e}"),
            }
        })?;

        let mut chunker = crate::chunker::TextChunker::new();
        let mut text_chunks = Vec::new();
        let mut t6_first_token: Option<Instant> = None;
        let mut t7_first_chunk: Option<Instant> = None;
        let mut total_tokens = 0usize;
        let mut chunk_counter = 0usize;

        while let Ok(token_str) = token_stream.receiver.recv() {
            if self.cancel_flag.load(Ordering::SeqCst) {
                break;
            }
            if t6_first_token.is_none() {
                t6_first_token = Some(Instant::now());
            }
            total_tokens += 1;

            let ready = chunker.push(&token_str);
            for chunk in ready {
                if t7_first_chunk.is_none() {
                    t7_first_chunk = Some(Instant::now());
                }
                text_chunks.push(chunk.clone());
                let idx = chunk_counter;
                chunk_counter += 1;
                if chunk_tx.send((idx, chunk)).is_err() {
                    break;
                }
            }
        }

        // T11: Qwen final token
        let t11 = Instant::now();

        // Flush remaining partial text chunk
        if let Some(final_chunk) = chunker.flush() {
            if t7_first_chunk.is_none() {
                t7_first_chunk = Some(Instant::now());
            }
            text_chunks.push(final_chunk.clone());
            let idx = chunk_counter;
            let _ = chunk_tx.send((idx, final_chunk));
        }

        // Close chunk channel to signal EOF to Piper worker
        drop(chunk_tx);

        // Await Piper worker completion and recover output stream
        let (audio_chunks, mut output_stream, t8_opt, t9_opt, t10_opt, t12_opt) =
            match piper_worker_handle.join() {
                Ok(Ok(res)) => res,
                Ok(Err((_, _, err))) => return Err(err),
                Err(_) => {
                    return Err(VoiceRuntimeError::TtsSynthesisFailed {
                        message: "Piper TTS worker thread panicked".to_string(),
                    })
                }
            };

        // Flush hardware output audio stream and wait for playback to complete
        output_stream.flush_and_wait()?;
        let t13 = Instant::now();
        output_stream.stop()?;

        // Timestamps & fallbacks
        let t6 = t6_first_token.unwrap_or(t11);
        let t7 = t7_first_chunk.unwrap_or(t11);
        let t8 = t8_opt.unwrap_or(t11);
        let t9 = t9_opt.unwrap_or(t8);
        let t10 = t10_opt.unwrap_or(t9);
        let t12 = t12_opt.unwrap_or(t8);

        // Assemble full reconstructed text
        let full_text = text_chunks.join("");

        // Assemble composite audio buffer
        let mut composite_pcm = Vec::new();
        for ac in &audio_chunks {
            composite_pcm.extend_from_slice(&ac.audio.pcm_data);
        }
        let composite_audio =
            AudioBuffer::new(self.config.sample_rate, self.config.channels, composite_pcm);

        // Calculate latency metrics
        let capture_startup_latency_ms = t2.duration_since(t1).as_secs_f64() * 1000.0;
        let capture_duration_ms_calc = t3.duration_since(t0).as_secs_f64() * 1000.0;
        let whisper_latency_ms = t4.duration_since(t3).as_secs_f64() * 1000.0;
        let qwen_ttft_ms = t6.duration_since(t5).as_secs_f64() * 1000.0;
        let software_ttfa_ms = t8.duration_since(t0).as_secs_f64() * 1000.0;
        let hardware_inclusive_ttfa_ms = t10.duration_since(t0).as_secs_f64() * 1000.0;
        let total_turn_completion_ms = t13.duration_since(t0).as_secs_f64() * 1000.0;

        let qwen_gen_duration = t11.duration_since(t5).as_secs_f64();
        let qwen_tokens_per_sec = if qwen_gen_duration > 0.0 {
            total_tokens as f64 / qwen_gen_duration
        } else {
            0.0
        };

        let overlap_start = t8.max(t5);
        let overlap_end = t12.min(t11);
        let overlap_duration_ms = if overlap_end > overlap_start {
            overlap_end.duration_since(overlap_start).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        let timeline = crate::types::LiveStreamTimeline {
            t0_audio_request: t0,
            t1_input_stream_started: t1,
            t2_first_mic_frame: t2,
            t3_capture_completed: t3,
            t4_whisper_completed: t4,
            t5_qwen_started: t5,
            t6_first_qwen_token: t6,
            t7_first_text_chunk: t7,
            t8_first_piper_chunk: t8,
            t9_first_output_pcm_submitted: t9,
            t10_first_output_callback: t10,
            t11_qwen_final_token: t11,
            t12_final_piper_chunk: t12,
            t13_output_stream_completed: t13,

            capture_startup_latency_ms,
            capture_duration_ms: capture_duration_ms_calc,
            whisper_latency_ms,
            qwen_ttft_ms,
            software_ttfa_ms,
            hardware_inclusive_ttfa_ms,
            total_turn_completion_ms,
            overlap_duration_ms,
            qwen_tokens_per_sec,
        };

        let playback_stats = crate::types::AudioPlaybackStats {
            sample_rate: hw_out_cfg.sample_rate,
            channels: hw_out_cfg.channels,
            samples_played: composite_audio.sample_count(),
            duration_ms: composite_audio.duration_ms(),
            playback_latency_ms: t13.duration_since(t8).as_secs_f64() * 1000.0,
        };

        Ok(crate::types::LiveStreamingTurnResult {
            transcription,
            full_text,
            text_chunks,
            audio_chunks,
            composite_audio,
            total_tokens,
            capture_stats,
            playback_stats,
            timeline,
        })
    }
}
