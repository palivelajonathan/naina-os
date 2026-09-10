//! Error types and Result alias for the client SDK.

use std::fmt;

/// Result alias returned by client SDK operations.
pub type SDKResult<T> = std::result::Result<T, SDKError>;

/// Strongly typed errors returned across the client SDK boundary.
#[derive(Debug)]
pub enum SDKError {
    /// SDK microkernel initialization failed.
    InitializationFailed { message: String },
    /// Requested session ID was not found.
    SessionNotFound { session_id: String },
    /// Requested session ID has expired.
    SessionExpired { session_id: String },
    /// Capability token validation failed for a client operation.
    CapabilityDenied { message: String },
    /// Error propagated from the underlying runtime crate.
    Runtime(runtime::RuntimeError),
    /// Error propagated from the service registry crate.
    Service(services::ServicesError),
    /// Error propagated from the configuration crate.
    Configuration(configuration::ConfigError),
    /// Error propagated from the logging crate.
    Logging(logging::error::LogError),
    /// Internal synchronization lock acquisition failure.
    LockError { message: String },
}

impl fmt::Display for SDKError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitializationFailed { message } => {
                write!(f, "SDK initialization failed: {message}")
            }
            Self::SessionNotFound { session_id } => {
                write!(f, "Session '{session_id}' not found")
            }
            Self::SessionExpired { session_id } => {
                write!(f, "Session '{session_id}' has expired")
            }
            Self::CapabilityDenied { message } => {
                write!(f, "Capability denied: {message}")
            }
            Self::Runtime(err) => write!(f, "Runtime error: {err}"),
            Self::Service(err) => write!(f, "Service error: {err}"),
            Self::Configuration(err) => write!(f, "Configuration error: {err}"),
            Self::Logging(err) => write!(f, "Logging error: {err}"),
            Self::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for SDKError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(err) => Some(err),
            Self::Service(err) => Some(err),
            Self::Configuration(err) => Some(err),
            Self::Logging(err) => Some(err),
            _ => None,
        }
    }
}

impl From<runtime::RuntimeError> for SDKError {
    fn from(err: runtime::RuntimeError) -> Self {
        Self::Runtime(err)
    }
}

impl From<services::ServicesError> for SDKError {
    fn from(err: services::ServicesError) -> Self {
        Self::Service(err)
    }
}

impl From<configuration::ConfigError> for SDKError {
    fn from(err: configuration::ConfigError) -> Self {
        Self::Configuration(err)
    }
}

impl From<logging::error::LogError> for SDKError {
    fn from(err: logging::error::LogError) -> Self {
        Self::Logging(err)
    }
}
