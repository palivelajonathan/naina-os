use orchestrator::{Orchestrator, OrchestratorConfig};
use std::sync::Arc;

#[test]
fn test_orchestrator_initialization_and_config() {
    let config = OrchestratorConfig::default();
    assert_eq!(config.max_steps, 10);
    assert_eq!(config.timeout_seconds, 60);
    assert_eq!(config.retry_limit, 2);

    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let memory = Arc::new(memory::MemoryStore::new(memory::MemoryConfig::default()));
    let context_engine = Arc::new(context_engine::ContextEngine::new(
        context_engine::ContextEngineConfig::default(),
        Arc::clone(&memory),
    ));
    let model_runtime = Arc::new(model_runtime::ModelRuntime::new(
        model_runtime::ModelRuntimeConfig::default(),
    ));
    let tool_registry = Arc::new(tool_registry::ToolRegistry::new(
        tool_registry::ToolRegistryConfig,
        Arc::clone(&runtime),
    ));

    let orch = Orchestrator::new(
        config.clone(),
        runtime,
        memory,
        context_engine,
        model_runtime,
        tool_registry,
    );

    assert_eq!(orch.config(), &config);
}
