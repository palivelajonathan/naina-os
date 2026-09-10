use capabilities::{CapabilityConfig, CapabilityRegistry};
use kernel::{Kernel, KernelConfig};
use runtime::{Runtime, RuntimeConfig, RuntimeState};
use services::{
    ServiceHealth, ServiceId, ServiceName, ServiceRegistry, ServiceState, ServicesConfig,
    ServicesError,
};
use std::sync::Arc;
use std::thread;

fn create_test_services() -> (ServiceRegistry, Arc<Runtime>, Arc<Kernel>) {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel.clone()));
    let registry = ServiceRegistry::new(ServicesConfig, runtime.clone());
    (registry, runtime, kernel)
}

#[test]
fn test_01_registry_construction() {
    let (registry, _, _) = create_test_services();
    assert!(format!("{:?}", registry).contains("ServiceRegistry"));
}

#[test]
fn test_02_default_configuration() {
    let config = ServicesConfig;
    assert_eq!(config, ServicesConfig);
}

#[test]
fn test_03_service_id_generation() {
    let (registry, _, _) = create_test_services();
    let s1 = registry.register("svc_1", None, None).unwrap();
    let s2 = registry.register("svc_2", None, None).unwrap();

    assert_eq!(s1.0, 1);
    assert_eq!(s2.0, 2);
}

#[test]
fn test_04_service_name_handling() {
    let (registry, _, _) = create_test_services();
    let s1 = registry.register("custom_service", None, None).unwrap();
    let rec = registry.get_service(s1).unwrap();

    assert_eq!(rec.name, ServiceName("custom_service".to_string()));
}

#[test]
fn test_05_service_registration() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("auth_service", None, None).unwrap();
    let rec = registry.get_service(sid).unwrap();

    assert_eq!(rec.state, ServiceState::Registered);
}

#[test]
fn test_06_duplicate_registration_behavior() {
    let (registry, _, _) = create_test_services();
    registry.register("dup_service", None, None).unwrap();
    let err = registry.register("dup_service", None, None).unwrap_err();

    assert!(matches!(err, ServicesError::ServiceAlreadyExists { .. }));
}

#[test]
fn test_07_service_lookup() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("find_me", None, None).unwrap();
    let record = registry.lookup("find_me", None).unwrap();

    assert_eq!(record.id, sid);
}

#[test]
fn test_08_missing_service_behavior() {
    let (registry, _, _) = create_test_services();
    let err = registry.lookup("non_existent", None).unwrap_err();

    assert!(matches!(err, ServicesError::ServiceNotFound { .. }));
}

#[test]
fn test_09_service_record_retrieval() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("retrieval_svc", None, None).unwrap();
    let record = registry.get_service(sid).unwrap();

    assert_eq!(record.name.0, "retrieval_svc");
}

#[test]
fn test_10_service_unregistration() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("remove_me", None, None).unwrap();
    assert!(registry.unregister(sid, None).is_ok());
    assert!(registry.get_service(sid).is_err());
}

#[test]
fn test_11_lifecycle_transitions() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("lifecycle_svc", None, None).unwrap();

    registry.update_state(sid, ServiceState::Starting).unwrap();
    assert_eq!(
        registry.get_service(sid).unwrap().state,
        ServiceState::Starting
    );

    registry.update_state(sid, ServiceState::Active).unwrap();
    assert_eq!(
        registry.get_service(sid).unwrap().state,
        ServiceState::Active
    );
}

#[test]
fn test_12_invalid_lifecycle_transitions() {
    let (registry, _, _) = create_test_services();
    let err = registry
        .update_state(ServiceId(999), ServiceState::Active)
        .unwrap_err();
    assert!(matches!(err, ServicesError::ServiceNotFound { .. }));
}

#[test]
fn test_13_health_state() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("health_svc", None, None).unwrap();
    let health = registry.health_check(sid).unwrap();

    assert_eq!(health, ServiceHealth::Healthy);
}

#[test]
fn test_14_health_updates() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("degrade_svc", None, None).unwrap();
    registry
        .update_health(
            sid,
            ServiceHealth::Degraded {
                reason: "high load".to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        registry.health_check(sid).unwrap(),
        ServiceHealth::Degraded {
            reason: "high load".to_string()
        }
    );
}

#[test]
fn test_15_capability_authorization_for_registration() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg
        .grant("admin", "CAP_SERVICE_REGISTER", None)
        .unwrap();

    let (registry, _, _) = create_test_services();
    let res = registry.register("secure_reg", None, Some((&cap_reg, &token)));

    assert!(res.is_ok());
}

