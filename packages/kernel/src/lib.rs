//! NAINA OS kernel package.

pub mod config;
pub mod error;
pub mod kernel;
pub mod traits;
pub mod types;

pub use capabilities;
pub use config::KernelConfig;
pub use error::{KernelError, Result};
pub use event_bus;
pub use kernel::Kernel;
pub use types::{KernelState, ProcessId, ProcessRecord, ProcessState, RestartPolicy};
