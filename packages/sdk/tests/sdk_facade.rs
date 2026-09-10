use sdk::{SDKBuilder, SDKConfig, SDKError, SDKFacade, SDKState};
use services::ServiceRegistry;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn setup_test_sdk_facade() -> SDKFacade {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    SDKBuilder::new()
        .with_runtime(runtime)
        .with_services(services)
        .build()
        .unwrap()
}

#[test]
fn test_01_construction_and_defaults() {
    let facade = setup_test_sdk_facade();
    assert_eq!(facade.config().system_name, "NAINA OS");
    assert_eq!(facade.state(), SDKState::Ready);
}

#[test]
fn test_02_custom_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = SDKConfig {
        system_name: "NAINA OS Custom".to_string(),
        enable_logging: true,
        auto_start_runtime: true,
        session_timeout_ms: 60_000,
    };

    let facade = SDKBuilder::new()
        .with_config(config)
        .with_runtime(runtime)
        .with_services(services)
        .build()
        .unwrap();

    assert_eq!(facade.config().system_name, "NAINA OS Custom");
    assert_eq!(facade.config().session_timeout_ms, 60_000);
}

#[test]
fn test_03_session_creation_and_lookup() {
    let facade = setup_test_sdk_facade();
    let session = facade.create_session("sess-1001").unwrap();

    assert_eq!(session.session_id, "sess-1001");
    assert!(session.is_active);

    let retrieved = facade.get_session("sess-1001").unwrap();
    assert_eq!(retrieved.session_id, "sess-1001");
    assert_eq!(retrieved.context_id, session.context_id);
}

#[test]
fn test_04_session_not_found_handling() {
    let facade = setup_test_sdk_facade();
    let err = facade.get_session("nonexistent-sess").unwrap_err();
    assert!(matches!(err, SDKError::SessionNotFound { .. }));
}

#[test]
fn test_05_session_closing_and_expiration() {
    let facade = setup_test_sdk_facade();
    let session = facade.create_session("sess-close").unwrap();
    assert!(session.is_active);

    facade.close_session("sess-close").unwrap();

    let err = facade.get_session("sess-close").unwrap_err();
    assert!(matches!(err, SDKError::SessionNotFound { .. }));
}

#[test]
fn test_06_empty_session_id_rejection() {
    let facade = setup_test_sdk_facade();
    let err = facade.create_session("   ").unwrap_err();
    assert!(matches!(err, SDKError::InitializationFailed { .. }));
}

#[test]
fn test_07_cleanup_expired_sessions() {
    let facade = setup_test_sdk_facade();
    let session = facade.create_session("sess-temp").unwrap();
    assert!(session.is_active);

    // Simulate time past session_timeout_ms (3,600,000ms)
    facade.cleanup_expired_sessions(5_000_000);

    let err = facade.get_session("sess-temp").unwrap_err();
    assert!(matches!(err, SDKError::SessionNotFound { .. }));
}

#[test]
fn test_08_shutdown_lifecycle() {
    let facade = setup_test_sdk_facade();
    let _ = facade.create_session("sess-shutdown").unwrap();

    facade.shutdown().unwrap();
    assert_eq!(facade.state(), SDKState::Shutdown);
}

#[test]
fn test_09_concurrent_session_access() {
    let facade = Arc::new(setup_test_sdk_facade());
    let mut handles = Vec::new();

    for i in 0..4 {
        let facade_ref = Arc::clone(&facade);
        handles.push(thread::spawn(move || {
            let session_id = format!("concurrent-sess-{i}");
            let session = facade_ref.create_session(&session_id).unwrap();
            assert_eq!(session.session_id, session_id);

            let retrieved = facade_ref.get_session(&session_id).unwrap();
            assert_eq!(retrieved.session_id, session_id);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_10_cold_boot_latency_benchmark_target() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let start = Instant::now();
    let facade = SDKBuilder::new()
        .with_runtime(runtime)
        .with_services(services)
        .build()
        .unwrap();
    let elapsed = start.elapsed();

    assert_eq!(facade.state(), SDKState::Ready);
    println!("Microkernel cold boot startup latency: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 2000,
        "Cold boot startup exceeded 2.0 s target limit: {} ms",
        elapsed.as_millis()
    );
}
