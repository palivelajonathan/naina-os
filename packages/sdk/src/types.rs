//! Public state machine types for the client SDK.

/// Operational lifecycle states for [`SDKFacade`](crate::sdk_facade::SDKFacade).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SDKState {
    /// SDK facade is uninitialized.
    Uninitialized,
    /// SDK microkernel boot initialization is in progress.
    Initializing,
    /// SDK facade is initialized and ready to accept client requests.
    Ready,
    /// SDK facade has been cleanly shut down.
    Shutdown,
}
