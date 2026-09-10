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
        let llm_response = model_provider.generate(&req).map_err(|e| {
            VoiceRuntimeError::SttTranscriptionFailed {
                message: format!("LLM reasoning failed: {e}"),
            }
        })?;
        let llm_latency_ms = llm_start.elapsed().as_millis() as u64;

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
            synthesis,
            tts_latency_ms,
            total_latency_ms,
        })
    }
}
