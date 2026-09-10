use desktop_runtime::{
    AppLaunchRequest, DesktopAutomationEngine, DesktopRuntime, DesktopRuntimeConfig,
    DesktopRuntimeError, DesktopState, MockDesktopEngine, Win32DesktopEngine,
};
use services::ServiceRegistry;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn setup_test_desktop_runtime() -> DesktopRuntime {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = DesktopRuntimeConfig::default();
    let dr = DesktopRuntime::new(config, runtime, services);

    // Register deterministic mock engine for CI execution
    let mock_engine = Arc::new(MockDesktopEngine::default());
    dr.register_engine(mock_engine);
    dr
}

#[test]
fn test_01_construction_and_configuration() {
    let dr = setup_test_desktop_runtime();
    assert_eq!(dr.config().max_search_depth, 5);
    assert_eq!(dr.config().max_element_count, 100);
    assert_eq!(dr.config().inspection_timeout_ms, 200);
    assert_eq!(dr.config().latency_target_ms, 500);
    assert_eq!(dr.state(), DesktopState::Idle);
}

#[test]
fn test_02_successful_app_launch() {
    let dr = setup_test_desktop_runtime();
    let req = AppLaunchRequest {
        app_name: "notepad".to_string(),
        executable_path: None,
        arguments: vec![],
    };

    let res = dr.launch_app(req).unwrap();
    assert_eq!(res.process_id, 4200);
    assert_eq!(res.window_handle, 0x0005_0001);
    assert_eq!(dr.state(), DesktopState::Idle);
}

#[test]
fn test_03_active_window_enumeration() {
    let dr = setup_test_desktop_runtime();
    let windows = dr.enumerate_windows().unwrap();

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].handle, 0x0005_0001);
    assert_eq!(windows[0].title, "Mock Application Window");
}

#[test]
fn test_04_bounded_ui_element_inspection() {
    let dr = setup_test_desktop_runtime();
    let elements = dr.inspect_ui_elements(0x0005_0001).unwrap();

    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].name, "OK Button");
    assert_eq!(elements[0].control_type, "Button");
    assert_eq!(dr.state(), DesktopState::Idle);
}

#[test]
fn test_05_synthetic_input_injection() {
    let dr = setup_test_desktop_runtime();
    dr.inject_input(0x0005_0001, "click_ok_button").unwrap();
    assert_eq!(dr.state(), DesktopState::Idle);
}

#[test]
fn test_06_app_launch_failure_and_state_recovery() {
    let dr = setup_test_desktop_runtime();
    let failing_engine = Arc::new(MockDesktopEngine {
        should_fail_launch: true,
        should_fail_inspect: false,
    });
    dr.register_engine(failing_engine);

    let req = AppLaunchRequest {
        app_name: "invalid_app".to_string(),
        executable_path: None,
        arguments: vec![],
    };

    let err = dr.launch_app(req).unwrap_err();
    assert!(matches!(err, DesktopRuntimeError::AppLaunchFailed { .. }));
    assert_eq!(dr.state(), DesktopState::Error);

    dr.reset_state();
    assert_eq!(dr.state(), DesktopState::Idle);
}

#[test]
fn test_07_ui_inspection_failure_and_state_recovery() {
    let dr = setup_test_desktop_runtime();
    let failing_engine = Arc::new(MockDesktopEngine {
        should_fail_launch: false,
        should_fail_inspect: true,
    });
    dr.register_engine(failing_engine);

    let err = dr.inspect_ui_elements(0x0005_0001).unwrap_err();
    assert!(matches!(err, DesktopRuntimeError::UiElementNotFound { .. }));
    assert_eq!(dr.state(), DesktopState::Error);

    dr.reset_state();
    assert_eq!(dr.state(), DesktopState::Idle);
}

#[test]
fn test_08_invalid_window_handle_rejection() {
    let dr = setup_test_desktop_runtime();
    let err = dr.inspect_ui_elements(0).unwrap_err();
    assert!(matches!(
        err,
        DesktopRuntimeError::WindowNotFound { handle: 0 }
    ));
}

#[test]
fn test_09_cancellation_protocol() {
    let dr = setup_test_desktop_runtime();
    dr.cancel_operation();

    let req = AppLaunchRequest {
        app_name: "notepad".to_string(),
        executable_path: None,
        arguments: vec![],
    };

    let err = dr.launch_app(req).unwrap_err();
    assert!(matches!(err, DesktopRuntimeError::AppLaunchFailed { .. }));

    dr.reset_state();
    assert_eq!(dr.state(), DesktopState::Idle);
}

#[test]
fn test_10_configuration_from_desktop_config_integration() {
    let desktop_cfg = configuration::DesktopConfig { enabled: true };
    let dr_cfg = DesktopRuntimeConfig::from_desktop_config(&desktop_cfg);

    assert!(dr_cfg.enabled);
    assert_eq!(dr_cfg.max_search_depth, 5);
    assert_eq!(dr_cfg.latency_target_ms, 500);
}

#[test]
fn test_11_win32_desktop_engine_fallback() {
    let win32_engine = Win32DesktopEngine::new();
    let hwnd = win32_engine.ensure_overlay_rendered(200).unwrap();
    assert_eq!(hwnd, 0x0001_0001);

    let windows = win32_engine.enumerate_windows().unwrap();
    assert!(windows.len() >= 2);
    assert_eq!(windows[0].handle, 0x0001_0001);
}

#[test]
fn test_12_concurrent_desktop_runtime_access() {
    let dr = setup_test_desktop_runtime();
    let dr_arc = Arc::new(dr);

    let mut handles = Vec::new();
    for _ in 0..4 {
        let dr_ref = Arc::clone(&dr_arc);
        handles.push(thread::spawn(move || {
            let req = AppLaunchRequest {
                app_name: "notepad".to_string(),
                executable_path: None,
                arguments: vec![],
            };
            let res = dr_ref.launch_app(req).unwrap();
            assert_eq!(res.process_id, 4200);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_13_performance_latency_measurement_target() {
    let dr = setup_test_desktop_runtime();
    let req = AppLaunchRequest {
        app_name: "notepad".to_string(),
        executable_path: None,
        arguments: vec![],
    };

    let start = Instant::now();
    let _ = dr.launch_app(req).unwrap();
    let elapsed = start.elapsed();

    println!("Desktop launch latency: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 500,
        "Desktop command execution exceeded 500ms target: {}ms",
        elapsed.as_millis()
    );
}
