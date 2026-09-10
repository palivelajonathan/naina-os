//! Configuration for the logging package.

use crate::level::LogLevel;

/// Minimal configuration consumed by the logging package.
///
/// At this stage, the logger only needs a default log level. The rest of the
/// configuration handling remains the responsibility of the configuration
/// package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoggerConfig {
    level: LogLevel,
}

impl LoggerConfig {
    /// Creates a logger configuration with an explicit level.
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }

    /// Returns the configured log level.
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Replaces the configured log level.
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

impl From<LogLevel> for LoggerConfig {
    fn from(level: LogLevel) -> Self {
        Self::new(level)
    }
}

impl From<configuration::LoggingConfig> for LoggerConfig {
    fn from(config: configuration::LoggingConfig) -> Self {
        Self::new(config.level.into())
    }
}

impl From<&configuration::LoggingConfig> for LoggerConfig {
    fn from(config: &configuration::LoggingConfig) -> Self {
        Self::new(config.level.into())
    }
}

impl From<configuration::Config> for LoggerConfig {
    fn from(config: configuration::Config) -> Self {
        Self::from(config.logging)
    }
}

impl From<&configuration::Config> for LoggerConfig {
    fn from(config: &configuration::Config) -> Self {
        Self::from(&config.logging)
    }
}

impl From<configuration::LogLevel> for LogLevel {
    fn from(level: configuration::LogLevel) -> Self {
        match level {
            configuration::LogLevel::Trace => Self::Trace,
            configuration::LogLevel::Debug => Self::Debug,
            configuration::LogLevel::Info => Self::Info,
            configuration::LogLevel::Warn => Self::Warn,
            configuration::LogLevel::Error => Self::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use configuration::{Config, LoggingConfig};

    #[test]
    fn defaults_to_info_level() {
        let config = LoggerConfig::default();

        assert_eq!(config.level(), LogLevel::Info);
    }

    #[test]
    fn explicit_level_is_preserved() {
        let config = LoggerConfig::new(LogLevel::Debug);

        assert_eq!(config.level(), LogLevel::Debug);
    }

    #[test]
    fn builder_style_update_replaces_level() {
        let config = LoggerConfig::default().with_level(LogLevel::Warn);

        assert_eq!(config.level(), LogLevel::Warn);
    }

    #[test]
    fn converts_from_configuration_logging_config() {
        let source = LoggingConfig {
            level: configuration::LogLevel::Trace,
            directory: "logs".to_string(),
        };

        let config = LoggerConfig::from(source);

        assert_eq!(config.level(), LogLevel::Trace);
    }

    #[test]
    fn converts_from_root_configuration() {
        let source = Config {
            name: "demo".to_string(),
            environment: configuration::Environment::Development,
            host: "127.0.0.1".to_string(),
            port: 8080,
            storage_path: "/tmp/naina".to_string(),
            resource_limits: configuration::ResourceLimits::default(),
            runtime: configuration::RuntimeConfig::default(),
            logging: LoggingConfig {
                level: configuration::LogLevel::Error,
                directory: "logs".to_string(),
            },
            model: configuration::ModelConfig::default(),
            memory: configuration::MemoryConfig::default(),
            voice: configuration::VoiceConfig::default(),
            browser: configuration::BrowserConfig::default(),
            desktop: configuration::DesktopConfig::default(),
            automation: configuration::AutomationConfig::default(),
            security: configuration::SecurityConfig::default(),
        };

        let config = LoggerConfig::from(source);

        assert_eq!(config.level(), LogLevel::Error);
    }
}
