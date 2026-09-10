use automation::{
    AutomationConfig, AutomationEngine, AutomationError, AutomationState, MockWorkflowStepRunner,
    WorkflowSpec, WorkflowStep,
};
use services::ServiceRegistry;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn setup_test_automation_engine() -> AutomationEngine {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = AutomationConfig::default();
    AutomationEngine::new(config, runtime, services)
}

#[test]
fn test_01_construction_and_defaults() {
    let engine = setup_test_automation_engine();
    assert!(engine.config().enabled);
    assert_eq!(engine.config().max_steps_per_workflow, 20);
    assert_eq!(engine.state(), AutomationState::Idle);
}

#[test]
fn test_02_custom_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = AutomationConfig {
        max_steps_per_workflow: 10,
        step_timeout_ms: 1_000,
        ..Default::default()
    };
    let engine = AutomationEngine::new(config, runtime, services);

    assert_eq!(engine.config().max_steps_per_workflow, 10);
    assert_eq!(engine.config().step_timeout_ms, 1_000);
}

#[test]
fn test_03_empty_workflow_rejection() {
    let engine = setup_test_automation_engine();
    let spec = WorkflowSpec {
        workflow_id: "wf-1".to_string(),
        name: "Empty Workflow".to_string(),
        steps: vec![],
    };

    let err = engine.execute_workflow(spec).unwrap_err();
    assert!(matches!(
        err,
        AutomationError::WorkflowValidationFailed { .. }
    ));
    assert_eq!(engine.state(), AutomationState::Idle);
}

#[test]
fn test_04_max_step_boundary_and_rejection() {
    let engine = setup_test_automation_engine();

    // 20 steps (valid boundary)
    let valid_steps: Vec<WorkflowStep> = (1..=20)
        .map(|i| WorkflowStep {
            step_id: format!("step-{i}"),
            action_type: "log".to_string(),
            payload: "hello".to_string(),
            condition: None,
            on_failure_retry: false,
        })
        .collect();

    let valid_spec = WorkflowSpec {
        workflow_id: "wf-valid".to_string(),
        name: "20 Step Workflow".to_string(),
        steps: valid_steps,
    };
    let res = engine.execute_workflow(valid_spec).unwrap();
    assert_eq!(res.status, "Completed");
    assert_eq!(res.step_results.len(), 20);

    // 21 steps (>20 rejection, no silent truncation!)
    let invalid_steps: Vec<WorkflowStep> = (1..=21)
        .map(|i| WorkflowStep {
            step_id: format!("step-{i}"),
            action_type: "log".to_string(),
            payload: "hello".to_string(),
            condition: None,
            on_failure_retry: false,
        })
        .collect();

    let invalid_spec = WorkflowSpec {
        workflow_id: "wf-invalid".to_string(),
        name: "21 Step Workflow".to_string(),
        steps: invalid_steps,
    };

    let err = engine.execute_workflow(invalid_spec).unwrap_err();
    assert!(matches!(
        err,
        AutomationError::WorkflowValidationFailed { .. }
    ));
}

#[test]
fn test_05_sequential_execution_and_telemetry() {
    let engine = setup_test_automation_engine();
    let spec = WorkflowSpec {
        workflow_id: "wf-seq".to_string(),
        name: "Sequential Workflow".to_string(),
        steps: vec![
            WorkflowStep {
                step_id: "s1".to_string(),
                action_type: "read_webpage".to_string(),
                payload: "https://naina.ai".to_string(),
                condition: None,
                on_failure_retry: false,
            },
            WorkflowStep {
                step_id: "s2".to_string(),
                action_type: "save_note".to_string(),
                payload: "Obsidian note content".to_string(),
                condition: None,
                on_failure_retry: false,
            },
        ],
    };

    let res = engine.execute_workflow(spec).unwrap();
    assert_eq!(res.status, "Completed");
    assert_eq!(res.step_results.len(), 2);
    assert_eq!(res.step_results[0].status, "Completed");
    assert_eq!(res.step_results[1].status, "Completed");
    assert!(res.step_results[0].output.is_some());
}

#[test]
fn test_06_conditional_branching_true_and_false() {
    let engine = setup_test_automation_engine();
    let spec = WorkflowSpec {
        workflow_id: "wf-cond".to_string(),
        name: "Conditional Workflow".to_string(),
        steps: vec![
            WorkflowStep {
                step_id: "s1".to_string(),
                action_type: "step_true".to_string(),
                payload: "data".to_string(),
                condition: Some("true".to_string()),
                on_failure_retry: false,
            },
            WorkflowStep {
                step_id: "s2".to_string(),
                action_type: "step_false".to_string(),
                payload: "data".to_string(),
                condition: Some("false".to_string()),
                on_failure_retry: false,
            },
        ],
    };

    let res = engine.execute_workflow(spec).unwrap();
    assert_eq!(res.status, "Completed");
    assert_eq!(res.step_results[0].status, "Completed");
    assert_eq!(res.step_results[1].status, "Skipped");
}

