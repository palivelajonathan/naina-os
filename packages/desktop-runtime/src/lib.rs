//! NAINA OS desktop-runtime package.

pub mod config;
pub mod desktop_runtime;
pub mod error;
pub mod platform;
pub mod traits;
pub mod types;

pub use config::DesktopRuntimeConfig;
pub use desktop_runtime::DesktopRuntime;
pub use error::{DesktopRuntimeError, Result};
pub use platform::{MockDesktopEngine, Win32DesktopEngine};
pub use traits::DesktopAutomationEngine;
pub use types::{AppLaunchRequest, AppLaunchResult, DesktopState, Rect, UiElement, WindowInfo};
