//! Primary data types for browser automation and CDP page inspection.

/// Active pipeline state of the browser runtime supervisor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrowserState {
    /// Runtime is idle and ready to accept browser actions.
    Idle,
    /// Spawning or attaching to a Chromium browser instance.
    StartingBrowser,
    /// Executing page navigation or URL loading.
    Navigating,
    /// Querying semantic DOM node hierarchy via CDP.
    InspectingDom,
    /// Extracting and simplifying page text content.
    ExtractingText,
    /// Supervisor encountered an error or failed action.
    Error,
}

/// Information metadata regarding an open browser tab or page target.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageInfo {
    /// Unique CDP target ID.
    pub target_id: String,
    /// Active URL of the page.
    pub url: String,
    /// Page document title.
    pub title: String,
    /// Whether the page is actively loading.
    pub is_loading: bool,
}

/// Semantic DOM node representation extracted via CDP DOM domain.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DomNode {
    /// CDP node identifier.
    pub node_id: u32,
    /// HTML tag name (e.g. "div", "button", "p").
    pub tag_name: String,
    /// Key-value attribute pairs.
    pub attributes: Vec<(String, String)>,
    /// Text content inside the element node.
    pub text_content: String,
    /// Total count of direct child nodes.
    pub children_count: usize,
}

/// Parameters for dispatching a browser automation action.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowserActionRequest {
    /// Optional target URL.
    pub url: Option<String>,
    /// Action type description (e.g. "navigate", "click", "type").
    pub action_type: String,
    /// Optional CSS / XPath target selector string.
    pub target_selector: Option<String>,
    /// Optional text payload for form filling.
    pub text_payload: Option<String>,
}

/// Result payload returned from a browser action execution.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowserActionResult {
    /// CDP target tab ID.
    pub target_id: String,
    /// Current URL after action completion.
    pub current_url: String,
    /// Execution status message.
    pub status: String,
    /// Extracted page text payload if requested.
    pub extracted_text: Option<String>,
}
