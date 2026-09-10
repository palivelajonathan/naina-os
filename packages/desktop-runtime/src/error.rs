//! Error types for desktop-runtime.

use std::fmt;

/// Result alias for desktop runtime operations.
pub type Result<T> = std::result::Result<T, DesktopRuntimeError>;

/// Primary error type for the desktop-runtime package.
#[derive(Debug)]
pub enum DesktopRuntimeError {
    /// Failed to launch a native application process.
    AppLaunchFailed { message: String },
    /// Specified window handle was not found or is invalid.
    WindowNotFound { handle: u64 },
    /// Specified UI element was not found during tree traversal.
    UiElementNotFound { element_id: String },
    /// Synthetic input injection failed.
    InputInjectionFailed { message: String },
    /// COM Single-Threaded Apartment (STA) initialization failed.
    ComInitializationFailed { message: String },
    /// Error originating from the core runtime subsystem.
    Runtime(runtime::RuntimeError),
    /// Error originating from the services subsystem.
    Service(services::ServicesError),
    /// Error originating from configuration management.
    Configuration(configuration::ConfigError),
    /// Internal lock acquisition error.
    LockError { message: String },
}

impl fmt::Display for DesktopRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppLaunchFailed { message } => write!(f, "App launch failed: {message}"),
            Self::WindowNotFound { handle } => write!(f, "Window handle not found: {handle}"),
            Self::UiElementNotFound { element_id } => {
                write!(f, "UI element not found: {element_id}")
            }
            Self::InputInjectionFailed { message } => {
                write!(f, "Input injection failed: {message}")
            }
            Self::ComInitializationFailed { message } => {
                write!(f, "COM STA initialization failed: {message}")
            }
            Self::Runtime(err) => write!(f, "Runtime error: {err}"),
            Self::Service(err) => write!(f, "Services error: {err}"),
            Self::Configuration(err) => write!(f, "Configuration error: {err}"),
            Self::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for DesktopRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(err) => Some(err),
            Self::Service(err) => Some(err),
            Self::Configuration(err) => Some(err),
            _ => None,
        }
    }
}

impl From<runtime::RuntimeError> for DesktopRuntimeError {
    fn from(err: runtime::RuntimeError) -> Self {
        Self::Runtime(err)
    }
}

impl From<services::ServicesError> for DesktopRuntimeError {
    fn from(err: services::ServicesError) -> Self {
        Self::Service(err)
    }
}

impl From<configuration::ConfigError> for DesktopRuntimeError {
    fn from(err: configuration::ConfigError) -> Self {
        Self::Configuration(err)
    }
}
