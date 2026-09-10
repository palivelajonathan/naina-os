//! Error types for the NAINA OS tool-registry package.

use crate::types::ToolId;
use std::fmt;

/// Result type used throughout the tool-registry package.
pub type Result<T> = std::result::Result<T, ToolError>;

/// Errors produced by the tool-registry package.
#[derive(Debug)]
pub enum ToolError {
    ToolNotFound { name_or_id: String },
    ToolAlreadyExists { name: String },
    Unauthorized { capability_id: String },
    InvalidState { current: String, expected: String },
    ExecutionFailed { id: ToolId, message: String },
    LockError { message: String },
    Runtime(runtime::RuntimeError),
    Capability(capabilities::CapabilityError),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::ToolNotFound { name_or_id } => write!(f, "Tool not found: {name_or_id}"),
            ToolError::ToolAlreadyExists { name } => write!(f, "Tool already exists: {name}"),
            ToolError::Unauthorized { capability_id } => {
                write!(
                    f,
                    "Unauthorized capability '{capability_id}' for tool operation"
                )
            }
            ToolError::InvalidState { current, expected } => write!(
                f,
                "Invalid tool state transition: current '{current}', expected '{expected}'"
            ),
            ToolError::ExecutionFailed { id, message } => {
                write!(f, "Tool execution failed for tool {}: {message}", id.0)
            }
            ToolError::LockError { message } => write!(f, "Tool registry lock error: {message}"),
            ToolError::Runtime(err) => write!(f, "Runtime error: {err}"),
            ToolError::Capability(err) => write!(f, "Capability error: {err}"),
        }
    }
}

impl std::error::Error for ToolError {}

impl From<runtime::RuntimeError> for ToolError {
    fn from(err: runtime::RuntimeError) -> Self {
        ToolError::Runtime(err)
    }
}

impl From<capabilities::CapabilityError> for ToolError {
    fn from(err: capabilities::CapabilityError) -> Self {
        ToolError::Capability(err)
    }
}