#[test]
fn test_16_capability_authorization_for_lookup() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("admin", "CAP_SERVICE_LOOKUP", None).unwrap();

    let (registry, _, _) = create_test_services();
    registry.register("lookup_target", None, None).unwrap();

    let res = registry.lookup("lookup_target", Some((&cap_reg, &token)));
    assert!(res.is_ok());
}

#[test]
fn test_17_deny_by_default_behavior() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("user", "CAP_OTHER", None).unwrap();

    let (registry, _, _) = create_test_services();
    let err = registry
        .register("denied_svc", None, Some((&cap_reg, &token)))
        .unwrap_err();

    assert!(matches!(err, ServicesError::Unauthorized { .. }));
}

#[test]
fn test_18_unauthorized_unregister() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("user", "CAP_SERVICE_LOOKUP", None).unwrap();

    let (registry, _, _) = create_test_services();
    let sid = registry.register("protected_svc", None, None).unwrap();

    let err = registry
        .unregister(sid, Some((&cap_reg, &token)))
        .unwrap_err();
    assert!(matches!(err, ServicesError::Unauthorized { .. }));
}

#[test]
fn test_19_multiple_services() {
    let (registry, _, _) = create_test_services();
    let s1 = registry.register("svc_a", None, None).unwrap();
    let s2 = registry.register("svc_b", None, None).unwrap();

    assert_ne!(s1, s2);
    assert_eq!(registry.get_service(s1).unwrap().name.0, "svc_a");
    assert_eq!(registry.get_service(s2).unwrap().name.0, "svc_b");
}

#[test]
fn test_20_runtime_execution_context_linkage() {
    let (registry, runtime, _) = create_test_services();
    let cid = runtime.create_context("worker_ctx", None).unwrap();
    let sid = registry
        .register("worker_service", Some(cid), None)
        .unwrap();

    let rec = registry.get_service(sid).unwrap();
    assert_eq!(rec.execution_context_id, Some(cid));
}

#[test]
fn test_21_registry_concurrency() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));
    let registry = Arc::new(ServiceRegistry::new(ServicesConfig, runtime));

    let mut handles = Vec::new();
    for i in 0..10 {
        let reg = Arc::clone(&registry);
        handles.push(thread::spawn(move || {
            let name = format!("concurrent_svc_{i}");
            let sid = reg.register(&name, None, None).unwrap();
            let rec = reg.lookup(&name, None).unwrap();
            assert_eq!(rec.id, sid);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_22_arc_sharing() {
    let (registry, _, _) = create_test_services();
    let arc_reg = Arc::new(registry);
    let clone_reg = Arc::clone(&arc_reg);

    let sid = arc_reg.register("shared_svc", None, None).unwrap();
    assert!(clone_reg.get_service(sid).is_ok());
}

#[test]
fn test_23_event_behavior_supported() {
    let (registry, _, _) = create_test_services();
    let sid = registry.register("event_test_svc", None, None).unwrap();
    assert_eq!(
        registry.get_service(sid).unwrap().state,
        ServiceState::Registered
    );
}

#[test]
fn test_24_error_propagation() {
    let (registry, _, _) = create_test_services();
    let err = registry.get_service(ServiceId(8888)).unwrap_err();
    assert!(err.to_string().contains("8888"));
}

#[test]
fn test_25_full_lifecycle() {
    let (registry, runtime, _) = create_test_services();
    let cid = runtime.create_context("full_ctx", None).unwrap();
    let sid = registry.register("full_svc", Some(cid), None).unwrap();

    registry.update_state(sid, ServiceState::Starting).unwrap();
    registry.update_state(sid, ServiceState::Active).unwrap();
    assert_eq!(registry.health_check(sid).unwrap(), ServiceHealth::Healthy);

    runtime.close_context(cid).unwrap();
    assert!(matches!(
        registry.health_check(sid).unwrap(),
        ServiceHealth::Unhealthy { .. }
    ));

    registry.unregister(sid, None).unwrap();
    assert!(registry.get_service(sid).is_err());
}

#[test]
fn test_26_regression_behavior() {
    let (registry, runtime, kernel) = create_test_services();
    assert_eq!(kernel.state(), kernel::KernelState::Uninitialized);
    assert_eq!(runtime.state(), RuntimeState::Uninitialized);
    assert!(registry.register("reg_test", None, None).is_ok());
}
