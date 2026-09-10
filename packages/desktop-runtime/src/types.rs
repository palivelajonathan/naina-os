//! Primary data types for desktop automation and window management.

/// Active pipeline state of the desktop runtime supervisor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DesktopState {
    /// Runtime is idle and ready to accept desktop actions.
    Idle,
    /// Launching a native application process via Win32.
    LaunchingApp,
    /// Inspecting semantic UI elements via UI Automation.
    InspectingUi,
    /// Injecting synthetic input events via SendInput.
    InjectingInput,
    /// Supervisor encountered an error or failed action.
    Error,
}

/// 2D rectangular bounding box for windows and UI elements.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Window metadata and state information.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowInfo {
    /// Serialized HWND window handle represented as a 64-bit unsigned integer.
    pub handle: u64,
    /// Window title text.
    pub title: String,
    /// Process ID owning the window.
    pub process_id: u32,
    /// Window bounding rectangle.
    pub bounds: Rect,
    /// Whether the window is visible on screen.
    pub is_visible: bool,
}

/// Semantic UI element definition obtained via UI Automation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UiElement {
    /// Unique identifier for the element.
    pub element_id: String,
    /// Accessible element name.
    pub name: String,
    /// Control type string (e.g. "Button", "Edit", "Window").
    pub control_type: String,
    /// Element bounding rectangle.
    pub bounds: Rect,
}

/// Parameters for launching a native desktop application.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppLaunchRequest {
    /// Application name (e.g. "notepad", "vscode").
    pub app_name: String,
    /// Optional explicit path to executable.
    pub executable_path: Option<String>,
    /// Command line arguments.
    pub arguments: Vec<String>,
}

/// Result payload returned upon application process launch.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppLaunchResult {
    /// Launched process ID.
    pub process_id: u32,
    /// Main window handle if available (or 0).
    pub window_handle: u64,
    /// Status description.
    pub status: String,
}
