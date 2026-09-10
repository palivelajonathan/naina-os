//! Error types for the NAINA OS context-engine package.

use crate::types::ConversationId;
use std::fmt;

/// Result type used throughout the context-engine package.
pub type Result<T> = std::result::Result<T, ContextEngineError>;

/// Errors produced by the context-engine package.
#[derive(Debug)]
pub enum ContextEngineError {
    ConversationNotFound { id: ConversationId },
    TokenLimitExceeded { limit: usize, requested: usize },
    ConfigError { message: String },
    LockError { message: String },
    Memory(memory::MemoryError),
    Configuration(configuration::ConfigError),
}

impl fmt::Display for ContextEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextEngineError::ConversationNotFound { id } => {
                write!(f, "Conversation not found: {}", id.0)
            }
            ContextEngineError::TokenLimitExceeded { limit, requested } => write!(
                f,
                "Context token limit exceeded: limit {limit}, requested {requested}"
            ),
            ContextEngineError::ConfigError { message } => {
                write!(f, "Context engine configuration error: {message}")
            }
            ContextEngineError::LockError { message } => {
                write!(f, "Context engine lock error: {message}")
            }
            ContextEngineError::Memory(err) => write!(f, "Memory error: {err}"),
            ContextEngineError::Configuration(err) => write!(f, "Configuration error: {err}"),
        }
    }
}

impl std::error::Error for ContextEngineError {}

impl From<memory::MemoryError> for ContextEngineError {
    fn from(err: memory::MemoryError) -> Self {
        ContextEngineError::Memory(err)
    }
}

impl From<configuration::ConfigError> for ContextEngineError {
    fn from(err: configuration::ConfigError) -> Self {
        ContextEngineError::Configuration(err)
    }
}
