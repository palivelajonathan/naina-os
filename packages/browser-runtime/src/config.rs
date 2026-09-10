//! Configuration definitions for browser-runtime.

/// Configuration parameters for browser process supervision, CDP connection, and navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRuntimeConfig {
    /// Whether to run Chromium in headless mode (`--headless=new`).
    pub headless: bool,
    /// CDP remote debugging port.
    pub remote_debugging_port: u16,
    /// Maximum navigation timeout in milliseconds before raising error.
    pub navigation_timeout_ms: u64,
    /// Target latency budget for browser page actions in milliseconds.
    pub latency_target_ms: u64,
    /// Whether browser runtime automation is enabled.
    pub enabled: bool,
    /// Optional explicit path to browser executable binary.
    pub browser_executable_path: Option<String>,
}

impl Default for BrowserRuntimeConfig {
    fn default() -> Self {
        Self {
            headless: true,
            remote_debugging_port: 9222,
            navigation_timeout_ms: 10_000,
            latency_target_ms: 1_000,
            enabled: true,
            browser_executable_path: None,
        }
    }
}

impl BrowserRuntimeConfig {
    /// Populates [`BrowserRuntimeConfig`] from root [`configuration::BrowserConfig`].
    pub fn from_browser_config(browser_config: &configuration::BrowserConfig) -> Self {
        Self {
            enabled: browser_config.enabled,
            ..Default::default()
        }
    }
}
