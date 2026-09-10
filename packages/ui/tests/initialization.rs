use std::sync::Arc;
use ui::{UIBuilder, UIConfig, UIState};

#[test]
fn test_ui_config_defaults() {
    let config = UIConfig::default();
    assert_eq!(config.title, "NAINA OS Desktop Host");
    assert!(config.enable_overlay);
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 800);
    assert_eq!(config.refresh_rate_hz, 60);
    assert!(config.auto_show);
}

#[test]
fn test_ui_framework_initialization() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));

    let framework = UIBuilder::new().with_runtime(runtime).build().unwrap();

    assert_eq!(framework.state(), UIState::Uninitialized);
    framework.initialize().unwrap();
    assert_eq!(framework.state(), UIState::OverlayVisible);
}
