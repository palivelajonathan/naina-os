//! Event model for the NAINA OS event-bus package.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);

/// A concrete structured event carrying topic metadata and attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    id: String,
    event_type: String,
    source: String,
    timestamp: SystemTime,
    payload: BTreeMap<String, String>,
}

impl Event {
    /// Creates a new event with a deterministically generated ID and current system timestamp.
    pub fn new(event_type: impl Into<String>, source: impl Into<String>) -> Self {
        let seq = NEXT_EVENT_ID.fetch_add(1, Ordering::SeqCst);
        let id = format!("evt_{seq}");
        Self {
            id,
            event_type: event_type.into(),
            source: source.into(),
            timestamp: SystemTime::now(),
            payload: BTreeMap::new(),
        }
    }

    /// Adds structured payload fields to the event.
    pub fn with_payload(mut self, payload: BTreeMap<String, String>) -> Self {
        self.payload = payload;
        self
    }

    /// Returns the unique identifier of the event.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the event type/topic.
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Returns the source component that published the event.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the timestamp when the event was created.
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    /// Returns the structured payload fields of the event.
    pub fn payload(&self) -> &BTreeMap<String, String> {
        &self.payload
    }
}
