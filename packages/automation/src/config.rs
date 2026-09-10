//! Configuration models and defaults for the NAINA OS workflow automation engine.

use std::fmt;

/// Configuration parameters for [`AutomationEngine`](crate::automation_engine::AutomationEngine).
#[derive(Clone, PartialEq, Eq)]
pub struct AutomationConfig {
    /// Maximum allowed steps per workflow specification (default: 20).
    pub max_steps_per_workflow: usize,
    /// Per-step timeout limit in milliseconds (default: 5,000 ms).
    pub step_timeout_ms: u64,
    /// Total workflow timeout limit in milliseconds (default: 30,000 ms).
    pub workflow_timeout_ms: u64,
    /// Maximum retry attempts for retryable step failures (default: 3).
    pub max_retry_attempts: u32,
    /// Architectural WASM/plugin load performance target in milliseconds (default: 100 ms).
    pub wasm_plugin_timeout_ms: u64,
    /// Flag indicating whether workflow automation execution is enabled.
    pub enabled: bool,
}

impl fmt::Debug for AutomationConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutomationConfig")
            .field("max_steps_per_workflow", &self.max_steps_per_workflow)
            .field("step_timeout_ms", &self.step_timeout_ms)
            .field("workflow_timeout_ms", &self.workflow_timeout_ms)
            .field("max_retry_attempts", &self.max_retry_attempts)
            .field("wasm_plugin_timeout_ms", &self.wasm_plugin_timeout_ms)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            max_steps_per_workflow: 20,
            step_timeout_ms: 5_000,
            workflow_timeout_ms: 30_000,
            max_retry_attempts: 3,
            wasm_plugin_timeout_ms: 100,
            enabled: true,
        }
    }
}
