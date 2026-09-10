//! Types for the NAINA OS runtime package.

/// Unique identifier for a runtime execution context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionContextId(pub u64);

/// System lifecycle states for the NAINA OS runtime execution framework.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeState {
    Uninitialized,
    Initializing,
    Ready,
    ShuttingDown,
    Stopped,
    Failed,
}

/// Execution state of a runtime context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextState {
    Created,
    Running,
    Completed,
    Failed { error: String },
    Closed,
}

/// Execution environment record for a worker subsystem task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionContext {
    pub id: ExecutionContextId,
    pub name: String,
    pub state: ContextState,
    pub kernel_process_id: kernel::ProcessId,
}
