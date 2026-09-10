//! Configuration for the NAINA OS memory package.

/// Configuration parameters for the memory store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub storage_path: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_path: "./vault".to_string(),
        }
    }
}

impl From<configuration::MemoryConfig> for MemoryConfig {
    fn from(cfg: configuration::MemoryConfig) -> Self {
        Self {
            enabled: cfg.enabled,
            storage_path: cfg.storage_path,
        }
    }
}
