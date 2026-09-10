//! NAINA OS logging package.

pub mod config;
pub mod error;
pub mod level;
pub mod logger;
pub mod record;
pub mod sink;

pub use config::LoggerConfig;
pub use error::Result;
pub use level::LogLevel;
pub use logger::Logger;
pub use record::{LogFields, LogRecord};
pub use sink::LogSink;
