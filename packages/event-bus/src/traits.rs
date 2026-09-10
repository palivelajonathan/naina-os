//! Traits for the NAINA OS event-bus package.

use crate::error::Result;
use crate::event::Event;

/// Handler trait for processing events received from the event bus.
pub trait EventHandler: Send + Sync {
    /// Handles an incoming event.
    fn handle(&self, event: &Event) -> Result<()>;
}
