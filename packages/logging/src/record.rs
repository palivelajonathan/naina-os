//! Structured logging record types for the NAINA OS logging package.

use crate::level::LogLevel;
use std::collections::BTreeMap;
use std::fmt;
use std::time::SystemTime;

/// Structured field values for a log record.
///
/// This type is intentionally minimal for Sprint 0: it maps field names to
/// string values without introducing a broader serialization framework.
pub type LogFields = BTreeMap<String, String>;

/// A single structured log event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRecord {
    timestamp: SystemTime,
    level: LogLevel,
    target: String,
    message: String,
    fields: Option<LogFields>,
}

impl LogRecord {
    /// Creates a new log record with the current system timestamp.
    pub fn new(target: impl Into<String>, level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: SystemTime::now(),
            level,
            target: target.into(),
            message: message.into(),
            fields: None,
        }
    }

    /// Returns the timestamp when the record was created.
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    /// Returns the log level for this record.
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Returns the target/component associated with this record.
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns the message carried by this record.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the structured fields for this record, if present.
    pub fn fields(&self) -> Option<&LogFields> {
        self.fields.as_ref()
    }

    /// Adds structured fields to this record.
    pub fn with_fields(mut self, fields: LogFields) -> Self {
        self.fields = Some(fields);
        self
    }
}

impl fmt::Display for LogRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{level}] {target}: {message}",
            level = self.level,
            target = self.target,
            message = self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn record_construction_preserves_fields() {
        let record = LogRecord::new("configuration", LogLevel::Info, "startup complete");

        assert_eq!(record.level(), LogLevel::Info);
        assert_eq!(record.target(), "configuration");
        assert_eq!(record.message(), "startup complete");
        assert!(record.fields().is_none());
    }

    #[test]
    fn record_with_fields_stores_structured_fields() {
        let mut fields = LogFields::new();
        fields.insert("user".to_string(), "alice".to_string());
        fields.insert("request_id".to_string(), "abc123".to_string());

        let record = LogRecord::new("runtime", LogLevel::Debug, "request received")
            .with_fields(fields.clone());

        assert_eq!(record.fields(), Some(&fields));
    }

    #[test]
    fn timestamp_is_recorded_at_creation() {
        let before = SystemTime::now();
        let record = LogRecord::new("logging", LogLevel::Warn, "disk space low");
        let after = SystemTime::now();

        assert!(record.timestamp() >= before);
        assert!(record.timestamp() <= after);
    }

    #[test]
    fn display_includes_level_target_and_message() {
        let record = LogRecord::new("kernel", LogLevel::Error, "panic occurred");

        assert_eq!(record.to_string(), "[error] kernel: panic occurred");
    }
}
