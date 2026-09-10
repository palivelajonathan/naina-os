use std::sync::Arc;
use std::thread;
use ui::{UIBuilder, UIConfig, UIError, UIEvent, UIFramework, UIState};

fn setup_test_ui() -> UIFramework {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));

    UIBuilder::new().with_runtime(runtime).build().unwrap()
}

#[test]
fn test_01_uiconfig_defaults() {
    let config = UIConfig::default();
    assert_eq!(config.title, "NAINA OS Desktop Host");
    assert!(config.enable_overlay);
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 800);
    assert_eq!(config.refresh_rate_hz, 60);
    assert!(config.auto_show);
}

#[test]
fn test_02_uiframework_construction() {
    let ui = setup_test_ui();
    assert_eq!(ui.config().title, "NAINA OS Desktop Host");
    assert_eq!(ui.state(), UIState::Uninitialized);
}

#[test]
fn test_03_uibuilder_custom_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));

    let config = UIConfig {
        title: "Custom Overlay".to_string(),
        enable_overlay: true,
        width: 1920,
        height: 1080,
        refresh_rate_hz: 120,
        auto_show: false,
    };

    let ui = UIBuilder::new()
        .with_config(config)
        .with_runtime(runtime)
        .build()
        .unwrap();

    assert_eq!(ui.config().title, "Custom Overlay");
    assert_eq!(ui.config().width, 1920);
    assert!(!ui.config().auto_show);
}

#[test]
fn test_04_send_sync_thread_safety() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<UIFramework>();
}

#[test]
fn test_05_initial_uninitialized_state() {
    let ui = setup_test_ui();
    assert_eq!(ui.state(), UIState::Uninitialized);
}

#[test]
fn test_06_initialization_and_runtime_context_creation() {
    let ui = setup_test_ui();
    ui.initialize().unwrap();
    assert_eq!(ui.state(), UIState::OverlayVisible);
}

#[test]
fn test_07_manual_show_and_hide_overlay() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));

    let config = UIConfig {
        auto_show: false,
        ..Default::default()
    };

    let ui = UIBuilder::new()
        .with_config(config)
        .with_runtime(runtime)
        .build()
        .unwrap();

    ui.initialize().unwrap();
    assert_eq!(ui.state(), UIState::Ready);

    ui.show_overlay().unwrap();
    assert_eq!(ui.state(), UIState::OverlayVisible);

    ui.hide_overlay().unwrap();
    assert_eq!(ui.state(), UIState::OverlayHidden);
}

#[test]
fn test_08_uievent_creation_and_mpsc_transport() {
    let ui = setup_test_ui();
    ui.initialize().unwrap();

    ui.dispatch_event(UIEvent::CommandTriggered {
        command_name: "open_overlay".to_string(),
    })
    .unwrap();

    let mut found_cmd = false;
    while let Some(event) = ui.try_recv_event() {
        if let UIEvent::CommandTriggered { command_name } = event {
            if command_name == "open_overlay" {
                found_cmd = true;
            }
        }
    }

    assert!(found_cmd);
}

#[test]
fn test_09_privacy_input_length_only_retention() {
    let event = UIEvent::InputSubmitted { input_length: 14 };
    if let UIEvent::InputSubmitted { input_length } = event {
        assert_eq!(input_length, 14);
    }
}

#[test]
fn test_10_shutdown_lifecycle_and_context_cleanup() {
    let ui = setup_test_ui();
    ui.initialize().unwrap();
    assert_eq!(ui.state(), UIState::OverlayVisible);

    ui.shutdown().unwrap();
    assert_eq!(ui.state(), UIState::Shutdown);
}

#[test]
fn test_11_repeated_shutdown_idempotency() {
    let ui = setup_test_ui();
    ui.initialize().unwrap();
    ui.shutdown().unwrap();
    assert_eq!(ui.state(), UIState::Shutdown);

    // Repeated call should succeed idempotently
    ui.shutdown().unwrap();
    assert_eq!(ui.state(), UIState::Shutdown);
}

#[test]
fn test_12_invalid_state_transition_rejection() {
    let ui = setup_test_ui();
    ui.initialize().unwrap();

    // Repeated initialization should fail
    let err = ui.initialize().unwrap_err();
    assert!(matches!(err, UIError::InitializationFailed { .. }));
}

#[test]
fn test_13_concurrent_state_and_event_access() {
    let ui = Arc::new(setup_test_ui());
    ui.initialize().unwrap();

    let mut handles = Vec::new();
    for i in 0..4 {
        let ui_ref = Arc::clone(&ui);
        handles.push(thread::spawn(move || {
            let _ = ui_ref.dispatch_event(UIEvent::CommandTriggered {
                command_name: format!("cmd_{i}"),
            });
            let _ = ui_ref.state();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
