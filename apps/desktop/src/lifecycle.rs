//! Windows process signal interception and lifecycle coordination.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Process shutdown signal watcher for Windows host applications.
#[derive(Debug, Clone)]
pub struct SignalWatcher {
    shutdown_requested: Arc<AtomicBool>,
}

impl Default for SignalWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalWatcher {
    /// Constructs a new [`SignalWatcher`].
    pub fn new() -> Self {
        Self {
            shutdown_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Triggers a graceful shutdown signal.
    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
    }

    /// Returns `true` if a shutdown signal has been requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }
}
