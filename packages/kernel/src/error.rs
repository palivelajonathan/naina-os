//! Error types for the NAINA OS kernel package.

use crate::types::ProcessId;
use std::fmt;

/// Result type used throughout the kernel package.
pub type Result<T> = std::result::Result<T, KernelError>;

/// Errors produced by the kernel package.
#[derive(Debug)]
pub enum KernelError {
    BootFailed { message: String },
    ProcessNotFound { id: ProcessId },
    ProcessFailed { id: ProcessId, message: String },
    Unauthorized { capability_id: String },
    InvalidState { current: String, expected: String },
    LockError { message: String },
    Capability(capabilities::CapabilityError),
    EventBus(event_bus::EventBusError),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::BootFailed { message } => write!(f, "Kernel boot failed: {message}"),
            KernelError::ProcessNotFound { id } => write!(f, "Process not found: {}", id.0),
            KernelError::ProcessFailed { id, message } => {
                write!(f, "Process {} failed: {message}", id.0)
            }
            KernelError::Unauthorized { capability_id } => {
                write!(
                    f,
                    "Unauthorized capability '{capability_id}' for kernel operation"
                )
            }
            KernelError::InvalidState { current, expected } => {
                write!(
                    f,
                    "Invalid kernel state transition: current '{current}', expected '{expected}'"
                )
            }
            KernelError::LockError { message } => write!(f, "Kernel lock error: {message}"),
            KernelError::Capability(err) => write!(f, "Capability error: {err}"),
            KernelError::EventBus(err) => write!(f, "EventBus error: {err}"),
        }
    }
}

impl std::error::Error for KernelError {}

impl From<capabilities::CapabilityError> for KernelError {
    fn from(err: capabilities::CapabilityError) -> Self {
        KernelError::Capability(err)
    }
}

impl From<event_bus::EventBusError> for KernelError {
    fn from(err: event_bus::EventBusError) -> Self {
        KernelError::EventBus(err)
    }
}
