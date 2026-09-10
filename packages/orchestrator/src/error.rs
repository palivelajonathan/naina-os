//! Error types for the NAINA OS orchestrator package.

use std::fmt;

/// Result type used throughout the orchestrator package.
pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Errors produced by the orchestrator package.
#[derive(Debug)]
pub enum OrchestratorError {
    TaskFailed {
        task_id: String,
        reason: String,
    },
    MaxStepsExceeded {
        task_id: String,
        max_steps: usize,
    },
    Timeout {
        task_id: String,
        duration_seconds: u64,
    },
    PlanningFailed {
        message: String,
    },
    ExecutionFailed {
        step_id: String,
        message: String,
    },
    EmptyPlan {
        task_id: String,
    },
    Runtime(runtime::RuntimeError),
    Memory(memory::MemoryError),
    ContextEngine(context_engine::ContextEngineError),
    ModelRuntime(model_runtime::ModelRuntimeError),
    ToolRegistry(tool_registry::ToolError),
    Configuration(configuration::ConfigError),
    LockError {
        message: String,
    },
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrchestratorError::TaskFailed { task_id, reason } => {
                write!(f, "Task '{task_id}' failed: {reason}")
            }
            OrchestratorError::MaxStepsExceeded { task_id, max_steps } => {
                write!(
                    f,
                    "Task '{task_id}' exceeded maximum allowed steps ({max_steps})"
                )
            }
            OrchestratorError::Timeout {
                task_id,
                duration_seconds,
            } => {
                write!(
                    f,
                    "Task '{task_id}' timed out after {duration_seconds} seconds"
                )
            }
            OrchestratorError::PlanningFailed { message } => {
                write!(f, "CARF planning failed: {message}")
            }
            OrchestratorError::ExecutionFailed { step_id, message } => {
                write!(f, "Execution step '{step_id}' failed: {message}")
            }
            OrchestratorError::EmptyPlan { task_id } => {
                write!(f, "Generated plan for task '{task_id}' was empty")
            }
            OrchestratorError::Runtime(err) => write!(f, "Runtime error: {err}"),
            OrchestratorError::Memory(err) => write!(f, "Memory error: {err}"),
            OrchestratorError::ContextEngine(err) => write!(f, "Context engine error: {err}"),
            OrchestratorError::ModelRuntime(err) => write!(f, "Model runtime error: {err}"),
            OrchestratorError::ToolRegistry(err) => write!(f, "Tool registry error: {err}"),
            OrchestratorError::Configuration(err) => write!(f, "Configuration error: {err}"),
            OrchestratorError::LockError { message } => {
                write!(f, "Lock acquisition error: {message}")
            }
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl From<runtime::RuntimeError> for OrchestratorError {
    fn from(err: runtime::RuntimeError) -> Self {
        OrchestratorError::Runtime(err)
    }
}

impl From<memory::MemoryError> for OrchestratorError {
    fn from(err: memory::MemoryError) -> Self {
        OrchestratorError::Memory(err)
    }
}

impl From<context_engine::ContextEngineError> for OrchestratorError {
    fn from(err: context_engine::ContextEngineError) -> Self {
        OrchestratorError::ContextEngine(err)
    }
}

impl From<model_runtime::ModelRuntimeError> for OrchestratorError {
    fn from(err: model_runtime::ModelRuntimeError) -> Self {
        OrchestratorError::ModelRuntime(err)
    }
}

impl From<tool_registry::ToolError> for OrchestratorError {
    fn from(err: tool_registry::ToolError) -> Self {
        OrchestratorError::ToolRegistry(err)
    }
}

impl From<configuration::ConfigError> for OrchestratorError {
    fn from(err: configuration::ConfigError) -> Self {
        OrchestratorError::Configuration(err)
    }
}
