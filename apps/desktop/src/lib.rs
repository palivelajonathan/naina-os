//! NAINA OS Desktop Host application composition root library.

pub mod config;
pub mod desktop_host;
pub mod error;
pub mod lifecycle;
pub mod types;

pub use config::DesktopHostConfig;
pub use desktop_host::DesktopHostApp;
pub use error::{DesktopHostError, DesktopHostResult};
pub use lifecycle::SignalWatcher;
pub use types::DesktopHostState;
