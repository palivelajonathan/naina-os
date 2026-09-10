//! Configuration for the NAINA OS kernel package.

use std::time::Duration;

/// Configuration parameters for the microkernel supervisor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelConfig {
    pub boot_timeout: Duration,
    pub max_process_restarts: u32,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            boot_timeout: Duration::from_secs(2),
            max_process_restarts: 3,
        }
    }
}
