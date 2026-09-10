//! Abstraction traits and mock implementations for workflow step execution.

use crate::error::{AutomationError, Result};
use crate::types::{StepResult, WorkflowStep};
use std::fmt::Debug;

/// Interface trait for workflow step execution providers.
pub trait WorkflowStepRunner: Send + Sync + Debug {
    /// Executes a single workflow step and returns execution telemetry.
    fn execute_step(&self, step: &WorkflowStep) -> Result<StepResult>;
}

/// Deterministic mock step runner usable in CI offline testing.
#[derive(Debug, Default)]
pub struct MockWorkflowStepRunner {
    /// Configurable flag to simulate step failure for testing.
    pub should_fail_step_id: Option<String>,
    /// Configurable flag to simulate capability denial for testing.
    pub should_deny_capability_step_id: Option<String>,
}

impl MockWorkflowStepRunner {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WorkflowStepRunner for MockWorkflowStepRunner {
    fn execute_step(&self, step: &WorkflowStep) -> Result<StepResult> {
        if let Some(ref deny_id) = self.should_deny_capability_step_id {
            if deny_id == &step.step_id {
                return Err(AutomationError::CapabilityDenied {
                    step_id: step.step_id.clone(),
                    message: "Mock capability denial".to_string(),
                });
            }
        }

        if let Some(ref fail_id) = self.should_fail_step_id {
            if fail_id == &step.step_id {
                return Err(AutomationError::StepExecutionFailed {
                    step_id: step.step_id.clone(),
                    message: "Mock simulated step error".to_string(),
                });
            }
        }

        Ok(StepResult {
            step_id: step.step_id.clone(),
            status: "Completed".to_string(),
            output: Some(format!(
                "Executed action '{}' with payload '{}'",
                step.action_type, step.payload
            )),
            elapsed_ms: 10,
            error: None,
        })
    }
}
