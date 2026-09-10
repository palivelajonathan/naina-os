//! Configuration definitions for desktop-runtime.

/// Configuration parameters for desktop automation, overlay bounds, and UI inspection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopRuntimeConfig {
    /// Maximum search depth for UI Automation element tree traversal.
    pub max_search_depth: usize,
    /// Maximum number of UI elements returned per inspection.
    pub max_element_count: usize,
    /// Maximum timeout in milliseconds for UI element tree inspection.
    pub inspection_timeout_ms: u64,
    /// Target latency budget for desktop command execution in milliseconds.
    pub latency_target_ms: u64,
    /// RAM limit for the desktop overlay host in megabytes.
    pub ram_limit_mb: usize,
    /// Whether desktop runtime automation is enabled.
    pub enabled: bool,
}

impl Default for DesktopRuntimeConfig {
    fn default() -> Self {
        Self {
            max_search_depth: 5,
            max_element_count: 100,
            inspection_timeout_ms: 200,
            latency_target_ms: 500,
            ram_limit_mb: 200,
            enabled: true,
        }
    }
}

impl DesktopRuntimeConfig {
    /// Populates [`DesktopRuntimeConfig`] from root [`configuration::DesktopConfig`].
    pub fn from_desktop_config(desktop_config: &configuration::DesktopConfig) -> Self {
        Self {
            enabled: desktop_config.enabled,
            ..Default::default()
        }
    }
}
