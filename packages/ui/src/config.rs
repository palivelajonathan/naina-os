//! User Interface Framework configuration parameters and models.

use std::fmt;

/// Configuration parameters for initializing the [`UIFramework`](crate::ui_framework::UIFramework).
#[derive(Clone, PartialEq, Eq)]
pub struct UIConfig {
    /// Window title string (default: "NAINA OS Desktop Host").
    pub title: String,
    /// Flag indicating whether desktop overlay host mode is enabled (default: true).
    pub enable_overlay: bool,
    /// Default window width in pixels (default: 1280).
    pub width: u32,
    /// Default window height in pixels (default: 800).
    pub height: u32,
    /// Target overlay refresh rate in Hz (default: 60).
    pub refresh_rate_hz: u32,
    /// Flag indicating whether the overlay should automatically show when ready (default: true).
    pub auto_show: bool,
}

impl fmt::Debug for UIConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UIConfig")
            .field("title", &self.title)
            .field("enable_overlay", &self.enable_overlay)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("refresh_rate_hz", &self.refresh_rate_hz)
            .field("auto_show", &self.auto_show)
            .finish()
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            title: "NAINA OS Desktop Host".to_string(),
            enable_overlay: true,
            width: 1280,
            height: 800,
            refresh_rate_hz: 60,
            auto_show: true,
        }
    }
}
