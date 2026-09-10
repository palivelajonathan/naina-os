//! NAINA OS event-bus package.

pub mod config;
pub mod error;
pub mod event;
pub mod event_bus;
pub mod traits;
pub mod types;

pub use config::EventBusConfig;
pub use error::{EventBusError, Result};
pub use event::Event;
pub use event_bus::EventBus;
pub use traits::EventHandler;
pub use types::SubscriptionId;
