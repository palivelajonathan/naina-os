//! Lifecycle state machine models for the Desktop Host application.

/// Operational lifecycle states for [`DesktopHostApp`](crate::desktop_host::DesktopHostApp).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DesktopHostState {
    /// Desktop host is uninitialized.
    Uninitialized,
    /// Desktop host composition root boot sequence is in progress.
    Booting,
    /// Desktop host is booted and ready to process MVN cognitive turns.
    Ready,
    /// Desktop host is actively processing an end-to-end cognitive loop turn.
    ProcessingTurn,
    /// Desktop host is undergoing graceful shutdown.
    ShuttingDown,
    /// Desktop host has shut down completely.
    Stopped,
}
