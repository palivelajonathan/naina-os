//! Data types for task requests, execution plans, and responses.

use context_engine::ConversationId;

/// Input request payload submitted to the orchestrator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRequest {
    pub task_id: String,
    pub prompt: String,
    pub conversation_id: Option<ConversationId>,
}

/// Execution state of a task or execution step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionState {
    Planning,
    ExecutingStep,
    Evaluating,
    Completed,
    Failed,
}

/// Individual step within an execution plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionStep {
    pub step_id: String,
    pub description: String,
    pub action_type: String, // "model_reasoning", "tool_call", "memory_query"
    pub input: String,
    pub output: Option<String>,
    pub state: ExecutionState,
    pub retry_count: usize,
    pub error_message: Option<String>,
}

/// Multi-step plan produced by the CARF planner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub steps: Vec<ExecutionStep>,
}

/// Output response returned upon task execution completion or failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: ExecutionState,
    pub output: String,
    pub steps_executed: usize,
}
