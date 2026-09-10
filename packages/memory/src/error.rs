//! Error types for the NAINA OS memory package.

use crate::types::MemoryId;
use std::fmt;

/// Result type used throughout the memory package.
pub type Result<T> = std::result::Result<T, MemoryError>;

/// Errors produced by the memory package.
#[derive(Debug)]
pub enum MemoryError {
    EntryNotFound { id: MemoryId },
    VaultNotFound { path: String },
    IndexCorrupted { message: String },
    ConfigError { message: String },
    IoError { message: String },
    LockError { message: String },
    Configuration(configuration::ConfigError),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::EntryNotFound { id } => write!(f, "Memory entry not found: {}", id.0),
            MemoryError::VaultNotFound { path } => write!(f, "Memory vault path not found: {path}"),
            MemoryError::IndexCorrupted { message } => {
                write!(f, "Memory index corrupted: {message}")
            }
            MemoryError::ConfigError { message } => {
                write!(f, "Memory store configuration error: {message}")
            }
            MemoryError::IoError { message } => write!(f, "Memory store I/O error: {message}"),
            MemoryError::LockError { message } => write!(f, "Memory store lock error: {message}"),
            MemoryError::Configuration(err) => write!(f, "Configuration error: {err}"),
        }
    }
}

impl std::error::Error for MemoryError {}

impl From<configuration::ConfigError> for MemoryError {
    fn from(err: configuration::ConfigError) -> Self {
        MemoryError::Configuration(err)
    }
}

impl From<std::io::Error> for MemoryError {
    fn from(err: std::io::Error) -> Self {
        MemoryError::IoError {
            message: err.to_string(),
        }
    }
}
