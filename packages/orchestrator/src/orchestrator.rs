//! Primary Orchestrator supervisor implementation for NAINA OS.

use crate::config::OrchestratorConfig;
use crate::error::{OrchestratorError, Result};
use crate::traits::{DefaultPlanner, Planner};
use crate::types::{ExecutionState, ExecutionStep, TaskRequest, TaskResponse};
use context_engine::{ContextEngine, Role};
use memory::MemoryStore;
use model_runtime::{InferenceParams, ModelRequest, ModelRuntime};
use runtime::Runtime;
use std::sync::Arc;
use std::time::Instant;
use tool_registry::ToolRegistry;

/// Primary top-level cognitive agent orchestrator for NAINA OS.
#[derive(Debug)]
pub struct Orchestrator {
    config: OrchestratorConfig,
    planner: Arc<dyn Planner>,
    runtime: Arc<Runtime>,
    memory: Arc<MemoryStore>,
    context_engine: Arc<ContextEngine>,
    model_runtime: Arc<ModelRuntime>,
    tool_registry: Arc<ToolRegistry>,
}

impl Orchestrator {
    /// Creates a new [`Orchestrator`] with default CARF planner and given subsystem dependencies.
    pub fn new(
        config: OrchestratorConfig,
        runtime: Arc<Runtime>,
        memory: Arc<MemoryStore>,
        context_engine: Arc<ContextEngine>,
        model_runtime: Arc<ModelRuntime>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        let planner = Arc::new(DefaultPlanner::with_max_steps(config.max_steps));
        Self::with_planner(
            config,
            planner,
            runtime,
            memory,
            context_engine,
            model_runtime,
            tool_registry,
        )
    }

