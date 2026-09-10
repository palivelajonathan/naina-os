use kernel::{Kernel, KernelConfig};
use runtime::{Runtime, RuntimeConfig};
use std::sync::Arc;

#[test]
fn test_runtime_instantiation() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let rt = Runtime::new(RuntimeConfig, kernel);
    assert!(format!("{:?}", rt).contains("Runtime"));
}
