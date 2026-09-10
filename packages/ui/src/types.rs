//! Public types, states, events, and lightweight window abstractions for the UI framework.

/// Operational lifecycle states for [`UIFramework`](crate::ui_framework::UIFramework).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UIState {
    /// UI framework is uninitialized.
    Uninitialized,
    /// UI framework initialization is in progress.
    Initializing,
    /// UI framework is initialized and ready for overlay interaction.
    Ready,
    /// Overlay host window is currently visible to the user.
    OverlayVisible,
    /// Overlay host window is currently hidden/minimized.
    OverlayHidden,
    /// UI framework has been cleanly shut down.
    Shutdown,
}

/// UI framework events emitted across the `std::sync::mpsc` channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UIEvent {
    /// The desktop overlay window was shown.
    OverlayShown,
    /// The desktop overlay window was hidden.
    OverlayHidden,
    /// A user input submitted event. Retains ONLY input length for privacy.
    InputSubmitted {
        /// Number of characters submitted in the input field.
        input_length: usize,
    },
    /// A UI command action was triggered by the user.
    CommandTriggered {
        /// Name of the triggered command action.
        command_name: String,
    },
    /// A managed UI window was closed.
    WindowClosed {
        /// Unique window identifier.
        window_id: String,
    },
}

/// Abstract lightweight record representing a managed window frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowRecord {
    /// Unique window handle ID.
    pub id: String,
    /// Window title bar text.
    pub title: String,
    /// Current window width in pixels.
    pub width: u32,
    /// Current window height in pixels.
    pub height: u32,
    /// Flag indicating whether the window frame is visible.
    pub is_visible: bool,
}

impl WindowRecord {
    /// Constructs a new [`WindowRecord`].
    pub fn new(id: String, title: String, width: u32, height: u32) -> Self {
        Self {
            id,
            title,
            width,
            height,
            is_visible: false,
        }
    }
}
