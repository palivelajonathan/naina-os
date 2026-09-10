use sdk::{SDKBuilder, SDKConfig, SDKState};
use services::ServiceRegistry;
use std::sync::Arc;

#[test]
fn test_sdk_instantiation_and_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = SDKConfig::default();
    assert_eq!(config.system_name, "NAINA OS");
    assert!(config.enable_logging);
    assert!(config.auto_start_runtime);
    assert_eq!(config.session_timeout_ms, 3_600_000);

    let facade = SDKBuilder::new()
        .with_config(config)
        .with_runtime(runtime)
        .with_services(services)
        .build()
        .unwrap();

    assert_eq!(facade.state(), SDKState::Ready);
}
