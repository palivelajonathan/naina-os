//! Unified error model for the NAINA OS logging package.

use std::fmt;
use std::io;

/// Result type used throughout the logging package.
pub type Result<T> = std::result::Result<T, LogError>;

/// Errors produced by the logging package.
#[derive(Debug)]
pub enum LogError {
    /// Failed to initialize the logger.
    Initialization { message: String },

    /// A sink failed to emit a log record.
    Sink { message: String },

    /// A log record failed to format.
    Formatting { message: String },

    /// An underlying I/O operation failed.
    Io(io::Error),

    /// The logger was invoked in an invalid state.
    InvalidState { message: String },
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogError::Initialization { message } => {
                write!(f, "Logger initialization failed: {message}")
            }
            LogError::Sink { message } => write!(f, "Log sink failure: {message}"),
            LogError::Formatting { message } => write!(f, "Log formatting failure: {message}"),
            LogError::Io(source) => write!(f, "I/O error: {source}"),
            LogError::InvalidState { message } => write!(f, "Invalid logger state: {message}"),
        }
    }
}

impl std::error::Error for LogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LogError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for LogError {
    fn from(source: io::Error) -> Self {
        LogError::Io(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::io;

    #[test]
    fn display_variants() {
        assert_eq!(
            LogError::Initialization {
                message: "failed to init".to_string()
            }
            .to_string(),
            "Logger initialization failed: failed to init"
        );

        assert_eq!(
            LogError::Sink {
                message: "console unavailable".to_string()
            }
            .to_string(),
            "Log sink failure: console unavailable"
        );

        assert_eq!(
            LogError::Formatting {
                message: "invalid record".to_string()
            }
            .to_string(),
            "Log formatting failure: invalid record"
        );

        assert_eq!(
            LogError::InvalidState {
                message: "shut down".to_string()
            }
            .to_string(),
            "Invalid logger state: shut down"
        );
    }

    #[test]
    fn io_error_source_is_preserved() {
        let source = io::Error::other("disk full");
        let error = LogError::Io(source);

        assert_eq!(error.to_string(), "I/O error: disk full");
        assert!(error.source().is_some());
    }

    #[test]
    fn io_error_converts_from_io_error() {
        let source = io::Error::other("write failed");
        let error: LogError = source.into();

        assert_eq!(error.to_string(), "I/O error: write failed");
    }
}
