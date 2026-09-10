//! Abstraction traits for browser automation engine providers.

use crate::error::Result;
use crate::types::{BrowserActionResult, DomNode, PageInfo};
use std::fmt::Debug;

/// Trait implemented by native CDP browser engines and mock browser automation providers.
pub trait BrowserAutomationEngine: Debug + Send + Sync {
    /// Returns the descriptive name of the engine provider.
    fn engine_name(&self) -> &str;

    /// Navigates the active browser tab to a specified URL.
    fn navigate_to(&self, url: &str) -> Result<BrowserActionResult>;

    /// Opens a new browser tab/target pointing to a specified URL.
    fn open_tab(&self, url: &str) -> Result<PageInfo>;

    /// Closes a target browser tab by ID.
    fn close_tab(&self, target_id: &str) -> Result<()>;

    /// Inspects semantic DOM nodes within a target tab.
    fn inspect_dom(&self, target_id: &str) -> Result<Vec<DomNode>>;

    /// Extracts clean readable text payload from a target tab.
    fn extract_page_text(&self, target_id: &str) -> Result<String>;
}
