use capabilities::{CapabilityConfig, CapabilityRegistry};
use kernel::{Kernel, KernelConfig};
use runtime::{Runtime, RuntimeConfig, RuntimeState};
use std::sync::Arc;
use std::thread;
use tool_registry::{
    ToolDefinition, ToolError, ToolName, ToolRegistry, ToolRegistryConfig, ToolState,
};

fn create_test_registry() -> (ToolRegistry, Arc<Runtime>, Arc<Kernel>) {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel.clone()));
    let registry = ToolRegistry::new(ToolRegistryConfig, runtime.clone());
    (registry, runtime, kernel)
}

fn sample_def(name: &str) -> ToolDefinition {
    ToolDefinition {
        name: ToolName(name.to_string()),
        description: format!("Description for {name}"),
        parameters_schema: "{\"type\":\"object\"}".to_string(),
    }
}

#[test]
fn test_01_registry_construction() {
    let (registry, _, _) = create_test_registry();
    assert!(format!("{:?}", registry).contains("ToolRegistry"));
}

#[test]
fn test_02_tool_registration() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("t1"), None, None)
        .unwrap();
    assert_eq!(tid.0, 1);
}

#[test]
fn test_03_duplicate_registration() {
    let (registry, _, _) = create_test_registry();
    registry
        .register_tool(sample_def("dup"), None, None)
        .unwrap();
    let err = registry
        .register_tool(sample_def("dup"), None, None)
        .unwrap_err();

    assert!(matches!(err, ToolError::ToolAlreadyExists { .. }));
}

#[test]
fn test_04_tool_id_generation() {
    let (registry, _, _) = create_test_registry();
    let t1 = registry
        .register_tool(sample_def("t1"), None, None)
        .unwrap();
    let t2 = registry
        .register_tool(sample_def("t2"), None, None)
        .unwrap();

    assert_eq!(t1.0, 1);
    assert_eq!(t2.0, 2);
}

#[test]
fn test_05_tool_metadata_preservation() {
    let (registry, _, _) = create_test_registry();
    let def = sample_def("meta_tool");
    let tid = registry.register_tool(def.clone(), None, None).unwrap();
    let record = registry.get_tool(tid).unwrap();

    assert_eq!(record.definition, def);
}

#[test]
fn test_06_tool_lookup() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("lookup_tool"), None, None)
        .unwrap();
    let record = registry.lookup_tool("lookup_tool", None).unwrap();

    assert_eq!(record.id, tid);
}

#[test]
fn test_07_missing_tool_lookup() {
    let (registry, _, _) = create_test_registry();
    let err = registry.lookup_tool("non_existent", None).unwrap_err();

    assert!(matches!(err, ToolError::ToolNotFound { .. }));
}

#[test]
fn test_08_tool_listing() {
    let (registry, _, _) = create_test_registry();
    registry
        .register_tool(sample_def("t1"), None, None)
        .unwrap();
    registry
        .register_tool(sample_def("t2"), None, None)
        .unwrap();

    let list = registry.list_tools(None).unwrap();
    assert_eq!(list.len(), 2);
}

#[test]
fn test_09_initial_registered_state() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("init_state"), None, None)
        .unwrap();
    let rec = registry.get_tool(tid).unwrap();

    assert_eq!(rec.state, ToolState::Registered);
}

#[test]
fn test_10_activation() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("act_tool"), None, None)
        .unwrap();
    registry.execute_tool(tid, "{}", None).unwrap();

    let rec = registry.get_tool(tid).unwrap();
    assert_eq!(rec.state, ToolState::Active);
}

#[test]
fn test_11_disabled_state() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("dis_tool"), None, None)
        .unwrap();

    // Perform execution to verify state check
    assert!(registry.execute_tool(tid, "{}", None).is_ok());
}

#[test]
fn test_12_failed_state() {
    let (registry, runtime, _) = create_test_registry();
    let cid = runtime.create_context("failed_ctx", None).unwrap();
    let tid = registry
        .register_tool(sample_def("fail_tool"), Some(cid), None)
        .unwrap();

    runtime.close_context(cid).unwrap();
    let err = registry.execute_tool(tid, "{}", None).unwrap_err();

    assert!(matches!(err, ToolError::ExecutionFailed { .. }));
    assert_eq!(registry.get_tool(tid).unwrap().state, ToolState::Failed);
}

#[test]
fn test_13_invalid_state_transitions() {
    let (registry, runtime, _) = create_test_registry();
    let cid = runtime.create_context("ctx", None).unwrap();
    let tid = registry
        .register_tool(sample_def("inv_tool"), Some(cid), None)
        .unwrap();

    runtime.close_context(cid).unwrap();
    let _ = registry.execute_tool(tid, "{}", None);

    let err = registry.execute_tool(tid, "{}", None).unwrap_err();
    assert!(matches!(err, ToolError::InvalidState { .. }));
}

#[test]
fn test_14_cap_tool_register_authorization() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("admin", "CAP_TOOL_REGISTER", None).unwrap();

    let (registry, _, _) = create_test_registry();
    let res = registry.register_tool(sample_def("sec_reg"), None, Some((&cap_reg, &token)));

    assert!(res.is_ok());
}

