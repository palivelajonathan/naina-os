//! NAINA OS capabilities package.

pub mod capability_registry;
pub mod config;
pub mod error;
pub mod traits;
pub mod types;

pub use capability_registry::CapabilityRegistry;
pub use config::CapabilityConfig;
pub use error::{CapabilityError, Result};
pub use types::CapabilityToken;
