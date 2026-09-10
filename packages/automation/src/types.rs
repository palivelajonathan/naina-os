//! Core data structures and state definitions for workflow automation.

/// Specification payload defining a multi-step automation workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkflowSpec {
    /// Unique identifier for the workflow.
    pub workflow_id: String,
    /// Human-readable name of the workflow.
    pub name: String,
    /// Sequential collection of steps composing the workflow.
    pub steps: Vec<WorkflowStep>,
}

/// Individual executable step within an automation workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkflowStep {
    /// Unique identifier for the step within the workflow.
    pub step_id: String,
    /// Identifier of the action type to invoke.
    pub action_type: String,
    /// Action parameters or input payload string.
    pub payload: String,
    /// Optional conditional execution expression string.
    pub condition: Option<String>,
    /// Flag indicating whether transient step failure warrants automatic retry.
    pub on_failure_retry: bool,
}

/// Telemetry and output result from an individual step execution.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StepResult {
    /// Identifier of the executed step.
    pub step_id: String,
    /// Status description of step execution (e.g. "Completed", "Skipped", "Failed").
    pub status: String,
    /// Output payload produced by the step, if successful.
    pub output: Option<String>,
    /// Execution latency in milliseconds.
    pub elapsed_ms: u64,
    /// Error message string, if the step failed.
    pub error: Option<String>,
}

/// Aggregate output and execution telemetry report for a complete workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkflowExecutionResult {
    /// Identifier of the executed workflow.
    pub workflow_id: String,
    /// Overall execution status (e.g. "Completed", "Failed", "Cancelled", "TimedOut").
    pub status: String,
    /// Collection of step results executed during the workflow.
    pub step_results: Vec<StepResult>,
    /// Total workflow execution time in milliseconds.
    pub total_elapsed_ms: u64,
}

/// Operational state machine states for [`AutomationEngine`](crate::automation_engine::AutomationEngine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutomationState {
    /// Engine is idle and ready to accept new workflows.
    Idle,
    /// Validating workflow specification schema and step counts.
    Validating,
    /// Actively executing workflow steps.
    Executing,
    /// Evaluating step telemetry and conditional branching logic.
    Evaluating,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed due to non-retryable error or exhausted step retries.
    Failed,
    /// Workflow was cancelled by caller.
    Cancelled,
    /// Workflow or step execution timed out.
    TimedOut,
}
