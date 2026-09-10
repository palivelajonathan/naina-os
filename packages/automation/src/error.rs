//! Error types and Result alias for the automation engine.

use std::fmt;

/// Result type returned by automation engine operations.
pub type Result<T> = std::result::Result<T, AutomationError>;

/// Strongly typed errors produced during workflow automation execution.
#[derive(Debug)]
pub enum AutomationError {
    /// Workflow specification schema validation failed.
    WorkflowValidationFailed { message: String },
    /// Individual step execution encountered an error.
    StepExecutionFailed { step_id: String, message: String },
    /// Workflow execution exceeded the total time budget.
    WorkflowTimeout { workflow_id: String },
    /// Workflow execution was cancelled via cancellation token.
    WorkflowCancelled { workflow_id: String },
    /// Capability authorization token check failed for a step action.
    CapabilityDenied { step_id: String, message: String },
    /// Error propagated from the underlying runtime crate.
    Runtime(runtime::RuntimeError),
    /// Error propagated from the service registry crate.
    Service(services::ServicesError),
    /// Error propagated from the configuration crate.
    Configuration(configuration::ConfigError),
    /// Synchronization lock acquisition failure.
    LockError { message: String },
}

impl fmt::Display for AutomationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkflowValidationFailed { message } => {
                write!(f, "Workflow validation failed: {message}")
            }
            Self::StepExecutionFailed { step_id, message } => {
                write!(f, "Step '{step_id}' failed: {message}")
            }
            Self::WorkflowTimeout { workflow_id } => {
                write!(f, "Workflow '{workflow_id}' timed out")
            }
            Self::WorkflowCancelled { workflow_id } => {
                write!(f, "Workflow '{workflow_id}' cancelled")
            }
            Self::CapabilityDenied { step_id, message } => {
                write!(f, "Capability denied for step '{step_id}': {message}")
            }
            Self::Runtime(err) => write!(f, "Runtime error: {err}"),
            Self::Service(err) => write!(f, "Service error: {err}"),
            Self::Configuration(err) => write!(f, "Configuration error: {err}"),
            Self::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for AutomationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(err) => Some(err),
            Self::Service(err) => Some(err),
            Self::Configuration(err) => Some(err),
            _ => None,
        }
    }
}

impl From<runtime::RuntimeError> for AutomationError {
    fn from(err: runtime::RuntimeError) -> Self {
        Self::Runtime(err)
    }
}

impl From<services::ServicesError> for AutomationError {
    fn from(err: services::ServicesError) -> Self {
        Self::Service(err)
    }
}

impl From<configuration::ConfigError> for AutomationError {
    fn from(err: configuration::ConfigError) -> Self {
        Self::Configuration(err)
    }
}
