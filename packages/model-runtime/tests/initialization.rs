use model_runtime::{ModelRuntime, ModelRuntimeConfig};

#[test]
fn test_model_runtime_instantiation() {
    let config = ModelRuntimeConfig::default();
    let runtime = ModelRuntime::new(config);
    assert!(format!("{:?}", runtime).contains("ModelRuntime"));
    assert_eq!(runtime.config().max_vram_bytes, 4_800_000_000);

    let root_config = configuration::Config::default();
    let runtime_from_root = ModelRuntime::from_root_config(&root_config);
    assert!(format!("{:?}", runtime_from_root).contains("ModelRuntime"));
}
