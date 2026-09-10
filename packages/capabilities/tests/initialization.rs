use capabilities::{CapabilityConfig, CapabilityRegistry};

#[test]
fn test_capability_registry_instantiation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    assert!(format!("{:?}", registry).contains("CapabilityRegistry"));
}
