use desktop_host::{DesktopHostApp, DesktopHostConfig, DesktopHostState};
use services::ServiceRegistry;
use std::sync::Arc;

#[test]
fn test_desktop_host_config_defaults() {
    let config = DesktopHostConfig::default();
    assert_eq!(config.qwen_model_path, "models/qwen7b.gguf");
    assert_eq!(config.whisper_model_path, "models/whisper.bin");
    assert_eq!(config.piper_model_path, "models/piper.onnx");
    assert_eq!(config.obsidian_vault_path, "vault/");
    assert_eq!(config.hotkey_trigger, "Ctrl+Shift+Space");
}

#[test]
fn test_desktop_host_app_boot() {
    let runtime = Arc::new(runtime::Runtime::with_default_kernel(
        runtime::RuntimeConfig,
    ));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let app = DesktopHostApp::boot(DesktopHostConfig::default(), runtime, services).unwrap();
    assert_eq!(app.state(), DesktopHostState::Ready);
}
