use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Standard result type used throughout the configuration package.
pub type Result<T> = std::result::Result<T, ConfigError>;

/// Errors that can occur while loading or validating configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// A required configuration value was not found.
    #[error("Missing required configuration field: {field}")]
    MissingField { field: String },

    /// A configuration value was present but invalid.
    #[error("Invalid value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },

    /// Failed to parse configuration data.
    #[error("Configuration parse error: {message}")]
    ParseError { message: String },

    /// Environment variable error.
    #[error("Environment variable '{variable}': {reason}")]
    EnvironmentError { variable: String, reason: String },

    /// Validation failed after configuration was loaded.
    #[error("Configuration validation failed: {message}")]
    ValidationError { message: String },

    /// File or I/O error.
    #[error("File not found: {path:?}")]
    FileNotFound { path: PathBuf },

    /// File system error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_formats_and_converts_from_io() {
        let error = ConfigError::InvalidValue {
            field: "worker_threads".to_string(),
            reason: "must be positive".to_string(),
        };

        assert!(error.to_string().contains("worker_threads"));

        let io_error = io::Error::other("boom");
        let wrapped: ConfigError = io_error.into();
        assert!(matches!(wrapped, ConfigError::Io(_)));
    }
}
