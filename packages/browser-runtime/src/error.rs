//! Error types for browser-runtime.

use std::fmt;

/// Result alias for browser runtime operations.
pub type Result<T> = std::result::Result<T, BrowserRuntimeError>;

/// Primary error type for the browser-runtime package.
#[derive(Debug)]
pub enum BrowserRuntimeError {
    /// Failed to launch or locate the browser process.
    BrowserLaunchFailed { message: String },
    /// CDP WebSocket / TCP connection failed.
    ConnectionFailed { message: String },
    /// Navigation to a URL failed or timed out.
    NavigationFailed { url: String, message: String },
    /// Failed to inspect semantic DOM nodes via CDP.
    DomInspectionFailed { message: String },
    /// Specified CDP target tab ID was not found.
    TabNotFound { target_id: String },
    /// Error originating from the core runtime subsystem.
    Runtime(runtime::RuntimeError),
    /// Error originating from the services subsystem.
    Service(services::ServicesError),
    /// Error originating from configuration management.
    Configuration(configuration::ConfigError),
    /// Internal lock acquisition error.
    LockError { message: String },
}

impl fmt::Display for BrowserRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrowserLaunchFailed { message } => write!(f, "Browser launch failed: {message}"),
            Self::ConnectionFailed { message } => write!(f, "CDP connection failed: {message}"),
            Self::NavigationFailed { url, message } => {
                write!(f, "Navigation failed for '{url}': {message}")
            }
            Self::DomInspectionFailed { message } => write!(f, "DOM inspection failed: {message}"),
            Self::TabNotFound { target_id } => write!(f, "Browser tab not found: {target_id}"),
            Self::Runtime(err) => write!(f, "Runtime error: {err}"),
            Self::Service(err) => write!(f, "Services error: {err}"),
            Self::Configuration(err) => write!(f, "Configuration error: {err}"),
            Self::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for BrowserRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(err) => Some(err),
            Self::Service(err) => Some(err),
            Self::Configuration(err) => Some(err),
            _ => None,
        }
    }
}

impl From<runtime::RuntimeError> for BrowserRuntimeError {
    fn from(err: runtime::RuntimeError) -> Self {
        Self::Runtime(err)
    }
}

impl From<services::ServicesError> for BrowserRuntimeError {
    fn from(err: services::ServicesError) -> Self {
        Self::Service(err)
    }
}

impl From<configuration::ConfigError> for BrowserRuntimeError {
    fn from(err: configuration::ConfigError) -> Self {
        Self::Configuration(err)
    }
}