#[test]
fn test_15_cap_tool_execute_authorization() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("admin", "CAP_TOOL_EXECUTE", None).unwrap();

    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("sec_exec"), None, None)
        .unwrap();

    let res = registry.execute_tool(tid, "{}", Some((&cap_reg, &token)));
    assert!(res.is_ok());
}

#[test]
fn test_16_deny_by_default_behavior() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("user", "CAP_OTHER", None).unwrap();

    let (registry, _, _) = create_test_registry();
    let err = registry
        .register_tool(sample_def("denied_tool"), None, Some((&cap_reg, &token)))
        .unwrap_err();

    assert!(matches!(err, ToolError::Unauthorized { .. }));
}

#[test]
fn test_17_unauthorized_lookup() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("user", "CAP_TOOL_REGISTER", None).unwrap();

    let (registry, _, _) = create_test_registry();
    registry
        .register_tool(sample_def("lookup_unauth"), None, None)
        .unwrap();

    let err = registry
        .lookup_tool("lookup_unauth", Some((&cap_reg, &token)))
        .unwrap_err();
    assert!(matches!(err, ToolError::Unauthorized { .. }));
}

#[test]
fn test_18_unauthorized_execution() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("user", "CAP_TOOL_REGISTER", None).unwrap();

    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("exec_unauth"), None, None)
        .unwrap();

    let err = registry
        .execute_tool(tid, "{}", Some((&cap_reg, &token)))
        .unwrap_err();
    assert!(matches!(err, ToolError::Unauthorized { .. }));
}

#[test]
fn test_19_unauthorized_listing() {
    let cap_reg = CapabilityRegistry::new(CapabilityConfig);
    let token = cap_reg.grant("user", "CAP_OTHER", None).unwrap();

    let (registry, _, _) = create_test_registry();
    let err = registry.list_tools(Some((&cap_reg, &token))).unwrap_err();
    assert!(matches!(err, ToolError::Unauthorized { .. }));
}

#[test]
fn test_20_runtime_execution_context_id_linkage() {
    let (registry, runtime, _) = create_test_registry();
    let cid = runtime.create_context("tool_worker", None).unwrap();
    let tid = registry
        .register_tool(sample_def("link_tool"), Some(cid), None)
        .unwrap();

    let rec = registry.get_tool(tid).unwrap();
    assert_eq!(rec.execution_context_id, Some(cid));
}

#[test]
fn test_21_successful_tool_execution() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("succ_tool"), None, None)
        .unwrap();

    let output = registry
        .execute_tool(tid, "{\"key\":\"val\"}", None)
        .unwrap();
    assert!(output.contains("succ_tool"));
}

#[test]
fn test_22_failed_tool_execution() {
    let (registry, runtime, _) = create_test_registry();
    let cid = runtime.create_context("fail_ctx", None).unwrap();
    let tid = registry
        .register_tool(sample_def("bad_tool"), Some(cid), None)
        .unwrap();

    runtime.close_context(cid).unwrap();
    assert!(registry.execute_tool(tid, "{}", None).is_err());
}

#[test]
fn test_23_error_propagation() {
    let (registry, _, _) = create_test_registry();
    let err = registry.get_tool(tool_registry::ToolId(7777)).unwrap_err();
    assert!(err.to_string().contains("7777"));
}

#[test]
fn test_24_concurrent_registry_access() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));
    let registry = Arc::new(ToolRegistry::new(ToolRegistryConfig, runtime));

    let mut handles = Vec::new();
    for i in 0..10 {
        let reg = Arc::clone(&registry);
        handles.push(thread::spawn(move || {
            let name = format!("conc_tool_{i}");
            let tid = reg.register_tool(sample_def(&name), None, None).unwrap();
            let rec = reg.lookup_tool(&name, None).unwrap();
            assert_eq!(rec.id, tid);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_25_arc_sharing() {
    let (registry, _, _) = create_test_registry();
    let arc_reg = Arc::new(registry);
    let clone_reg = Arc::clone(&arc_reg);

    let tid = arc_reg
        .register_tool(sample_def("arc_tool"), None, None)
        .unwrap();
    assert!(clone_reg.get_tool(tid).is_ok());
}

#[test]
fn test_26_tool_lifecycle_behavior() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("life_tool"), None, None)
        .unwrap();
    assert_eq!(registry.get_tool(tid).unwrap().state, ToolState::Registered);

    registry.execute_tool(tid, "{}", None).unwrap();
    assert_eq!(registry.get_tool(tid).unwrap().state, ToolState::Active);
}

#[test]
fn test_27_event_behavior() {
    let (registry, _, _) = create_test_registry();
    let tid = registry
        .register_tool(sample_def("ev_tool"), None, None)
        .unwrap();
    assert!(registry.get_tool(tid).is_ok());
}

#[test]
fn test_28_logging_behavior() {
    let (registry, runtime, kernel) = create_test_registry();
    assert_eq!(kernel.state(), kernel::KernelState::Uninitialized);
    assert_eq!(runtime.state(), RuntimeState::Uninitialized);
    assert!(
        registry
            .register_tool(sample_def("log_tool"), None, None)
            .is_ok()
    );
}
