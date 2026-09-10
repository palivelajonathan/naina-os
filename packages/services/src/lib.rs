//! NAINA OS services package.

pub mod config;
pub mod error;
pub mod service_registry;
pub mod traits;
pub mod types;

pub use config::ServicesConfig;
pub use error::{Result, ServicesError};
pub use service_registry::ServiceRegistry;
pub use types::{ServiceHealth, ServiceId, ServiceName, ServiceRecord, ServiceState};
