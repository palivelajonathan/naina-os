//! Unified error model for the NAINA OS event-bus package.

use crate::types::SubscriptionId;
use std::fmt;

/// Result type used throughout the event-bus package.
pub type Result<T> = std::result::Result<T, EventBusError>;

/// Errors produced by the event-bus package.
#[derive(Debug)]
pub enum EventBusError {
    /// A subscriber handler failed during event dispatch.
    Handler {
        subscription_id: SubscriptionId,
        message: String,
    },
    /// Internal lock acquisition failed.
    LockError { message: String },
}

impl fmt::Display for EventBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventBusError::Handler {
                subscription_id,
                message,
            } => {
                write!(
                    f,
                    "Handler error (subscription {subscription_id:?}): {message}"
                )
            }
            EventBusError::LockError { message } => write!(f, "Lock error: {message}"),
        }
    }
}

impl std::error::Error for EventBusError {}
