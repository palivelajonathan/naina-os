//! NAINA OS automation package.

pub mod automation_engine;
pub mod config;
pub mod error;
pub mod traits;
pub mod types;

pub use automation_engine::AutomationEngine;
pub use config::AutomationConfig;
pub use error::{AutomationError, Result};
pub use traits::{MockWorkflowStepRunner, WorkflowStepRunner};
pub use types::{AutomationState, StepResult, WorkflowExecutionResult, WorkflowSpec, WorkflowStep};
