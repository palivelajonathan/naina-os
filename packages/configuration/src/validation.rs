use crate::error::ConfigError;
use crate::models::Config;

/// Validates a `Config` instance against system invariants and resource budgets.
pub fn validate_config(config: &Config) -> Result<(), ConfigError> {
    if config.name.trim().is_empty() {
        return Err(ConfigError::ValidationError {
            message: "Application name cannot be empty".to_string(),
        });
    }

    if config.host.trim().is_empty() {
        return Err(ConfigError::ValidationError {
            message: "Host address cannot be empty".to_string(),
        });
    }

    if config.port == 0 {
        return Err(ConfigError::ValidationError {
            message: "Port number must be between 1 and 65535".to_string(),
        });
    }

    if config.storage_path.trim().is_empty() {
        return Err(ConfigError::ValidationError {
            message: "Storage path cannot be empty".to_string(),
        });
    }

    if config.resource_limits.max_ram_mb == 0 {
        return Err(ConfigError::ValidationError {
            message: "Maximum RAM allocation limit must be greater than 0".to_string(),
        });
    }

    if config.resource_limits.max_latency_ms == 0 {
        return Err(ConfigError::ValidationError {
            message: "Maximum latency must be greater than 0".to_string(),
        });
    }

    if config.resource_limits.max_vram_mb == 0 {
        return Err(ConfigError::ValidationError {
            message: "Maximum VRAM allocation limit must be greater than 0".to_string(),
        });
    }

    if config.runtime.worker_threads == 0 {
        return Err(ConfigError::ValidationError {
            message: "Runtime worker_threads must be greater than 0".to_string(),
        });
    }

    if config.runtime.shutdown_timeout_secs == 0 {
        return Err(ConfigError::ValidationError {
            message: "Runtime shutdown_timeout_secs must be greater than 0".to_string(),
        });
    }

    Ok(())
}
