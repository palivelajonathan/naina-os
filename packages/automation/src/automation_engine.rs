//! Primary AutomationEngine supervisor implementation for NAINA OS.

use crate::config::AutomationConfig;
use crate::error::{AutomationError, Result};
use crate::traits::{MockWorkflowStepRunner, WorkflowStepRunner};
use crate::types::{AutomationState, StepResult, WorkflowExecutionResult, WorkflowSpec};
use logging::{LogLevel, Logger, LoggerConfig};
use runtime::Runtime;
use services::ServiceRegistry;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// Primary workflow automation engine supervisor for NAINA OS.
pub struct AutomationEngine {
    config: AutomationConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    state: RwLock<AutomationState>,
    runner: RwLock<Arc<dyn WorkflowStepRunner>>,
    cancel_flag: Arc<AtomicBool>,
    logger: Mutex<Logger>,
}

unsafe impl Send for AutomationEngine {}
unsafe impl Sync for AutomationEngine {}

impl fmt::Debug for AutomationEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutomationEngine")
            .field("config", &self.config)
            .field("runtime", &self.runtime)
            .field("services", &self.services)
            .field("state", &self.state)
            .field("cancel_flag", &self.cancel_flag)
            .finish()
    }
}

impl AutomationEngine {
    /// Constructs a new [`AutomationEngine`] instance.
    pub fn new(
        config: AutomationConfig,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let default_runner: Arc<dyn WorkflowStepRunner> = Arc::new(MockWorkflowStepRunner::new());
        let logger = Logger::new(LoggerConfig::default())
            .unwrap()
            .with_component("automation");

        Self {
            config,
            runtime,
            services,
            state: RwLock::new(AutomationState::Idle),
            runner: RwLock::new(default_runner),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            logger: Mutex::new(logger),
        }
    }

    /// Constructs an [`AutomationEngine`] with default configuration.
    pub fn with_default_config(runtime: Arc<Runtime>, services: Arc<ServiceRegistry>) -> Self {
        Self::new(AutomationConfig::default(), runtime, services)
    }

    /// Constructs an [`AutomationEngine`] from root [`configuration::Config`].
    pub fn from_root_config(
        root_config: &configuration::Config,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let config = AutomationConfig {
            enabled: root_config.automation.enabled,
            ..Default::default()
        };
        Self::new(config, runtime, services)
    }

    /// Registers a custom step runner provider.
    pub fn register_runner(&self, runner: Arc<dyn WorkflowStepRunner>) {
        if let Ok(mut lock) = self.runner.write() {
            *lock = runner;
        }
    }

    /// Returns the active configuration.
    pub fn config(&self) -> &AutomationConfig {
        &self.config
    }

    /// Returns the current automation state.
    pub fn state(&self) -> AutomationState {
        self.state
            .read()
            .map(|s| *s)
            .unwrap_or(AutomationState::Failed)
    }

    /// Sets the engine state safely according to valid transition paths.
    pub fn set_state(&self, new_state: AutomationState) -> Result<()> {
        let mut lock = self.state.write().map_err(|_| AutomationError::LockError {
            message: "Failed to acquire state write lock".to_string(),
        })?;

        let current = *lock;
        let valid = match (current, new_state) {
            (AutomationState::Idle, AutomationState::Validating) => true,
            (AutomationState::Validating, AutomationState::Executing) => true,
            (AutomationState::Executing, AutomationState::Evaluating) => true,
            (AutomationState::Evaluating, AutomationState::Executing) => true,
            (AutomationState::Evaluating, AutomationState::Completed) => true,
            (
                AutomationState::Validating
                | AutomationState::Executing
                | AutomationState::Evaluating,
                AutomationState::Failed,
            ) => true,
            (
                AutomationState::Validating
                | AutomationState::Executing
                | AutomationState::Evaluating,
                AutomationState::Cancelled,
            ) => true,
            (
                AutomationState::Validating
                | AutomationState::Executing
                | AutomationState::Evaluating,
                AutomationState::TimedOut,
            ) => true,
            (
                AutomationState::Validating
                | AutomationState::Executing
                | AutomationState::Evaluating
                | AutomationState::Completed
                | AutomationState::Failed
                | AutomationState::Cancelled
                | AutomationState::TimedOut,
                AutomationState::Idle,
            ) => true,
            (a, b) if a == b => true,
            _ => false,
        };

        if !valid {
            return Err(AutomationError::WorkflowValidationFailed {
                message: format!("Invalid state transition from {current:?} to {new_state:?}"),
            });
        }

        *lock = new_state;
        Ok(())
    }

    /// Returns a reference to the inner runtime handle.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Returns a reference to the inner service registry handle.
    pub fn services(&self) -> &Arc<ServiceRegistry> {
        &self.services
    }

    /// Cancels any active or pending workflow execution.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Resets the engine state back to `Idle` after error or cancellation.
    pub fn reset_state(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
        let mut lock = self.state.write().unwrap();
        *lock = AutomationState::Idle;
    }

