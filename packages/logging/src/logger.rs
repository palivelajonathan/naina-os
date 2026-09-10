//! Logger implementation for NAINA OS.

use crate::config::LoggerConfig;
use crate::error::Result;
use crate::level::LogLevel;
use crate::record::{LogFields, LogRecord};
use crate::sink::LogSink;

/// Logger handles log event filtering, target assignment, and dispatching to sinks.
pub struct Logger {
    config: LoggerConfig,
    component: String,
    sinks: Vec<Box<dyn LogSink>>,
}

impl Logger {
    /// Creates a new logger with the given configuration and default component `"naina"`.
    pub fn new(config: LoggerConfig) -> Result<Self> {
        Ok(Self {
            config,
            component: "naina".to_string(),
            sinks: Vec::new(),
        })
    }

    /// Replaces the component associated with this logger.
    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.component = component.into();
        self
    }

    /// Registers a log sink with this logger.
    pub fn with_sink(mut self, sink: Box<dyn LogSink>) -> Self {
        self.sinks.push(sink);
        self
    }

    /// Checks whether a log level is enabled for this logger.
    pub fn enabled(&self, level: LogLevel) -> bool {
        level_weight(level) >= level_weight(self.config.level())
    }

    /// Logs a message at the given log level.
    pub fn log(&self, level: LogLevel, message: impl Into<String>) -> Result<()> {
        if !self.enabled(level) {
            return Ok(());
        }

        let record = LogRecord::new(&self.component, level, message);
        for sink in &self.sinks {
            sink.write(&record)?;
        }
        Ok(())
    }

    /// Logs a message with structured fields at the given log level.
    pub fn log_with_fields(
        &self,
        level: LogLevel,
        message: impl Into<String>,
        fields: LogFields,
    ) -> Result<()> {
        if !self.enabled(level) {
            return Ok(());
        }

        let record = LogRecord::new(&self.component, level, message).with_fields(fields);
        for sink in &self.sinks {
            sink.write(&record)?;
        }
        Ok(())
    }

    /// Logs a trace-level message.
    pub fn trace(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Trace, message)
    }

    /// Logs a debug-level message.
    pub fn debug(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Debug, message)
    }

    /// Logs an info-level message.
    pub fn info(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Info, message)
    }

    /// Logs a warn-level message.
    pub fn warn(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Warn, message)
    }

    /// Logs an error-level message.
    pub fn error(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Error, message)
    }
}

