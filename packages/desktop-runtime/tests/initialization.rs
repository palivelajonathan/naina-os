use desktop_runtime::{DesktopRuntime, DesktopRuntimeConfig, DesktopState};
use services::ServiceRegistry;
use std::sync::Arc;

#[test]
fn test_desktop_runtime_instantiation_and_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = DesktopRuntimeConfig::default();
    let dr = DesktopRuntime::new(config, runtime, services);

    assert_eq!(dr.config().max_search_depth, 5);
    assert_eq!(dr.config().max_element_count, 100);
    assert_eq!(dr.config().inspection_timeout_ms, 200);
    assert_eq!(dr.config().latency_target_ms, 500);
    assert_eq!(dr.config().ram_limit_mb, 200);
    assert_eq!(dr.state(), DesktopState::Idle);
}
