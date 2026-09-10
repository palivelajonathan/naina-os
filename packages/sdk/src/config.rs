//! Client SDK configuration parameters and default models.

use std::fmt;

/// Configuration options for initializing the [`SDKFacade`](crate::sdk_facade::SDKFacade).
#[derive(Clone, PartialEq, Eq)]
pub struct SDKConfig {
    /// Human-readable system name string (default: "NAINA OS").
    pub system_name: String,
    /// Flag indicating whether system logging should be enabled (default: true).
    pub enable_logging: bool,
    /// Flag indicating whether the microkernel runtime should auto-start on build (default: true).
    pub auto_start_runtime: bool,
    /// Active session timeout duration in milliseconds (default: 3,600,000 ms / 1 hour).
    pub session_timeout_ms: u64,
}

impl fmt::Debug for SDKConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SDKConfig")
            .field("system_name", &self.system_name)
            .field("enable_logging", &self.enable_logging)
            .field("auto_start_runtime", &self.auto_start_runtime)
            .field("session_timeout_ms", &self.session_timeout_ms)
            .finish()
    }
}

impl Default for SDKConfig {
    fn default() -> Self {
        Self {
            system_name: "NAINA OS".to_string(),
            enable_logging: true,
            auto_start_runtime: true,
            session_timeout_ms: 3_600_000,
        }
    }
}
