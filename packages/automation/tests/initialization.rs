use automation::{AutomationConfig, AutomationEngine, AutomationState};
use services::ServiceRegistry;
use std::sync::Arc;

#[test]
fn test_automation_engine_instantiation_and_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = AutomationConfig::default();
    let engine = AutomationEngine::new(config, runtime, services);

    assert!(engine.config().enabled);
    assert_eq!(engine.config().max_steps_per_workflow, 20);
    assert_eq!(engine.config().step_timeout_ms, 5_000);
    assert_eq!(engine.config().workflow_timeout_ms, 30_000);
    assert_eq!(engine.config().max_retry_attempts, 3);
    assert_eq!(engine.config().wasm_plugin_timeout_ms, 100);
    assert_eq!(engine.state(), AutomationState::Idle);
}
