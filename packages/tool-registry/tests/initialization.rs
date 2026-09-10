use kernel::{Kernel, KernelConfig};
use runtime::{Runtime, RuntimeConfig};
use std::sync::Arc;
use tool_registry::{ToolRegistry, ToolRegistryConfig};

#[test]
fn test_tool_registry_instantiation() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));
    let registry = ToolRegistry::new(ToolRegistryConfig, runtime);
    assert!(format!("{:?}", registry).contains("ToolRegistry"));
}
