//! Trait abstractions for Speech-to-Text (STT) and Text-to-Speech (TTS) engines.

use crate::error::Result;
use crate::types::{AudioBuffer, SynthesisResult, TranscriptionResult};
use std::fmt::Debug;

/// Interface for Speech-to-Text (STT) transcription backends (e.g. Whisper).
pub trait SttEngine: Send + Sync + Debug {
    /// Returns the engine backend name.
    fn engine_name(&self) -> &str;

    /// Transcribes an audio buffer into text.
    fn transcribe(&self, audio: &AudioBuffer) -> Result<TranscriptionResult>;
}

/// Interface for Text-to-Speech (TTS) synthesis backends (e.g. Piper).
pub trait TtsEngine: Send + Sync + Debug {
    /// Returns the engine backend name.
    fn engine_name(&self) -> &str;

    /// Synthesizes text into spoken audio buffer.
    fn synthesize(&self, text: &str) -> Result<SynthesisResult>;
}
