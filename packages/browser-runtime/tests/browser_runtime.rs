use browser_runtime::{
    BrowserAutomationEngine, BrowserRuntime, BrowserRuntimeConfig, BrowserRuntimeError,
    BrowserState, CdpBrowserEngine, MockBrowserEngine,
};
use services::ServiceRegistry;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn setup_test_browser_runtime() -> BrowserRuntime {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = BrowserRuntimeConfig::default();
    let br = BrowserRuntime::new(config, runtime, services);

    // Register deterministic mock engine for CI execution
    let mock_engine = Arc::new(MockBrowserEngine::default());
    br.register_engine(mock_engine);
    br
}

#[test]
fn test_01_construction_and_configuration() {
    let br = setup_test_browser_runtime();
    assert!(br.config().headless);
    assert_eq!(br.config().remote_debugging_port, 9222);
    assert_eq!(br.config().navigation_timeout_ms, 10_000);
    assert_eq!(br.config().latency_target_ms, 1_000);
    assert_eq!(br.state(), BrowserState::Idle);
}

#[test]
fn test_02_successful_navigation() {
    let br = setup_test_browser_runtime();
    let res = br.navigate_to("https://naina.ai").unwrap();

    assert_eq!(res.target_id, "mock_tab_1");
    assert_eq!(res.current_url, "https://naina.ai");
    assert!(res.extracted_text.is_some());
    assert_eq!(br.state(), BrowserState::Idle);
}

#[test]
fn test_03_tab_opening_and_closing() {
    let br = setup_test_browser_runtime();
    let info = br.open_tab("https://example.com").unwrap();
    assert_eq!(info.target_id, "mock_tab_1");

    br.close_tab("mock_tab_1").unwrap();
    assert_eq!(br.state(), BrowserState::Idle);
}

#[test]
fn test_04_semantic_dom_inspection() {
    let br = setup_test_browser_runtime();
    let nodes = br.inspect_dom("mock_tab_1").unwrap();

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].tag_name, "h1");
    assert_eq!(nodes[0].text_content, "Mock Heading");
    assert_eq!(br.state(), BrowserState::Idle);
}

#[test]
fn test_05_page_text_extraction() {
    let br = setup_test_browser_runtime();
    let text = br.extract_page_text("mock_tab_1").unwrap();

    assert!(text.contains("Mock clean extracted page text"));
    assert_eq!(br.state(), BrowserState::Idle);
}

#[test]
fn test_06_navigation_failure_and_state_recovery() {
    let br = setup_test_browser_runtime();
    let failing_engine = Arc::new(MockBrowserEngine {
        should_fail_launch: false,
        should_fail_nav: true,
    });
    br.register_engine(failing_engine);

    let err = br.navigate_to("https://invalid.url").unwrap_err();
    assert!(matches!(err, BrowserRuntimeError::NavigationFailed { .. }));
    assert_eq!(br.state(), BrowserState::Error);

    br.reset_state();
    assert_eq!(br.state(), BrowserState::Idle);
}

#[test]
fn test_07_tab_not_found_handling() {
    let br = setup_test_browser_runtime();
    let err = br.inspect_dom("missing_tab").unwrap_err();
    assert!(matches!(err, BrowserRuntimeError::TabNotFound { .. }));
}

#[test]
fn test_08_cancellation_protocol() {
    let br = setup_test_browser_runtime();
    br.cancel_operation();

    let err = br.navigate_to("https://naina.ai").unwrap_err();
    assert!(matches!(err, BrowserRuntimeError::NavigationFailed { .. }));

    br.reset_state();
    assert_eq!(br.state(), BrowserState::Idle);
}

#[test]
fn test_09_configuration_from_browser_config_integration() {
    let browser_cfg = configuration::BrowserConfig { enabled: true };
    let br_cfg = BrowserRuntimeConfig::from_browser_config(&browser_cfg);

    assert!(br_cfg.enabled);
    assert!(br_cfg.headless);
    assert_eq!(br_cfg.latency_target_ms, 1_000);
}

#[test]
fn test_10_cdp_browser_engine_spawn_fallback() {
    let cdp_engine = CdpBrowserEngine::new();
    let pid = cdp_engine.ensure_browser_spawned(None, 9222, true).unwrap();
    assert!(pid > 0);

    let res = cdp_engine.navigate_to("https://naina.ai").unwrap();
    assert_eq!(res.current_url, "https://naina.ai");
}

#[test]
fn test_11_concurrent_browser_runtime_access() {
    let br = setup_test_browser_runtime();
    let br_arc = Arc::new(br);

    let mut handles = Vec::new();
    for _ in 0..4 {
        let br_ref = Arc::clone(&br_arc);
        handles.push(thread::spawn(move || {
            let res = br_ref.navigate_to("https://naina.ai").unwrap();
            assert_eq!(res.target_id, "mock_tab_1");
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_12_performance_latency_measurement_target() {
    let br = setup_test_browser_runtime();
    let start = Instant::now();
    let _ = br.navigate_to("https://naina.ai").unwrap();
    let elapsed = start.elapsed();

    println!("Browser navigation latency: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 1000,
        "Browser command execution exceeded 1000ms target: {}ms",
        elapsed.as_millis()
    );
}
