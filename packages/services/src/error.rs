//! Error types for the NAINA OS services package.

use std::fmt;

/// Result type used throughout the services package.
pub type Result<T> = std::result::Result<T, ServicesError>;

/// Errors produced by the services package.
#[derive(Debug)]
pub enum ServicesError {
    ServiceNotFound { name_or_id: String },
    ServiceAlreadyExists { name: String },
    Unauthorized { capability_id: String },
    InvalidState { current: String, expected: String },
    LockError { message: String },
    Runtime(runtime::RuntimeError),
    Capability(capabilities::CapabilityError),
}

impl fmt::Display for ServicesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServicesError::ServiceNotFound { name_or_id } => {
                write!(f, "Service not found: {name_or_id}")
            }
            ServicesError::ServiceAlreadyExists { name } => {
                write!(f, "Service already exists: {name}")
            }
            ServicesError::Unauthorized { capability_id } => {
                write!(
                    f,
                    "Unauthorized capability '{capability_id}' for service operation"
                )
            }
            ServicesError::InvalidState { current, expected } => {
                write!(
                    f,
                    "Invalid service state transition: current '{current}', expected '{expected}'"
                )
            }
            ServicesError::LockError { message } => write!(f, "Services lock error: {message}"),
            ServicesError::Runtime(err) => write!(f, "Runtime error: {err}"),
            ServicesError::Capability(err) => write!(f, "Capability error: {err}"),
        }
    }
}

impl std::error::Error for ServicesError {}

impl From<runtime::RuntimeError> for ServicesError {
    fn from(err: runtime::RuntimeError) -> Self {
        ServicesError::Runtime(err)
    }
}

impl From<capabilities::CapabilityError> for ServicesError {
    fn from(err: capabilities::CapabilityError) -> Self {
        ServicesError::Capability(err)
    }
}
