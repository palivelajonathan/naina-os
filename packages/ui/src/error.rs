//! Error types and Result alias for the UI framework.

use std::fmt;

/// Result alias returned by UI framework operations.
pub type UIResult<T> = std::result::Result<T, UIError>;

/// Strongly typed errors returned across the UI framework boundary.
#[derive(Debug)]
pub enum UIError {
    /// UI framework initialization failed.
    InitializationFailed { message: String },
    /// Abstract rendering or overlay state operation failed.
    RenderError { message: String },
    /// Requested window ID was not found.
    WindowNotFound { window_id: String },
    /// Error propagated from the underlying runtime crate.
    Runtime(runtime::RuntimeError),
    /// Error propagated from the configuration crate.
    Configuration(configuration::ConfigError),
    /// Error propagated from the logging crate.
    Logging(logging::error::LogError),
    /// Internal synchronization lock acquisition failure.
    LockError { message: String },
}

impl fmt::Display for UIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitializationFailed { message } => {
                write!(f, "UI initialization failed: {message}")
            }
            Self::RenderError { message } => write!(f, "UI render error: {message}"),
            Self::WindowNotFound { window_id } => write!(f, "Window '{window_id}' not found"),
            Self::Runtime(err) => write!(f, "Runtime error: {err}"),
            Self::Configuration(err) => write!(f, "Configuration error: {err}"),
            Self::Logging(err) => write!(f, "Logging error: {err}"),
            Self::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for UIError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(err) => Some(err),
            Self::Configuration(err) => Some(err),
            Self::Logging(err) => Some(err),
            _ => None,
        }
    }
}

impl From<runtime::RuntimeError> for UIError {
    fn from(err: runtime::RuntimeError) -> Self {
        Self::Runtime(err)
    }
}

impl From<configuration::ConfigError> for UIError {
    fn from(err: configuration::ConfigError) -> Self {
        Self::Configuration(err)
    }
}

impl From<logging::error::LogError> for UIError {
    fn from(err: logging::error::LogError) -> Self {
        Self::Logging(err)
    }
}
