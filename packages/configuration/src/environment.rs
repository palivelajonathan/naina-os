use crate::error::{ConfigError, Result};
use crate::models::{Config, LogLevel};
use crate::types::Environment;
use std::env;

/// Reader utility for inspecting and overriding configuration settings from environment variables.
#[derive(Debug, Default)]
pub struct EnvironmentReader;

impl EnvironmentReader {
    /// Reads environment variables starting with `NAINA_` and applies overrides to the `Config`.
    pub fn apply_overrides(config: &mut Config) -> Result<()> {
        if let Ok(env_str) = env::var("NAINA_ENV") {
            config.environment =
                env_str
                    .parse::<Environment>()
                    .map_err(|reason| ConfigError::EnvironmentError {
                        variable: "NAINA_ENV".to_string(),
                        reason,
                    })?;
        }

        if let Ok(name) = env::var("NAINA_NAME") {
            if name.trim().is_empty() {
                return Err(ConfigError::EnvironmentError {
                    variable: "NAINA_NAME".to_string(),
                    reason: "value cannot be empty".to_string(),
                });
            }
            config.name = name;
        }

        if let Ok(host) = env::var("NAINA_HOST") {
            if host.trim().is_empty() {
                return Err(ConfigError::EnvironmentError {
                    variable: "NAINA_HOST".to_string(),
                    reason: "value cannot be empty".to_string(),
                });
            }
            config.host = host;
        }

        if let Ok(port_str) = env::var("NAINA_PORT") {
            let port = port_str
                .parse::<u16>()
                .map_err(|_| ConfigError::EnvironmentError {
                    variable: "NAINA_PORT".to_string(),
                    reason: "must be a valid port number".to_string(),
                })?;

            if port == 0 {
                return Err(ConfigError::EnvironmentError {
                    variable: "NAINA_PORT".to_string(),
                    reason: "must be greater than 0".to_string(),
                });
            }

            config.port = port;
        }

        if let Ok(log_level) = env::var("NAINA_LOG_LEVEL") {
            if log_level.trim().is_empty() {
                return Err(ConfigError::EnvironmentError {
                    variable: "NAINA_LOG_LEVEL".to_string(),
                    reason: "value cannot be empty".to_string(),
                });
            }
            config.logging.level =
                log_level
                    .parse::<LogLevel>()
                    .map_err(|reason| ConfigError::EnvironmentError {
                        variable: "NAINA_LOG_LEVEL".to_string(),
                        reason,
                    })?;
        }

        if let Ok(storage_path) = env::var("NAINA_STORAGE_PATH") {
            if storage_path.trim().is_empty() {
                return Err(ConfigError::EnvironmentError {
                    variable: "NAINA_STORAGE_PATH".to_string(),
                    reason: "value cannot be empty".to_string(),
                });
            }
            config.storage_path = storage_path;
        }

        Ok(())
    }
}
