use crate::environment::EnvironmentReader;
use crate::error::{ConfigError, Result};
use crate::models::Config;
use crate::types::{ConfigFormat, Environment};
use crate::validation::validate_config;
use std::fs;
use std::path::{Path, PathBuf};

/// Fluent builder and loader for NAINA OS system configuration.
#[derive(Debug, Default, Clone)]
pub struct ConfigLoader {
    file_path: Option<PathBuf>,
    environment: Option<Environment>,
}

impl ConfigLoader {
    /// Creates a new `ConfigLoader` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an explicit path to a configuration file (.toml or .json).
    pub fn with_file_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.file_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Explicitly sets the target runtime environment.
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Parses configuration content from a raw string given a specific `ConfigFormat`.
    pub fn load_from_str(content: &str, format: ConfigFormat) -> Result<Config> {
        let mut config: Config = match format {
            ConfigFormat::Toml => toml::from_str(content).map_err(|e| ConfigError::ParseError {
                message: format!("TOML error: {e}"),
            })?,
            ConfigFormat::Json => {
                serde_json::from_str(content).map_err(|e| ConfigError::ParseError {
                    message: format!("JSON error: {e}"),
                })?
            }
        };

        EnvironmentReader::apply_overrides(&mut config)?;
        validate_config(&config)?;

        Ok(config)
    }

    /// Loads, applies defaults, overrides with environment variables, validates, and returns a `Config`.
    pub fn load(&self) -> Result<Config> {
        let mut config = if let Some(path) = &self.file_path {
            if !path.exists() {
                return Err(ConfigError::FileNotFound { path: path.clone() });
            }

            let content = fs::read_to_string(path).map_err(ConfigError::Io)?;

            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

            match extension {
                "toml" => {
                    toml::from_str::<Config>(&content).map_err(|e| ConfigError::ParseError {
                        message: format!("TOML error: {e}"),
                    })?
                }
                "json" => serde_json::from_str::<Config>(&content).map_err(|e| {
                    ConfigError::ParseError {
                        message: format!("JSON error: {e}"),
                    }
                })?,
                unknown => {
                    return Err(ConfigError::ParseError {
                        message: format!("Unsupported file format extension: '{unknown}'"),
                    });
                }
            }
        } else {
            Config::default()
        };

        if let Some(env) = self.environment {
            config.environment = env;
        }

        EnvironmentReader::apply_overrides(&mut config)?;
        validate_config(&config)?;

        Ok(config)
    }
}
