//! Configuration types for the NAINA OS orchestrator package.

/// Configuration parameters for the orchestrator execution engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrchestratorConfig {
    /// Maximum allowed execution loop steps per task to prevent infinite loops.
    pub max_steps: usize,
    /// Maximum allowed execution duration in seconds per task.
    pub timeout_seconds: u64,
    /// Maximum allowed retries per failed execution step.
    pub retry_limit: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            timeout_seconds: 60,
            retry_limit: 2,
        }
    }
}