    /// Executes a multi-step workflow specification deterministically.
    pub fn execute_workflow(&self, workflow: WorkflowSpec) -> Result<WorkflowExecutionResult> {
        let workflow_start = Instant::now();

        if let Ok(logger) = self.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                format!("Starting execution of workflow '{}'", workflow.workflow_id),
            );
        }

        // 1. Check enabled state
        if !self.config.enabled {
            return Err(AutomationError::WorkflowValidationFailed {
                message: "Workflow automation engine is disabled in configuration".to_string(),
            });
        }

        // 2. Check cancellation
        if self.cancel_flag.load(Ordering::SeqCst) {
            let _ = self.set_state(AutomationState::Cancelled);
            let _ = self.set_state(AutomationState::Idle);
            return Err(AutomationError::WorkflowCancelled {
                workflow_id: workflow.workflow_id,
            });
        }

        // 3. Schema validation
        self.set_state(AutomationState::Validating)?;
        if workflow.workflow_id.trim().is_empty() {
            let _ = self.set_state(AutomationState::Idle);
            return Err(AutomationError::WorkflowValidationFailed {
                message: "Workflow ID cannot be empty".to_string(),
            });
        }
        if workflow.steps.is_empty() {
            let _ = self.set_state(AutomationState::Idle);
            return Err(AutomationError::WorkflowValidationFailed {
                message: "Workflow must contain at least one step".to_string(),
            });
        }

        // 4. Step count boundary check (never truncate!)
        if workflow.steps.len() > self.config.max_steps_per_workflow {
            let _ = self.set_state(AutomationState::Idle);
            return Err(AutomationError::WorkflowValidationFailed {
                message: format!(
                    "Workflow step count ({}) exceeds maximum limit ({})",
                    workflow.steps.len(),
                    self.config.max_steps_per_workflow
                ),
            });
        }

        self.set_state(AutomationState::Executing)?;
        let mut step_results = Vec::new();
        let mut workflow_failed = false;

        let runner = {
            let lock = self.runner.read().map_err(|_| AutomationError::LockError {
                message: "Failed to acquire runner read lock".to_string(),
            })?;
            Arc::clone(&*lock)
        };

        for step in &workflow.steps {
            // Check cancellation
            if self.cancel_flag.load(Ordering::SeqCst) {
                let _ = self.set_state(AutomationState::Cancelled);
                let _ = self.set_state(AutomationState::Idle);
                return Err(AutomationError::WorkflowCancelled {
                    workflow_id: workflow.workflow_id,
                });
            }

            // Check total workflow timeout
            if workflow_start.elapsed().as_millis() as u64 > self.config.workflow_timeout_ms {
                let _ = self.set_state(AutomationState::TimedOut);
                let _ = self.set_state(AutomationState::Idle);
                return Err(AutomationError::WorkflowTimeout {
                    workflow_id: workflow.workflow_id,
                });
            }

            // Evaluate conditional branching
            if let Some(ref cond) = step.condition {
                let cond_lower = cond.trim().to_lowercase();
                if cond_lower == "false" || cond_lower == "0" {
                    // Skip step
                    step_results.push(StepResult {
                        step_id: step.step_id.clone(),
                        status: "Skipped".to_string(),
                        output: None,
                        elapsed_ms: 0,
                        error: None,
                    });
                    continue;
                }
            }

            // Execute step with retry policy
            let step_start = Instant::now();
            let mut attempts = 0;
            let mut step_success = false;
            let mut last_error = None;
            let mut result_output = None;

            while attempts
                <= if step.on_failure_retry {
                    self.config.max_retry_attempts
                } else {
                    0
                }
            {
                attempts += 1;
                match runner.execute_step(step) {
                    Ok(res) => {
                        step_success = true;
                        result_output = res.output;
                        break;
                    }
                    Err(err) => {
                        // Non-retryable errors fail immediately
                        if matches!(err, AutomationError::CapabilityDenied { .. }) {
                            last_error = Some(err.to_string());
                            break;
                        }

                        last_error = Some(err.to_string());
                        if attempts <= self.config.max_retry_attempts && step.on_failure_retry {
                            // Exponential backoff
                            let backoff = Duration::from_millis(100 * (1 << (attempts - 1)));
                            thread::sleep(backoff);
                        }
                    }
                }
            }

            let elapsed_ms = step_start.elapsed().as_millis() as u64;

            // Check per-step timeout target
            if elapsed_ms > self.config.step_timeout_ms {
                let _ = self.set_state(AutomationState::TimedOut);
                let _ = self.set_state(AutomationState::Idle);
                return Err(AutomationError::WorkflowTimeout {
                    workflow_id: workflow.workflow_id,
                });
            }

            if step_success {
                step_results.push(StepResult {
                    step_id: step.step_id.clone(),
                    status: "Completed".to_string(),
                    output: result_output,
                    elapsed_ms,
                    error: None,
                });
            } else {
                workflow_failed = true;
                step_results.push(StepResult {
                    step_id: step.step_id.clone(),
                    status: "Failed".to_string(),
                    output: None,
                    elapsed_ms,
                    error: last_error,
                });
                break;
            }
        }

        self.set_state(AutomationState::Evaluating)?;
        let total_elapsed_ms = workflow_start.elapsed().as_millis() as u64;

        if workflow_failed {
            self.set_state(AutomationState::Failed)?;
            let _ = self.set_state(AutomationState::Idle);
            Ok(WorkflowExecutionResult {
                workflow_id: workflow.workflow_id,
                status: "Failed".to_string(),
                step_results,
                total_elapsed_ms,
            })
        } else {
            self.set_state(AutomationState::Completed)?;
            let _ = self.set_state(AutomationState::Idle);
            Ok(WorkflowExecutionResult {
                workflow_id: workflow.workflow_id,
                status: "Completed".to_string(),
                step_results,
                total_elapsed_ms,
            })
        }
    }
}
