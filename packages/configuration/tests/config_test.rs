use configuration::{Config, ConfigError, ConfigFormat, ConfigLoader, ConfigProvider, LogLevel};
use serial_test::serial;
use std::env;

struct TempEnv {
    original: Vec<(String, Option<String>)>,
}

impl TempEnv {
    fn set(vars: &[(&str, &str)]) -> Self {
        let original = vars
            .iter()
            .map(|(key, _)| (key.to_string(), env::var(key).ok()))
            .collect();
        for (key, value) in vars {
            unsafe { env::set_var(key, value) };
        }
        Self { original }
    }

    fn clear(keys: &[&str]) -> Self {
        let original = keys
            .iter()
            .map(|key| (key.to_string(), env::var(key).ok()))
            .collect();
        for key in keys {
            unsafe { env::remove_var(key) };
        }
        Self { original }
    }
}

impl Drop for TempEnv {
    fn drop(&mut self) {
        for (key, value) in self.original.drain(..) {
            if let Some(value) = value {
                unsafe { env::set_var(key, value) };
            } else {
                unsafe { env::remove_var(key) };
            }
        }
    }
}

#[test]
fn test_default_config_contains_subsystem_blocks() {
    let config = Config::default();

    assert_eq!(config.runtime.worker_threads, 1);
    assert_eq!(config.runtime.shutdown_timeout_secs, 30);
    assert_eq!(config.logging.level, LogLevel::Info);
    assert!(!config.security.allow_unsafe_operations);
}

#[test]
fn test_config_provider_is_implemented() {
    let config = Config::default();
    assert_eq!(config.get_config().name, "NAINA OS");
    assert!(config.is_development());
}

#[test]
#[serial]
fn test_load_from_str_parses_toml_and_applies_defaults() {
    let _env_guard = TempEnv::clear(&[
        "NAINA_ENV",
        "NAINA_NAME",
        "NAINA_HOST",
        "NAINA_PORT",
        "NAINA_LOG_LEVEL",
        "NAINA_STORAGE_PATH",
    ]);

    let content = r#"
name = "test-system"
[logging]
level = "debug"
"#;
    let config = ConfigLoader::load_from_str(content, ConfigFormat::Toml).unwrap();

    assert_eq!(config.name, "test-system");
    assert_eq!(config.logging.level, LogLevel::Debug);
    assert_eq!(config.host, "127.0.0.1");
}

#[test]
#[serial]
fn test_load_uses_environment_overrides() {
    let _env_guard = TempEnv::set(&[
        ("NAINA_LOG_LEVEL", "warn"),
        ("NAINA_HOST", "10.1.1.1"),
        ("NAINA_PORT", "3000"),
    ]);

    let config = ConfigLoader::new().load().unwrap();

    assert_eq!(config.logging.level, LogLevel::Warn);
    assert_eq!(config.host, "10.1.1.1");
    assert_eq!(config.port, 3000);
}

#[test]
#[serial]
fn test_invalid_environment_override_returns_error() {
    let _env_guard = TempEnv::set(&[("NAINA_LOG_LEVEL", "invalid-level")]);

    let err = ConfigLoader::new().load().unwrap_err();

    assert!(
        matches!(err, ConfigError::EnvironmentError { variable, .. } if variable == "NAINA_LOG_LEVEL")
    );
}

#[test]
#[serial]
fn test_invalid_config_validation_returns_error() {
    let _env_guard = TempEnv::clear(&[
        "NAINA_ENV",
        "NAINA_NAME",
        "NAINA_HOST",
        "NAINA_PORT",
        "NAINA_LOG_LEVEL",
        "NAINA_STORAGE_PATH",
    ]);

    let content = r#"port = 0"#;
    let err = ConfigLoader::load_from_str(content, ConfigFormat::Toml).unwrap_err();

    assert!(matches!(err, ConfigError::ValidationError { .. }));
}
