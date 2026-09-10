//! Error types for the NAINA OS runtime package.

use crate::types::ExecutionContextId;
use std::fmt;

/// Result type used throughout the runtime package.
pub type Result<T> = std::result::Result<T, RuntimeError>;

/// Errors produced by the runtime package.
#[derive(Debug)]
pub enum RuntimeError {
    InitFailed {
        message: String,
    },
    ContextNotFound {
        id: ExecutionContextId,
    },
    ContextFailed {
        id: ExecutionContextId,
        message: String,
    },
    Unauthorized {
        capability_id: String,
    },
    InvalidState {
        current: String,
        expected: String,
    },
    LockError {
        message: String,
    },
    Kernel(kernel::KernelError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::InitFailed { message } => {
                write!(f, "Runtime initialization failed: {message}")
            }
            RuntimeError::ContextNotFound { id } => {
                write!(f, "Execution context not found: {}", id.0)
            }
            RuntimeError::ContextFailed { id, message } => {
                write!(f, "Execution context {} failed: {message}", id.0)
            }
            RuntimeError::Unauthorized { capability_id } => {
                write!(
                    f,
                    "Unauthorized capability '{capability_id}' for runtime operation"
                )
            }
            RuntimeError::InvalidState { current, expected } => {
                write!(
                    f,
                    "Invalid runtime state transition: current '{current}', expected '{expected}'"
                )
            }
            RuntimeError::LockError { message } => write!(f, "Runtime lock error: {message}"),
            RuntimeError::Kernel(err) => write!(f, "Kernel error: {err}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<kernel::KernelError> for RuntimeError {
    fn from(err: kernel::KernelError) -> Self {
        RuntimeError::Kernel(err)
    }
}