fn level_weight(level: LogLevel) -> u8 {
    match level {
        LogLevel::Trace => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LogError;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Default)]
    struct RecordingSink {
        records: Rc<RefCell<Vec<LogRecord>>>,
        should_fail: bool,
    }

    impl RecordingSink {
        fn failing() -> Self {
            Self {
                records: Rc::new(RefCell::new(Vec::new())),
                should_fail: true,
            }
        }
    }

    impl LogSink for RecordingSink {
        fn write(&self, record: &LogRecord) -> Result<()> {
            if self.should_fail {
                return Err(LogError::Sink {
                    message: "sink failure".to_string(),
                });
            }
            self.records.borrow_mut().push(record.clone());
            Ok(())
        }
    }

    #[test]
    fn test_1_default_configuration() {
        let config = LoggerConfig::default();
        let logger = Logger::new(config).unwrap();
        assert_eq!(logger.config.level(), LogLevel::Info);
    }

    #[test]
    fn test_2_explicit_configuration() {
        let config = LoggerConfig::new(LogLevel::Debug);
        let logger = Logger::new(config).unwrap();
        assert_eq!(logger.config.level(), LogLevel::Debug);
    }

    #[test]
    fn test_3_default_component() {
        let sink = RecordingSink::default();
        let logger = Logger::new(LoggerConfig::default())
            .unwrap()
            .with_sink(Box::new(sink.clone()));

        logger.info("msg").unwrap();
        assert_eq!(sink.records.borrow()[0].target(), "naina");
    }

    #[test]
    fn test_4_custom_component() {
        let sink = RecordingSink::default();
        let logger = Logger::new(LoggerConfig::default())
            .unwrap()
            .with_component("kernel")
            .with_sink(Box::new(sink.clone()));

        logger.info("msg").unwrap();
        assert_eq!(sink.records.borrow()[0].target(), "kernel");
    }

    #[test]
    fn test_5_trace_filtering() {
        let logger = Logger::new(LoggerConfig::new(LogLevel::Trace)).unwrap();
        assert!(logger.enabled(LogLevel::Trace));
        assert!(logger.enabled(LogLevel::Debug));
        assert!(logger.enabled(LogLevel::Info));
        assert!(logger.enabled(LogLevel::Warn));
        assert!(logger.enabled(LogLevel::Error));
    }

    #[test]
    fn test_6_debug_filtering() {
        let logger = Logger::new(LoggerConfig::new(LogLevel::Debug)).unwrap();
        assert!(!logger.enabled(LogLevel::Trace));
        assert!(logger.enabled(LogLevel::Debug));
        assert!(logger.enabled(LogLevel::Info));
        assert!(logger.enabled(LogLevel::Warn));
        assert!(logger.enabled(LogLevel::Error));
    }

    #[test]
    fn test_7_info_filtering() {
        let logger = Logger::new(LoggerConfig::new(LogLevel::Info)).unwrap();
        assert!(!logger.enabled(LogLevel::Trace));
        assert!(!logger.enabled(LogLevel::Debug));
        assert!(logger.enabled(LogLevel::Info));
        assert!(logger.enabled(LogLevel::Warn));
        assert!(logger.enabled(LogLevel::Error));
    }

    #[test]
    fn test_8_warn_filtering() {
        let logger = Logger::new(LoggerConfig::new(LogLevel::Warn)).unwrap();
        assert!(!logger.enabled(LogLevel::Trace));
        assert!(!logger.enabled(LogLevel::Debug));
        assert!(!logger.enabled(LogLevel::Info));
        assert!(logger.enabled(LogLevel::Warn));
        assert!(logger.enabled(LogLevel::Error));
    }

    #[test]
    fn test_9_error_filtering() {
        let logger = Logger::new(LoggerConfig::new(LogLevel::Error)).unwrap();
        assert!(!logger.enabled(LogLevel::Trace));
        assert!(!logger.enabled(LogLevel::Debug));
        assert!(!logger.enabled(LogLevel::Info));
        assert!(!logger.enabled(LogLevel::Warn));
        assert!(logger.enabled(LogLevel::Error));
    }

    #[test]
    fn test_10_enabled() {
        let logger = Logger::new(LoggerConfig::new(LogLevel::Warn)).unwrap();
        assert!(!logger.enabled(LogLevel::Info));
        assert!(logger.enabled(LogLevel::Warn));
    }

    #[test]
    fn test_11_enabled_records_reach_sinks() {
        let sink = RecordingSink::default();
        let logger = Logger::new(LoggerConfig::new(LogLevel::Info))
            .unwrap()
            .with_sink(Box::new(sink.clone()));

        logger.info("enabled event").unwrap();

        let records = sink.records.borrow();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message(), "enabled event");
        assert_eq!(records[0].level(), LogLevel::Info);
    }

    #[test]
    fn test_12_disabled_records_do_not_reach_sinks() {
        let sink = RecordingSink::default();
        let logger = Logger::new(LoggerConfig::new(LogLevel::Warn))
            .unwrap()
            .with_sink(Box::new(sink.clone()));

        let res = logger.info("disabled event");
        assert!(res.is_ok());
        assert!(sink.records.borrow().is_empty());
    }

    #[test]
    fn test_13_structured_fields() {
        let sink = RecordingSink::default();
        let logger = Logger::new(LoggerConfig::new(LogLevel::Info))
            .unwrap()
            .with_sink(Box::new(sink.clone()));

        let mut fields = LogFields::new();
        fields.insert("key".to_string(), "val".to_string());

        logger
            .log_with_fields(LogLevel::Info, "event", fields.clone())
            .unwrap();

        let records = sink.records.borrow();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields(), Some(&fields));
    }

    #[test]
    fn test_14_multiple_sinks() {
        let sink1 = RecordingSink::default();
        let sink2 = RecordingSink::default();
        let logger = Logger::new(LoggerConfig::new(LogLevel::Info))
            .unwrap()
            .with_sink(Box::new(sink1.clone()))
            .with_sink(Box::new(sink2.clone()));

        logger.info("multi-sink").unwrap();

        assert_eq!(sink1.records.borrow().len(), 1);
        assert_eq!(sink2.records.borrow().len(), 1);
        assert_eq!(sink1.records.borrow()[0].message(), "multi-sink");
        assert_eq!(sink2.records.borrow()[0].message(), "multi-sink");
    }

    #[test]
    fn test_15_sink_error_propagation() {
        let sink = RecordingSink::failing();
        let logger = Logger::new(LoggerConfig::new(LogLevel::Info))
            .unwrap()
            .with_sink(Box::new(sink));

        let res = logger.info("fail");
        assert!(matches!(res, Err(LogError::Sink { .. })));
    }

    #[test]
    fn test_16_zero_sink_behavior() {
        let logger = Logger::new(LoggerConfig::new(LogLevel::Info)).unwrap();
        let res = logger.info("no sinks");
        assert!(res.is_ok());
    }

    #[test]
    fn test_17_convenience_methods() {
        let sink = RecordingSink::default();
        let logger = Logger::new(LoggerConfig::new(LogLevel::Trace))
            .unwrap()
            .with_sink(Box::new(sink.clone()));

        logger.trace("trace msg").unwrap();
        logger.debug("debug msg").unwrap();
        logger.info("info msg").unwrap();
        logger.warn("warn msg").unwrap();
        logger.error("error msg").unwrap();

        let records = sink.records.borrow();
        assert_eq!(records.len(), 5);
        assert_eq!(records[0].level(), LogLevel::Trace);
        assert_eq!(records[1].level(), LogLevel::Debug);
        assert_eq!(records[2].level(), LogLevel::Info);
        assert_eq!(records[3].level(), LogLevel::Warn);
        assert_eq!(records[4].level(), LogLevel::Error);
    }
}
