//! NAINA OS browser-runtime package.

pub mod browser_runtime;
pub mod config;
pub mod error;
pub mod platform;
pub mod traits;
pub mod types;

pub use browser_runtime::BrowserRuntime;
pub use config::BrowserRuntimeConfig;
pub use error::{BrowserRuntimeError, Result};
pub use platform::{CdpBrowserEngine, CdpTransport, ChildProcessGuard, MockBrowserEngine};
pub use traits::BrowserAutomationEngine;
pub use types::{BrowserActionRequest, BrowserActionResult, BrowserState, DomNode, PageInfo};
