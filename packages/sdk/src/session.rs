//! Session models and tracking for client SDK handles.

use runtime::ExecutionContextId;

/// Active client session handle bound to a microkernel [`ExecutionContextId`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SDKSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Underlying microkernel execution context ID.
    pub context_id: ExecutionContextId,
    /// Unix timestamp in milliseconds when the session was created.
    pub created_at_ms: u64,
    /// Flag indicating whether the session is active.
    pub is_active: bool,
}

impl SDKSession {
    /// Constructs a new [`SDKSession`] instance.
    pub fn new(session_id: String, context_id: ExecutionContextId, created_at_ms: u64) -> Self {
        Self {
            session_id,
            context_id,
            created_at_ms,
            is_active: true,
        }
    }

    /// Returns `true` if the session has exceeded the specified timeout duration.
    pub fn is_expired(&self, current_time_ms: u64, timeout_ms: u64) -> bool {
        if !self.is_active {
            return true;
        }
        current_time_ms.saturating_sub(self.created_at_ms) > timeout_ms
    }
}
