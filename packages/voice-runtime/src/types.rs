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
    pub synthesis: SynthesisResult,
    pub tts_latency_ms: u64,
    pub total_latency_ms: u64,
}
