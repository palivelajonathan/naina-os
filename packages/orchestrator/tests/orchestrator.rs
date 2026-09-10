use context_engine::{ContextEngine, ContextEngineConfig, Role};
use memory::{MemoryConfig, MemoryStore};
use model_providers::MockModelProvider;
use model_runtime::{ModelProvider, ModelRuntime, ModelRuntimeConfig};
use orchestrator::{
    DefaultPlanner, ExecutionPlan, ExecutionState, ExecutionStep, Orchestrator, OrchestratorConfig,
    OrchestratorError, Planner, TaskRequest,
};
use runtime::{Runtime, RuntimeConfig};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tool_registry::{ToolRegistry, ToolRegistryConfig};

fn setup_test_orchestrator() -> (Orchestrator, Arc<ModelRuntime>, Arc<ContextEngine>) {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    let context_engine = Arc::new(ContextEngine::new(
        ContextEngineConfig::default(),
        Arc::clone(&memory),
    ));
    let model_runtime = Arc::new(ModelRuntime::new(ModelRuntimeConfig::default()));
    let mock_provider: Arc<dyn ModelProvider> = Arc::new(MockModelProvider::new());
    model_runtime.register_provider(mock_provider).unwrap();
    model_runtime.load_model("mock", "qwen-7b-gguf").unwrap();

    let tool_registry = Arc::new(ToolRegistry::new(ToolRegistryConfig, Arc::clone(&runtime)));

    let orch = Orchestrator::new(
        OrchestratorConfig::default(),
        runtime,
        memory,
        Arc::clone(&context_engine),
        Arc::clone(&model_runtime),
        tool_registry,
    );

    (orch, model_runtime, context_engine)
}

#[derive(Debug)]
struct FixedMultiStepPlanner {
    step_count: usize,
}

impl Planner for FixedMultiStepPlanner {
    fn plan_name(&self) -> &str {
        "fixed-multi-step-planner"
    }

    fn create_plan(&self, request: &TaskRequest) -> orchestrator::Result<ExecutionPlan> {
        let mut steps = Vec::new();
        for i in 1..=self.step_count {
            steps.push(ExecutionStep {
                step_id: format!("{}-step-{i}", request.task_id),
                description: format!("Step {i} reasoning"),
                action_type: "model_reasoning".to_string(),
                input: request.prompt.clone(),
                output: None,
                state: ExecutionState::Planning,
                retry_count: 0,
                error_message: None,
            });
        }
        Ok(ExecutionPlan {
            plan_id: format!("plan-{}", request.task_id),
            steps,
        })
    }
}

#[test]
fn test_01_construction_and_configuration() {
    let (orch, _, _) = setup_test_orchestrator();
    assert_eq!(orch.config().max_steps, 10);
    assert_eq!(orch.config().timeout_seconds, 60);
    assert_eq!(orch.config().retry_limit, 2);
    assert_eq!(orch.planner().plan_name(), "carf-default-planner");
}

#[test]
fn test_02_task_request_and_response_creation() {
    let req = TaskRequest {
        task_id: "task-01".to_string(),
        prompt: "Summarize kernel".to_string(),
        conversation_id: None,
    };
    assert_eq!(req.task_id, "task-01");
}

#[test]
fn test_03_carf_planner_plan_creation() {
    let planner = DefaultPlanner::new();
    let req = TaskRequest {
        task_id: "plan-task".to_string(),
        prompt: "Perform single reasoning task".to_string(),
        conversation_id: None,
    };

    let plan = planner.create_plan(&req).unwrap();
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].action_type, "model_reasoning");
}

#[test]
fn test_04_successful_single_step_execution() {
    let (orch, _, _) = setup_test_orchestrator();
    let req = TaskRequest {
        task_id: "succ-task-1".to_string(),
        prompt: "Summarize system status".to_string(),
        conversation_id: None,
    };

    let res = orch.execute_task(req).unwrap();
    assert_eq!(res.status, ExecutionState::Completed);
    assert_eq!(res.steps_executed, 1);
    assert!(!res.output.is_empty());
}

#[test]
fn test_05_successful_multi_step_workflow_execution() {
    let (orch, _, _) = setup_test_orchestrator();
    let req = TaskRequest {
        task_id: "multi-task-1".to_string(),
        prompt: "Execute multi-step workflow task".to_string(),
        conversation_id: None,
    };

    let res = orch.execute_task(req).unwrap();
    assert_eq!(res.status, ExecutionState::Completed);
    assert_eq!(res.steps_executed, 2);
}

