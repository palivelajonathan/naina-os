use desktop_host::{DesktopHostApp, DesktopHostConfig, DesktopHostState};
use services::ServiceRegistry;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn setup_test_host_app() -> DesktopHostApp {
    let runtime = Arc::new(runtime::Runtime::with_default_kernel(
        runtime::RuntimeConfig,
    ));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    DesktopHostApp::boot(DesktopHostConfig::default(), runtime, services).unwrap()
}

#[test]
fn test_01_construction_and_defaults() {
    let app = setup_test_host_app();
    assert_eq!(app.config().qwen_model_path, "models/qwen7b.gguf");
    assert_eq!(app.state(), DesktopHostState::Ready);
}

#[test]
fn test_02_custom_config_boot() {
    let runtime = Arc::new(runtime::Runtime::with_default_kernel(
        runtime::RuntimeConfig,
    ));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = DesktopHostConfig {
        qwen_model_path: "custom/qwen.gguf".to_string(),
        whisper_model_path: "custom/whisper.bin".to_string(),
        piper_model_path: "custom/piper.onnx".to_string(),
        obsidian_vault_path: "custom/vault/".to_string(),
        hotkey_trigger: "Ctrl+Alt+N".to_string(),
    };

    let app = DesktopHostApp::boot(config, runtime, services).unwrap();
    assert_eq!(app.config().qwen_model_path, "custom/qwen.gguf");
    assert_eq!(app.config().hotkey_trigger, "Ctrl+Alt+N");
}

#[test]
fn test_03_mvn_turn_coordination_pipeline() {
    let app = setup_test_host_app();
    let dummy_pcm = vec![0u8; 1600];

    let output_wav = app.process_voice_turn(&dummy_pcm).unwrap();
    assert!(!output_wav.is_empty());
    assert_eq!(app.state(), DesktopHostState::Ready);
}

#[test]
fn test_04_signal_watcher_request_shutdown() {
    let app = setup_test_host_app();
    assert!(!app.signal_watcher().is_shutdown_requested());

    app.signal_watcher().request_shutdown();
    assert!(app.signal_watcher().is_shutdown_requested());
}

#[test]
fn test_05_shutdown_lifecycle() {
    let app = setup_test_host_app();
    assert_eq!(app.state(), DesktopHostState::Ready);

    app.shutdown().unwrap();
    assert_eq!(app.state(), DesktopHostState::Stopped);
}

#[test]
fn test_06_idempotent_shutdown() {
    let app = setup_test_host_app();
    app.shutdown().unwrap();
    assert_eq!(app.state(), DesktopHostState::Stopped);

    // Repeated call should succeed idempotently
    app.shutdown().unwrap();
    assert_eq!(app.state(), DesktopHostState::Stopped);
}

#[test]
fn test_07_concurrent_app_queries() {
    let app = Arc::new(setup_test_host_app());
    let mut handles = Vec::new();

    for _ in 0..4 {
        let app_ref = Arc::clone(&app);
        handles.push(thread::spawn(move || {
            assert_eq!(app_ref.state(), DesktopHostState::Ready);
            assert_eq!(app_ref.config().hotkey_trigger, "Ctrl+Shift+Space");
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_08_cold_boot_latency_benchmark_target() {
    let runtime = Arc::new(runtime::Runtime::with_default_kernel(
        runtime::RuntimeConfig,
    ));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let start = Instant::now();
    let app = DesktopHostApp::boot(DesktopHostConfig::default(), runtime, services).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(app.state(), DesktopHostState::Ready);
    println!(
        "Desktop Host composition boot startup latency: {:?}",
        elapsed
    );
    assert!(
        elapsed.as_millis() < 2000,
        "Cold boot startup exceeded 2.0 s target limit: {} ms",
        elapsed.as_millis()
    );
}
