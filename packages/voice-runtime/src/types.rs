//! Data types for audio buffers, STT transcriptions, TTS synthesis outputs, and voice states.

/// State of the voice processing pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceState {
    Idle,
    Listening,
    Transcribing,
    Synthesizing,
    Speaking,
    Error,
}

/// Raw audio buffer payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub pcm_data: Vec<u8>,
}

impl AudioBuffer {
    /// Creates a new PCM audio buffer.
    pub fn new(sample_rate: u32, channels: u16, pcm_data: Vec<u8>) -> Self {
        Self {
            sample_rate,
            channels,
            pcm_data,
        }
    }

    /// Returns total sample count (assuming 16-bit i16 PCM samples).
    pub fn sample_count(&self) -> usize {
        self.pcm_data.len() / 2
    }

    /// Calculates audio duration in milliseconds based on sample rate and channel count.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0;
        }
        let samples = self.sample_count();
        ((samples as u64) * 1000) / (self.sample_rate as u64 * self.channels as u64)
    }
}

/// Result produced by Whisper STT speech transcription.
#[derive(Clone, Debug, PartialEq)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub duration_ms: u64,
}

/// Result produced by Piper TTS speech synthesis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynthesisResult {
    pub audio: AudioBuffer,
    pub duration_ms: u64,
}

/// Result produced by a full end-to-end cognitive voice turn (STT -> LLM -> TTS).
#[derive(Clone, Debug, PartialEq)]
pub struct CognitiveTurnResult {
    pub transcription: TranscriptionResult,
    pub stt_latency_ms: u64,
    pub llm_response: model_runtime::ModelResponse,
    pub llm_latency_ms: u64,
    pub llm_ttft_ms: u64,
    pub llm_tokens_per_sec: f64,
    pub synthesis: SynthesisResult,
    pub tts_latency_ms: u64,
    pub total_latency_ms: u64,
}

/// An individual synthesized audio chunk generated during streaming TTS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynthesizedAudioChunk {
    pub chunk_index: usize,
    pub text: String,
    pub audio: AudioBuffer,
    pub latency_ms: u64,
}

/// High-resolution instrumentation timeline capturing all critical milestones (T0 through T10).
#[derive(Clone, Debug, PartialEq)]
pub struct StreamTimeline {
    pub t0_request_accepted: std::time::Instant,
    pub t1_whisper_start: std::time::Instant,
    pub t2_whisper_complete: std::time::Instant,
    pub t3_qwen_start: std::time::Instant,
    pub t4_first_qwen_token: std::time::Instant,
    pub t5_first_text_chunk: std::time::Instant,
    pub t6_piper_first_chunk_start: std::time::Instant,
    pub t7_first_audio_generated: std::time::Instant,
    pub t8_first_audio_available: std::time::Instant,
    pub t9_qwen_final_token: std::time::Instant,
    pub t10_final_piper_audio: std::time::Instant,

    pub whisper_latency_ms: f64,
    pub qwen_ttft_ms: f64,
    pub time_to_first_chunk_ms: f64,
    pub piper_first_chunk_latency_ms: f64,
    pub time_to_first_audio_ms: f64,
    pub qwen_tokens_per_sec: f64,
    pub total_completion_ms: f64,
    pub overlap_duration_ms: f64,
}

/// Result produced by an incremental streaming cognitive voice turn.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamingTurnResult {
    pub transcription: TranscriptionResult,
    pub full_text: String,
    pub text_chunks: Vec<String>,
    pub audio_chunks: Vec<SynthesizedAudioChunk>,
    pub composite_audio: AudioBuffer,
    pub total_tokens: usize,
    pub timeline: StreamTimeline,
}
