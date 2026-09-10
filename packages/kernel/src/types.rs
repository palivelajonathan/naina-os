//! Types for the NAINA OS kernel package.

/// Unique identifier for a monitored process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(pub u64);

/// System lifecycle states for the NAINA OS microkernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelState {
    Uninitialized,
    Booting,
    Running,
    ShuttingDown,
    Stopped,
    Failed,
}

/// Execution and health states for a monitored process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessState {
    Registered,
    Starting,
    Running,
    Stopped,
    Failed { error: String },
}

/// Restart policy for crash recovery under process supervision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    OnFailure { max_retries: u32 },
}

/// Record representing a monitored subsystem process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessRecord {
    pub id: ProcessId,
    pub name: String,
    pub state: ProcessState,
    pub policy: RestartPolicy,
    pub restart_count: u32,
}
