use kernel::{Kernel, KernelConfig};

#[test]
fn test_kernel_instantiation() {
    let kernel = Kernel::new(KernelConfig::default());
    assert!(format!("{:?}", kernel).contains("Kernel"));
}
