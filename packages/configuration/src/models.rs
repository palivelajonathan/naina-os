use crate::defaults::*;
use crate::types::Environment;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Root configuration for NAINA OS.
///
/// This structure aggregates every subsystem configuration into a
/// single immutable configuration object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    pub environment: Environment,
    pub host: String,
    pub port: u16,
    pub storage_path: String,
    pub resource_limits: ResourceLimits,
    pub runtime: RuntimeConfig,
    pub logging: LoggingConfig,
    pub model: ModelConfig,
    pub memory: MemoryConfig,
    pub voice: VoiceConfig,
    pub browser: BrowserConfig,
    pub desktop: DesktopConfig,
    pub automation: AutomationConfig,
    pub security: SecurityConfig,
}

impl Config {
    /// Returns true when the loaded environment is development.
    pub fn is_development(&self) -> bool {
        self.environment == Environment::Development
    }

    /// Returns true when the loaded environment is production.
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: DEFAULT_NAME.to_string(),
            environment: Environment::default(),
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            storage_path: DEFAULT_STORAGE_PATH.to_string(),
            resource_limits: ResourceLimits::default(),
            runtime: RuntimeConfig::default(),
            logging: LoggingConfig::default(),
            model: ModelConfig::default(),
            memory: MemoryConfig::default(),
            voice: VoiceConfig::default(),
            browser: BrowserConfig::default(),
            desktop: DesktopConfig::default(),
            automation: AutomationConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// Runtime configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    pub worker_threads: usize,
    pub shutdown_timeout_secs: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: 1,
            shutdown_timeout_secs: 30,
        }
    }
}

/// Logging configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub directory: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            directory: DEFAULT_LOG_DIRECTORY.to_string(),
        }
    }
}

/// AI model configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelConfig {
    pub provider: String,
    pub model_name: String,
    pub context_window: usize,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: String::new(),
            model_name: String::new(),
            context_window: 2048,
        }
    }
}

/// Memory subsystem configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub storage_path: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            storage_path: DEFAULT_MEMORY_STORAGE_PATH.to_string(),
        }
    }
}

/// Voice runtime configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub wake_word: String,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            wake_word: DEFAULT_VOICE_WAKE_WORD.to_string(),
        }
    }
}

/// Browser runtime configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    pub enabled: bool,
}

/// Desktop runtime configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopConfig {
    pub enabled: bool,
}

/// Automation engine configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutomationConfig {
    pub enabled: bool,
}

/// Security configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub allow_unsafe_operations: bool,
}

/// Logging levels supported by NAINA.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        };
        write!(f, "{text}")
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" | "err" => Ok(Self::Error),
            invalid => Err(format!("Unknown log level: '{invalid}'")),
        }
    }
}

/// Resource limits used by NAINA OS.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_ram_mb: u64,
    pub max_latency_ms: u64,
    pub max_vram_mb: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_ram_mb: DEFAULT_MAX_RAM_MB,
            max_latency_ms: DEFAULT_MAX_LATENCY_MS,
            max_vram_mb: DEFAULT_MAX_VRAM_MB,
        }
    }
}
