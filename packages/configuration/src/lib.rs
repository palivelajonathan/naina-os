//! NAINA OS Configuration Package
//!
//! Provides a minimal, strongly typed configuration layer for NAINA OS.

mod config;
mod defaults;
mod environment;
mod error;
mod loader;
mod models;
mod traits;
mod types;
mod validation;

pub use error::{ConfigError, Result};
pub use loader::ConfigLoader;
pub use models::{
    AutomationConfig, BrowserConfig, Config, DesktopConfig, LogLevel, LoggingConfig, MemoryConfig,
    ModelConfig, ResourceLimits, RuntimeConfig, SecurityConfig, VoiceConfig,
};
pub use traits::ConfigProvider;
pub use types::{ConfigFormat, Environment};
