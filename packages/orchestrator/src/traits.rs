//! Trait abstractions for the CARF task planner.

use crate::error::{OrchestratorError, Result};
use crate::types::{ExecutionPlan, ExecutionState, ExecutionStep, TaskRequest};
use std::fmt::Debug;

/// Interface contract for CARF task planning engine integration.
pub trait Planner: Send + Sync + Debug {
    /// Returns the name of the planning strategy.
    fn plan_name(&self) -> &str;

    /// Generates an [`ExecutionPlan`] for a given [`TaskRequest`].
    fn create_plan(&self, request: &TaskRequest) -> Result<ExecutionPlan>;
}

/// Default CARF-compliant task planner for NAINA OS.
#[derive(Debug, Default)]
pub struct DefaultPlanner {
    max_allowed_steps: usize,
}

impl DefaultPlanner {
    /// Constructs a new [`DefaultPlanner`] with a max allowed step ceiling of 10.
    pub fn new() -> Self {
        Self {
            max_allowed_steps: 10,
        }
    }

    /// Constructs a [`DefaultPlanner`] with a custom max step limit.
    pub fn with_max_steps(max_steps: usize) -> Self {
        Self {
            max_allowed_steps: max_steps,
        }
    }
}

impl Planner for DefaultPlanner {
    fn plan_name(&self) -> &str {
        "carf-default-planner"
    }

    fn create_plan(&self, request: &TaskRequest) -> Result<ExecutionPlan> {
        if request.prompt.trim().is_empty() {
            return Err(OrchestratorError::PlanningFailed {
                message: "Cannot create plan for empty prompt".to_string(),
            });
        }

        let plan_id = format!("plan-{}", request.task_id);
        let mut steps = Vec::new();
        let prompt_lower = request.prompt.to_lowercase();

        if prompt_lower.contains("fail") || prompt_lower.contains("malformed") {
            return Err(OrchestratorError::PlanningFailed {
                message: format!("Simulated planner error for task '{}'", request.task_id),
            });
        }

        if prompt_lower.contains("workflow") || prompt_lower.contains("multi-step") {
            // Multi-step workflow (e.g. summarize webpage -> save note)
            steps.push(ExecutionStep {
                step_id: format!("{plan_id}-step-1"),
                description: "Assemble context and query memory".to_string(),
                action_type: "memory_query".to_string(),
                input: request.prompt.clone(),
                output: None,
                state: ExecutionState::Planning,
                retry_count: 0,
                error_message: None,
            });
            steps.push(ExecutionStep {
                step_id: format!("{plan_id}-step-2"),
                description: "Perform model reasoning generation".to_string(),
                action_type: "model_reasoning".to_string(),
                input: request.prompt.clone(),
                output: None,
                state: ExecutionState::Planning,
                retry_count: 0,
                error_message: None,
            });
        } else if prompt_lower.contains("tool")
            || prompt_lower.contains("search")
            || prompt_lower.contains("call")
        {
            steps.push(ExecutionStep {
                step_id: format!("{plan_id}-step-1"),
                description: "Execute requested tool call via ToolRegistry".to_string(),
                action_type: "tool_call".to_string(),
                input: request.prompt.clone(),
                output: None,
                state: ExecutionState::Planning,
                retry_count: 0,
                error_message: None,
            });
        } else {
            // Single reasoning step
            steps.push(ExecutionStep {
                step_id: format!("{plan_id}-step-1"),
                description: "Model reasoning inference step".to_string(),
                action_type: "model_reasoning".to_string(),
                input: request.prompt.clone(),
                output: None,
                state: ExecutionState::Planning,
                retry_count: 0,
                error_message: None,
            });
        }

        // Truncate steps if exceeding allowed ceiling
        if steps.len() > self.max_allowed_steps {
            steps.truncate(self.max_allowed_steps);
        }

        Ok(ExecutionPlan { plan_id, steps })
    }
}
