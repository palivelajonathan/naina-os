//! Log sink abstraction for the NAINA OS logging package.

use crate::error::Result;
use crate::record::LogRecord;

/// A sink that accepts log records and delivers them to a concrete output.
///
/// The trait is intentionally minimal. It receives a borrowed record, reports
/// failures through the shared logging error type, and does not own runtime
/// state or perform filtering.
pub trait LogSink {
    /// Writes a log record to the sink.
    ///
    /// Implementations should accept the record by reference so callers can
    /// reuse the event without transferring ownership.
    fn write(&self, record: &LogRecord) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LogLevel;
    use std::cell::RefCell;

    struct RecordingSink {
        written: RefCell<bool>,
    }

    impl LogSink for RecordingSink {
        fn write(&self, record: &LogRecord) -> Result<()> {
            assert_eq!(record.level(), LogLevel::Info);
            assert_eq!(record.message(), "hello");
            *self.written.borrow_mut() = true;
            Ok(())
        }
    }

    #[test]
    fn sink_can_be_used_through_a_trait_object() {
        let sink = RecordingSink {
            written: RefCell::new(false),
        };
        let record = LogRecord::new("test", LogLevel::Info, "hello");

        {
            let sink: &dyn LogSink = &sink;
            assert!(sink.write(&record).is_ok());
        }

        assert!(*sink.written.borrow());
    }
}
