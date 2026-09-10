//! Abstraction traits for desktop automation engine providers.

use crate::error::Result;
use crate::types::{AppLaunchRequest, AppLaunchResult, UiElement, WindowInfo};
use std::fmt::Debug;

/// Trait implemented by native Win32/COM engines and mock desktop automation providers.
pub trait DesktopAutomationEngine: Debug + Send + Sync {
    /// Returns the descriptive name of the engine provider.
    fn engine_name(&self) -> &str;

    /// Launches a native application process.
    fn launch_app(&self, request: &AppLaunchRequest) -> Result<AppLaunchResult>;

    /// Enumerates active desktop top-level windows.
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>>;

    /// Inspects semantic UI elements within a specified window handle.
    fn inspect_ui_elements(
        &self,
        window_handle: u64,
        max_depth: usize,
        max_elements: usize,
    ) -> Result<Vec<UiElement>>;

    /// Inject synthetic input event into a window target.
    fn inject_input(&self, window_handle: u64, action: &str) -> Result<()>;
}
