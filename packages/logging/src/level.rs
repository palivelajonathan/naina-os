//! Log level definitions for the NAINA OS logging package.

use std::fmt;
use std::str::FromStr;

/// Supported log levels in NAINA OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Trace-level output, useful for the most detailed debugging information.
    Trace,
    /// Debug-level output, suitable for diagnostic information during development.
    Debug,
    /// Informational output, intended for general runtime events.
    Info,
    /// Warning output, indicating a potential issue or unexpected state.
    Warn,
    /// Error output, indicating an error that may require attention.
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };

        write!(f, "{level}")
    }
}

impl FromStr for LogLevel {
    type Err = LogLevelParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" | "err" => Ok(LogLevel::Error),
            invalid => Err(LogLevelParseError {
                invalid: invalid.to_string(),
            }),
        }
    }
}

/// Error returned when parsing a string into a [`LogLevel`] fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLevelParseError {
    invalid: String,
}

impl fmt::Display for LogLevelParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid log level: '{}'", self.invalid)
    }
}

impl std::error::Error for LogLevelParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_lowercase_text() {
        assert_eq!(LogLevel::Trace.to_string(), "trace");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Error.to_string(), "error");
    }

    #[test]
    fn parse_accepts_known_level_names() {
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("Info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("ERROR".parse::<LogLevel>().unwrap(), LogLevel::Error);
    }

    #[test]
    fn parse_accepts_warning_alias() {
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("err".parse::<LogLevel>().unwrap(), LogLevel::Error);
    }

    #[test]
    fn parse_rejects_unknown_levels() {
        let error = "verbose".parse::<LogLevel>().unwrap_err();

        assert_eq!(error.to_string(), "invalid log level: 'verbose'");
    }
}
