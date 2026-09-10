//! Error types and Result alias for the Desktop Host application.

use std::fmt;

/// Result alias returned by Desktop Host application operations.
pub type DesktopHostResult<T> = std::result::Result<T, DesktopHostError>;

/// Strongly typed errors returned by the Desktop Host composition root.
#[derive(Debug)]
pub enum DesktopHostError {
    /// Boot initialization sequence failed.
    BootFailed { message: String },
    /// Logging framework operation failed.
    LoggingError(logging::error::LogError),
    /// Voice runtime operation failed.
    VoiceError(voice_runtime::VoiceRuntimeError),
    /// Orchestrator task planning or execution failed.
    OrchestratorError(orchestrator::OrchestratorError),
    /// Model runtime or provider inference failed.
    ModelError(model_runtime::ModelRuntimeError),
    /// Memory store retrieval or indexing failed.
    MemoryError(memory::MemoryError),
    /// Service registry operation failed.
    ServicesError(services::ServicesError),
    /// Microkernel runtime operation failed.
    RuntimeError(runtime::RuntimeError),
    /// SDK facade operation failed.
    SDKError(sdk::SDKError),
    /// UI framework operation failed.
    UIError(ui::UIError),
    /// Shutdown sequence encountered an error.
    ShutdownError { message: String },
    /// Internal synchronization lock acquisition failure.
    LockError { message: String },
}

impl fmt::Display for DesktopHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BootFailed { message } => write!(f, "Desktop host boot failed: {message}"),
            Self::LoggingError(err) => write!(f, "Logging error: {err}"),
            Self::VoiceError(err) => write!(f, "Voice runtime error: {err}"),
            Self::OrchestratorError(err) => write!(f, "Orchestrator error: {err}"),
            Self::ModelError(err) => write!(f, "Model runtime error: {err}"),
            Self::MemoryError(err) => write!(f, "Memory error: {err}"),
            Self::ServicesError(err) => write!(f, "Services error: {err}"),
            Self::RuntimeError(err) => write!(f, "Runtime error: {err}"),
            Self::SDKError(err) => write!(f, "SDK error: {err}"),
            Self::UIError(err) => write!(f, "UI error: {err}"),
            Self::ShutdownError { message } => write!(f, "Shutdown error: {message}"),
            Self::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for DesktopHostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LoggingError(err) => Some(err),
            Self::VoiceError(err) => Some(err),
            Self::OrchestratorError(err) => Some(err),
            Self::ModelError(err) => Some(err),
            Self::MemoryError(err) => Some(err),
            Self::ServicesError(err) => Some(err),
            Self::RuntimeError(err) => Some(err),
            Self::SDKError(err) => Some(err),
            Self::UIError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<logging::error::LogError> for DesktopHostError {
    fn from(err: logging::error::LogError) -> Self {
        Self::LoggingError(err)
    }
}

impl From<voice_runtime::VoiceRuntimeError> for DesktopHostError {
    fn from(err: voice_runtime::VoiceRuntimeError) -> Self {
        Self::VoiceError(err)
    }
}

impl From<orchestrator::OrchestratorError> for DesktopHostError {
    fn from(err: orchestrator::OrchestratorError) -> Self {
        Self::OrchestratorError(err)
    }
}

impl From<model_runtime::ModelRuntimeError> for DesktopHostError {
    fn from(err: model_runtime::ModelRuntimeError) -> Self {
        Self::ModelError(err)
    }
}

impl From<memory::MemoryError> for DesktopHostError {
    fn from(err: memory::MemoryError) -> Self {
        Self::MemoryError(err)
    }
}

impl From<services::ServicesError> for DesktopHostError {
    fn from(err: services::ServicesError) -> Self {
        Self::ServicesError(err)
    }
}

impl From<runtime::RuntimeError> for DesktopHostError {
    fn from(err: runtime::RuntimeError) -> Self {
        Self::RuntimeError(err)
    }
}

impl From<sdk::SDKError> for DesktopHostError {
    fn from(err: sdk::SDKError) -> Self {
        Self::SDKError(err)
    }
}

impl From<ui::UIError> for DesktopHostError {
    fn from(err: ui::UIError) -> Self {
        Self::UIError(err)
    }
}