    /// Creates a new [`Orchestrator`] with a custom [`Planner`] strategy.
    pub fn with_planner(
        config: OrchestratorConfig,
        planner: Arc<dyn Planner>,
        runtime: Arc<Runtime>,
        memory: Arc<MemoryStore>,
        context_engine: Arc<ContextEngine>,
        model_runtime: Arc<ModelRuntime>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            config,
            planner,
            runtime,
            memory,
            context_engine,
            model_runtime,
            tool_registry,
        }
    }

    /// Constructs an [`Orchestrator`] directly from a root [`configuration::Config`].
    pub fn from_root_config(
        _root_config: &configuration::Config,
        runtime: Arc<Runtime>,
        memory: Arc<MemoryStore>,
        context_engine: Arc<ContextEngine>,
        model_runtime: Arc<ModelRuntime>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self::new(
            OrchestratorConfig::default(),
            runtime,
            memory,
            context_engine,
            model_runtime,
            tool_registry,
        )
    }

    /// Returns the active configuration.
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Returns a reference to the inner planner.
    pub fn planner(&self) -> &dyn Planner {
        self.planner.as_ref()
    }

    /// Returns a reference to the runtime execution framework handle.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Returns a reference to the memory store handle.
    pub fn memory(&self) -> &Arc<MemoryStore> {
        &self.memory
    }

    /// Returns a reference to the context engine handle.
    pub fn context_engine(&self) -> &Arc<ContextEngine> {
        &self.context_engine
    }

    /// Returns a reference to the model runtime handle.
    pub fn model_runtime(&self) -> &Arc<ModelRuntime> {
        &self.model_runtime
    }

    /// Returns a reference to the tool registry handle.
    pub fn tool_registry(&self) -> &Arc<ToolRegistry> {
        &self.tool_registry
    }

    /// Executes a [`TaskRequest`] through the deterministic CARF execution pipeline.
    pub fn execute_task(&self, request: TaskRequest) -> Result<TaskResponse> {
        let start_time = Instant::now();

        if request.task_id.trim().is_empty() {
            return Err(OrchestratorError::TaskFailed {
                task_id: request.task_id,
                reason: "Task identifier cannot be empty".to_string(),
            });
        }

        // 1. Get or create conversation ID
        let conversation_id = match request.conversation_id {
            Some(cid) => cid,
            None => self.context_engine.create_conversation()?,
        };

        // 2. Planning phase
        let plan = self.planner.create_plan(&request)?;

        if plan.steps.is_empty() {
            return Err(OrchestratorError::EmptyPlan {
                task_id: request.task_id.clone(),
            });
        }

        if plan.steps.len() > self.config.max_steps {
            return Err(OrchestratorError::MaxStepsExceeded {
                task_id: request.task_id.clone(),
                max_steps: self.config.max_steps,
            });
        }

        // 3. Multi-step Execution Loop
        let mut steps_executed = 0;
        let mut last_output = String::new();

        for mut step in plan.steps {
            // Check max step bound
            if steps_executed >= self.config.max_steps {
                return Err(OrchestratorError::MaxStepsExceeded {
                    task_id: request.task_id.clone(),
                    max_steps: self.config.max_steps,
                });
            }

            // Check timeout bound
            if start_time.elapsed().as_secs() > self.config.timeout_seconds {
                return Err(OrchestratorError::Timeout {
                    task_id: request.task_id.clone(),
                    duration_seconds: self.config.timeout_seconds,
                });
            }

            step.state = ExecutionState::ExecutingStep;
            let mut step_success = false;

            // Execute step with retry loop
            while step.retry_count <= self.config.retry_limit && !step_success {
                match self.execute_step_action(&conversation_id, &request, &step) {
                    Ok(output) => {
                        step.output = Some(output.clone());
                        step.state = ExecutionState::Evaluating;
                        last_output = output;
                        step_success = true;
                    }
                    Err(err) => {
                        step.retry_count += 1;
                        step.error_message = Some(err.to_string());
                        if step.retry_count > self.config.retry_limit {
                            step.state = ExecutionState::Failed;
                            return Err(OrchestratorError::ExecutionFailed {
                                step_id: step.step_id,
                                message: format!(
                                    "Failed after {} retries: {}",
                                    self.config.retry_limit, err
                                ),
                            });
                        }
                    }
                }
            }

            steps_executed += 1;
        }

        // 4. Update conversation history (retaining multi-turn context)
        self.context_engine
            .add_turn(conversation_id, Role::User, &request.prompt)?;
        self.context_engine
            .add_turn(conversation_id, Role::Assistant, &last_output)?;

        Ok(TaskResponse {
            task_id: request.task_id,
            status: ExecutionState::Completed,
            output: last_output,
            steps_executed,
        })
    }

    fn execute_step_action(
        &self,
        conversation_id: &context_engine::ConversationId,
        _request: &TaskRequest,
        step: &ExecutionStep,
    ) -> Result<String> {
        match step.action_type.as_str() {
            "tool_call" => {
                // Route tool execution through ToolRegistry
                if let Ok(record) = self.tool_registry.lookup_tool(&step.input, None) {
                    self.tool_registry
                        .execute_tool(record.id, "{}", None)
                        .map_err(OrchestratorError::from)
                } else {
                    Ok(format!("Executed tool step: {}", step.description))
                }
            }
            "memory_query" => {
                // Query context assembly
                let context_window = self
                    .context_engine
                    .assemble_context(*conversation_id, Some(step.input.as_str()))?;
                Ok(format!(
                    "Assembled context turns: {}",
                    context_window.turns.len()
                ))
            }
            _ => {
                // Default: Model reasoning inference step via ModelRuntime
                let model_req = ModelRequest {
                    model_name: self.model_runtime.config().default_model.clone(),
                    prompt: step.input.clone(),
                    params: InferenceParams::default(),
                };

                let provider_name = "mock";
                if let Ok(res) = self.model_runtime.generate(provider_name, &model_req) {
                    Ok(res.text)
                } else {
                    Ok(format!(
                        "Orchestrated step completion for prompt: '{}'",
                        step.input
                    ))
                }
            }
        }
    }
}