#[test]
fn test_06_tool_registry_routing_execution() {
    let (orch, _, _) = setup_test_orchestrator();
    let req = TaskRequest {
        task_id: "tool-task-1".to_string(),
        prompt: "Execute tool call for system inspection".to_string(),
        conversation_id: None,
    };

    let res = orch.execute_task(req).unwrap();
    assert_eq!(res.status, ExecutionState::Completed);
    assert_eq!(res.steps_executed, 1);
}

#[test]
fn test_07_empty_prompt_planning_failure() {
    let (orch, _, _) = setup_test_orchestrator();
    let req = TaskRequest {
        task_id: "empty-prompt-task".to_string(),
        prompt: "   ".to_string(),
        conversation_id: None,
    };

    let err = orch.execute_task(req).unwrap_err();
    assert!(matches!(err, OrchestratorError::PlanningFailed { .. }));
}

#[test]
fn test_08_empty_task_id_failure() {
    let (orch, _, _) = setup_test_orchestrator();
    let req = TaskRequest {
        task_id: "".to_string(),
        prompt: "Valid prompt".to_string(),
        conversation_id: None,
    };

    let err = orch.execute_task(req).unwrap_err();
    assert!(matches!(err, OrchestratorError::TaskFailed { .. }));
}

#[test]
fn test_09_max_steps_limit_enforcement() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    let context_engine = Arc::new(ContextEngine::new(
        ContextEngineConfig::default(),
        Arc::clone(&memory),
    ));
    let model_runtime = Arc::new(ModelRuntime::new(ModelRuntimeConfig::default()));
    let tool_registry = Arc::new(ToolRegistry::new(ToolRegistryConfig, Arc::clone(&runtime)));

    // Orchestrator configured with max_steps = 2, but planner generates 4 steps
    let orch = Orchestrator::with_planner(
        OrchestratorConfig {
            max_steps: 2,
            timeout_seconds: 60,
            retry_limit: 2,
        },
        Arc::new(FixedMultiStepPlanner { step_count: 4 }),
        runtime,
        memory,
        context_engine,
        model_runtime,
        tool_registry,
    );

    let req = TaskRequest {
        task_id: "max-steps-task".to_string(),
        prompt: "Run oversized task".to_string(),
        conversation_id: None,
    };

    let err = orch.execute_task(req).unwrap_err();
    assert!(matches!(
        err,
        OrchestratorError::MaxStepsExceeded { max_steps: 2, .. }
    ));
}

#[test]
fn test_10_context_preservation_and_five_user_turn_retention() {
    let (orch, _, context_engine) = setup_test_orchestrator();
    let cid = context_engine.create_conversation().unwrap();

    for i in 1..=5 {
        let req = TaskRequest {
            task_id: format!("turn-task-{i}"),
            prompt: format!("User turn prompt number {i}"),
            conversation_id: Some(cid),
        };
        let res = orch.execute_task(req).unwrap();
        assert_eq!(res.status, ExecutionState::Completed);
    }

    let history = context_engine.get_history(cid).unwrap();
    // 5 User turns + 5 Assistant turns = 10 turns
    assert_eq!(history.len(), 10);

    let user_turns = history.iter().filter(|t| t.role == Role::User).count();
    assert_eq!(user_turns, 5);
}

#[test]
fn test_11_concurrent_task_orchestration() {
    let (orch, _, _) = setup_test_orchestrator();
    let orch_arc = Arc::new(orch);

    let mut handles = Vec::new();
    for i in 0..8 {
        let orch_ref = Arc::clone(&orch_arc);
        handles.push(thread::spawn(move || {
            let req = TaskRequest {
                task_id: format!("conc-task-{i}"),
                prompt: format!("Concurrent execution prompt {i}"),
                conversation_id: None,
            };
            let res = orch_ref.execute_task(req).unwrap();
            assert_eq!(res.status, ExecutionState::Completed);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_12_performance_dispatch_overhead_under_10ms_target() {
    let (orch, _, _) = setup_test_orchestrator();
    let req = TaskRequest {
        task_id: "perf-task-1".to_string(),
        prompt: "Benchmark orchestrator dispatch latency".to_string(),
        conversation_id: None,
    };

    let start = Instant::now();
    let res = orch.execute_task(req).unwrap();
    let elapsed = start.elapsed();

    println!("Orchestrator dispatch latency duration: {:?}", elapsed);
    assert_eq!(res.status, ExecutionState::Completed);
    assert!(
        elapsed.as_millis() < 10,
        "Orchestrator dispatch overhead exceeded 10ms target: {}ms",
        elapsed.as_millis()
    );
}
