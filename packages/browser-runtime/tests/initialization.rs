use browser_runtime::{BrowserRuntime, BrowserRuntimeConfig, BrowserState};
use services::ServiceRegistry;
use std::sync::Arc;

#[test]
fn test_browser_runtime_instantiation_and_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = BrowserRuntimeConfig::default();
    let br = BrowserRuntime::new(config, runtime, services);

    assert!(br.config().headless);
    assert_eq!(br.config().remote_debugging_port, 9222);
    assert_eq!(br.config().navigation_timeout_ms, 10_000);
    assert_eq!(br.config().latency_target_ms, 1_000);
    assert_eq!(br.state(), BrowserState::Idle);
}