#[test]
fn test_07_retry_behavior_on_step_failure() {
    let engine = setup_test_automation_engine();
    let runner = Arc::new(MockWorkflowStepRunner {
        should_fail_step_id: Some("failing_step".to_string()),
        should_deny_capability_step_id: None,
    });
    engine.register_runner(runner);

    let spec = WorkflowSpec {
        workflow_id: "wf-retry".to_string(),
        name: "Retry Workflow".to_string(),
        steps: vec![WorkflowStep {
            step_id: "failing_step".to_string(),
            action_type: "unstable_action".to_string(),
            payload: "input".to_string(),
            condition: None,
            on_failure_retry: true,
        }],
    };

    let res = engine.execute_workflow(spec).unwrap();
    assert_eq!(res.status, "Failed");
    assert_eq!(res.step_results[0].status, "Failed");
}

#[test]
fn test_08_capability_denial_non_retryable() {
    let engine = setup_test_automation_engine();
    let runner = Arc::new(MockWorkflowStepRunner {
        should_fail_step_id: None,
        should_deny_capability_step_id: Some("forbidden_step".to_string()),
    });
    engine.register_runner(runner);

    let spec = WorkflowSpec {
        workflow_id: "wf-cap".to_string(),
        name: "Capability Denial Workflow".to_string(),
        steps: vec![WorkflowStep {
            step_id: "forbidden_step".to_string(),
            action_type: "restricted_action".to_string(),
            payload: "input".to_string(),
            condition: None,
            on_failure_retry: true,
        }],
    };

    let res = engine.execute_workflow(spec).unwrap();
    assert_eq!(res.status, "Failed");
    assert!(res.step_results[0]
        .error
        .as_ref()
        .unwrap()
        .contains("Capability denied"));
}

#[test]
fn test_09_cancellation_protocol() {
    let engine = setup_test_automation_engine();
    engine.cancel();

    let spec = WorkflowSpec {
        workflow_id: "wf-cancel".to_string(),
        name: "Cancelled Workflow".to_string(),
        steps: vec![WorkflowStep {
            step_id: "s1".to_string(),
            action_type: "action".to_string(),
            payload: "payload".to_string(),
            condition: None,
            on_failure_retry: false,
        }],
    };

    let err = engine.execute_workflow(spec).unwrap_err();
    assert!(matches!(err, AutomationError::WorkflowCancelled { .. }));

    engine.reset_state();
    assert_eq!(engine.state(), AutomationState::Idle);
}

#[test]
fn test_10_state_transition_validation() {
    let engine = setup_test_automation_engine();
    assert_eq!(engine.state(), AutomationState::Idle);

    engine.set_state(AutomationState::Validating).unwrap();
    assert_eq!(engine.state(), AutomationState::Validating);

    // Invalid state transition from Validating directly to Completed
    let err = engine.set_state(AutomationState::Completed).unwrap_err();
    assert!(matches!(
        err,
        AutomationError::WorkflowValidationFailed { .. }
    ));

    engine.reset_state();
    assert_eq!(engine.state(), AutomationState::Idle);
}

#[test]
fn test_11_concurrent_engine_access() {
    let engine = setup_test_automation_engine();
    let engine_arc = Arc::new(engine);

    let mut handles = Vec::new();
    for i in 0..4 {
        let eng = Arc::clone(&engine_arc);
        handles.push(thread::spawn(move || {
            let spec = WorkflowSpec {
                workflow_id: format!("wf-concurrent-{i}"),
                name: "Concurrent Workflow".to_string(),
                steps: vec![WorkflowStep {
                    step_id: format!("s-{i}"),
                    action_type: "action".to_string(),
                    payload: "payload".to_string(),
                    condition: None,
                    on_failure_retry: false,
                }],
            };
            let res = eng.execute_workflow(spec).unwrap();
            assert_eq!(res.status, "Completed");
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_12_performance_execution_latency_check() {
    let engine = setup_test_automation_engine();
    let spec = WorkflowSpec {
        workflow_id: "wf-perf".to_string(),
        name: "Perf Workflow".to_string(),
        steps: vec![WorkflowStep {
            step_id: "s1".to_string(),
            action_type: "fast_action".to_string(),
            payload: "input".to_string(),
            condition: None,
            on_failure_retry: false,
        }],
    };

    let start = Instant::now();
    let res = engine.execute_workflow(spec).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(res.status, "Completed");
    assert!(
        elapsed.as_millis() < 500,
        "Execution latency exceeded 500ms"
    );
}
