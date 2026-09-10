use kernel::{Kernel, KernelConfig};
use runtime::{Runtime, RuntimeConfig};
use services::{ServiceRegistry, ServicesConfig};
use std::sync::Arc;

#[test]
fn test_service_registry_instantiation() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));
    let registry = ServiceRegistry::new(ServicesConfig, runtime);
    assert!(format!("{:?}", registry).contains("ServiceRegistry"));
}
