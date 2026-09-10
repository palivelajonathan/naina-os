//! Unified error model for the NAINA OS capabilities package.

use std::fmt;

/// Result type used throughout the capabilities package.
pub type Result<T> = std::result::Result<T, CapabilityError>;

/// Errors produced by the capabilities package.
#[derive(Debug)]
pub enum CapabilityError {
    /// The presented token does not grant the required capability.
    Unauthorized {
        capability_id: String,
        message: String,
    },
    /// The capability token has expired.
    TokenExpired { token_id: String },
    /// The capability token has been revoked.
    TokenRevoked { token_id: String },
    /// The requested capability token was not found in the registry.
    TokenNotFound { token_id: String },
    /// Internal lock acquisition failed.
    LockError { message: String },
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityError::Unauthorized {
                capability_id,
                message,
            } => {
                write!(f, "Unauthorized capability '{capability_id}': {message}")
            }
            CapabilityError::TokenExpired { token_id } => {
                write!(f, "Capability token '{token_id}' has expired")
            }
            CapabilityError::TokenRevoked { token_id } => {
                write!(f, "Capability token '{token_id}' has been revoked")
            }
            CapabilityError::TokenNotFound { token_id } => {
                write!(f, "Capability token '{token_id}' not found")
            }
            CapabilityError::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for CapabilityError {}
