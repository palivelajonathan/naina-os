//! Error types for the NAINA OS model-runtime package.

use std::fmt;

/// Result type used throughout the model-runtime package.
pub type Result<T> = std::result::Result<T, ModelRuntimeError>;

/// Errors produced by the model-runtime package.
#[derive(Debug)]
pub enum ModelRuntimeError {
    ModelNotFound {
        model_name: String,
    },
    ProviderNotFound {
        provider_name: String,
    },
    LoadFailed {
        message: String,
    },
    InferenceFailed {
        message: String,
    },
    VramExceeded {
        limit_bytes: usize,
        requested_bytes: usize,
    },
    ConfigError {
        message: String,
    },
    LockError {
        message: String,
    },
    Configuration(configuration::ConfigError),
}

impl fmt::Display for ModelRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelRuntimeError::ModelNotFound { model_name } => {
                write!(f, "Model not found: {model_name}")
            }
            ModelRuntimeError::ProviderNotFound { provider_name } => {
                write!(f, "Provider not found: {provider_name}")
            }
            ModelRuntimeError::LoadFailed { message } => {
                write!(f, "Model load failed: {message}")
            }
            ModelRuntimeError::InferenceFailed { message } => {
                write!(f, "Model inference failed: {message}")
            }
            ModelRuntimeError::VramExceeded {
                limit_bytes,
                requested_bytes,
            } => write!(
                f,
                "GPU VRAM limit exceeded: limit {limit_bytes} bytes, requested {requested_bytes} bytes"
            ),
            ModelRuntimeError::ConfigError { message } => {
                write!(f, "Model runtime configuration error: {message}")
            }
            ModelRuntimeError::LockError { message } => {
                write!(f, "Model runtime lock error: {message}")
            }
            ModelRuntimeError::Configuration(err) => write!(f, "Configuration error: {err}"),
        }
    }
}

impl std::error::Error for ModelRuntimeError {}

impl From<configuration::ConfigError> for ModelRuntimeError {
    fn from(err: configuration::ConfigError) -> Self {
        ModelRuntimeError::Configuration(err)
    }
}
