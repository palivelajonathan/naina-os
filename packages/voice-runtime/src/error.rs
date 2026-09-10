//! Error types for the NAINA OS voice-runtime package.

use std::fmt;

/// Result type used throughout the voice-runtime package.
pub type Result<T> = std::result::Result<T, VoiceRuntimeError>;

/// Errors produced by the voice-runtime package.
#[derive(Debug)]
pub enum VoiceRuntimeError {
    AudioCaptureFailed { message: String },
    SttTranscriptionFailed { message: String },
    TtsSynthesisFailed { message: String },
    EngineNotLoaded { engine_name: String },
    InvalidAudioFormat { message: String },
    BargeInInterrupted,
    Runtime(runtime::RuntimeError),
    Service(services::ServicesError),
    Configuration(configuration::ConfigError),
    LockError { message: String },
}

impl fmt::Display for VoiceRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoiceRuntimeError::AudioCaptureFailed { message } => {
                write!(f, "Audio capture failed: {message}")
            }
            VoiceRuntimeError::SttTranscriptionFailed { message } => {
                write!(f, "STT transcription failed: {message}")
            }
            VoiceRuntimeError::TtsSynthesisFailed { message } => {
                write!(f, "TTS synthesis failed: {message}")
            }
            VoiceRuntimeError::EngineNotLoaded { engine_name } => {
                write!(f, "Voice engine '{engine_name}' not loaded")
            }
            VoiceRuntimeError::InvalidAudioFormat { message } => {
                write!(f, "Invalid audio format: {message}")
            }
            VoiceRuntimeError::BargeInInterrupted => {
                write!(f, "Voice turn interrupted by user barge-in")
            }
            VoiceRuntimeError::Runtime(err) => write!(f, "Runtime error: {err}"),
            VoiceRuntimeError::Service(err) => write!(f, "Service error: {err}"),
            VoiceRuntimeError::Configuration(err) => write!(f, "Configuration error: {err}"),
            VoiceRuntimeError::LockError { message } => {
                write!(f, "Lock acquisition error: {message}")
            }
        }
    }
}

impl std::error::Error for VoiceRuntimeError {}

impl From<runtime::RuntimeError> for VoiceRuntimeError {
    fn from(err: runtime::RuntimeError) -> Self {
        VoiceRuntimeError::Runtime(err)
    }
}

impl From<services::ServicesError> for VoiceRuntimeError {
    fn from(err: services::ServicesError) -> Self {
        VoiceRuntimeError::Service(err)
    }
}

impl From<configuration::ConfigError> for VoiceRuntimeError {
    fn from(err: configuration::ConfigError) -> Self {
        VoiceRuntimeError::Configuration(err)
    }
}
