use kernel::capabilities::{CapabilityConfig, CapabilityRegistry};
use kernel::event_bus::{Event, EventBus, EventBusConfig, EventHandler};
use kernel::{Kernel, KernelConfig};
use runtime::{ContextState, Runtime, RuntimeConfig, RuntimeState};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

struct TestHandler {
    count: Arc<AtomicU32>,
}

impl EventHandler for TestHandler {
    fn handle(&self, _event: &Event) -> kernel::event_bus::Result<()> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn create_test_runtime() -> (Runtime, Arc<Kernel>) {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Runtime::new(RuntimeConfig, kernel.clone());
    (runtime, kernel)
}

#[test]
fn test_01_runtime_construction() {
    let (runtime, _) = create_test_runtime();
    assert!(format!("{:?}", runtime).contains("Runtime"));
}

#[test]
fn test_02_initial_state() {
    let (runtime, _) = create_test_runtime();
    assert_eq!(runtime.state(), RuntimeState::Uninitialized);
}

#[test]
fn test_03_initialization() {
    let (runtime, _) = create_test_runtime();
    assert!(runtime.initialize(None).is_ok());
    assert_eq!(runtime.state(), RuntimeState::Initializing);
}

#[test]
fn test_04_initialization_event() {
    let bus = EventBus::new(EventBusConfig);
    let counter = Arc::new(AtomicU32::new(0));
    let handler = TestHandler {
        count: counter.clone(),
    };
    bus.subscribe("runtime.initializing", Box::new(handler))
        .unwrap();

    let (runtime, _) = create_test_runtime();
    runtime.initialize(Some(&bus)).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_05_start() {
    let (runtime, _) = create_test_runtime();
    runtime.initialize(None).unwrap();
    assert!(runtime.start(None).is_ok());
    assert_eq!(runtime.state(), RuntimeState::Ready);
}

#[test]
fn test_06_ready_state() {
    let (runtime, _) = create_test_runtime();
    runtime.initialize(None).unwrap();
    runtime.start(None).unwrap();
    assert_eq!(runtime.state(), RuntimeState::Ready);
}

#[test]
fn test_07_start_event() {
    let bus = EventBus::new(EventBusConfig);
    let counter = Arc::new(AtomicU32::new(0));
    let handler = TestHandler {
        count: counter.clone(),
    };
    bus.subscribe("runtime.ready", Box::new(handler)).unwrap();

    let (runtime, _) = create_test_runtime();
    runtime.initialize(Some(&bus)).unwrap();
    runtime.start(Some(&bus)).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_08_invalid_lifecycle_transitions() {
    let (runtime, _) = create_test_runtime();
    assert!(runtime.start(None).is_err());
}

#[test]
fn test_09_context_creation() {
    let (runtime, _) = create_test_runtime();
    let cid = runtime.create_context("task_1", None).unwrap();

    assert_eq!(
        runtime.get_context_state(cid).unwrap(),
        ContextState::Created
    );
}

#[test]
fn test_10_context_id_generation() {
    let (runtime, _) = create_test_runtime();
    let c1 = runtime.create_context("ctx_1", None).unwrap();
    let c2 = runtime.create_context("ctx_2", None).unwrap();

    assert_eq!(c1.0, 1);
    assert_eq!(c2.0, 2);
}

#[test]
fn test_11_kernel_process_id_linkage() {
    let (runtime, kernel) = create_test_runtime();
    let cid = runtime.create_context("linked_task", None).unwrap();

    assert!(runtime.get_context_state(cid).is_ok());
    assert!(kernel.get_process_state(kernel::ProcessId(cid.0)).is_ok());
}

#[test]
fn test_12_cap_runtime_execute_authorization() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry
        .grant("admin", "CAP_RUNTIME_EXECUTE", None)
        .unwrap();

    let (runtime, _) = create_test_runtime();
    let res = runtime.create_context("secure_ctx", Some((&registry, &token)));

    assert!(res.is_ok());
}

#[test]
fn test_13_deny_by_default_behavior() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("user", "CAP_OTHER", None).unwrap();

    let (runtime, _) = create_test_runtime();
    assert!(
        runtime
            .create_context("ctx", Some((&registry, &token)))
            .is_err()
    );
}

#[test]
fn test_14_context_execution() {
    let (runtime, _) = create_test_runtime();
    let cid = runtime.create_context("exec_task", None).unwrap();
    assert!(runtime.execute_context(cid).is_ok());
}

#[test]
fn test_15_context_completion() {
    let (runtime, _) = create_test_runtime();
    let cid = runtime.create_context("exec_task", None).unwrap();
    runtime.execute_context(cid).unwrap();

    assert_eq!(
        runtime.get_context_state(cid).unwrap(),
        ContextState::Completed
    );
}

#[test]
fn test_16_context_failure() {
    let (runtime, _) = create_test_runtime();
    let cid = runtime.create_context("ctx", None).unwrap();
    runtime.close_context(cid).unwrap();
    assert_eq!(
        runtime.get_context_state(cid).unwrap(),
        ContextState::Closed
    );
}

#[test]
fn test_17_context_closing() {
    let (runtime, _) = create_test_runtime();
    let cid = runtime.create_context("ctx", None).unwrap();
    assert!(runtime.close_context(cid).is_ok());
    assert_eq!(
        runtime.get_context_state(cid).unwrap(),
        ContextState::Closed
    );
}

#[test]
fn test_18_closed_context_behavior() {
    let (runtime, _) = create_test_runtime();
    let cid = runtime.create_context("ctx", None).unwrap();
    runtime.close_context(cid).unwrap();

    assert!(runtime.execute_context(cid).is_err());
}

#[test]
fn test_19_context_not_found_behavior() {
    let (runtime, _) = create_test_runtime();
    let err = runtime
        .execute_context(runtime::ExecutionContextId(9999))
        .unwrap_err();
    assert!(err.to_string().contains("9999"));
}

#[test]
fn test_20_runtime_shutdown() {
    let (runtime, _) = create_test_runtime();
    runtime.initialize(None).unwrap();
    runtime.start(None).unwrap();
    assert!(runtime.shutdown(None).is_ok());
    assert_eq!(runtime.state(), RuntimeState::Stopped);
}

#[test]
fn test_21_shutdown_event() {
    let bus = EventBus::new(EventBusConfig);
    let counter = Arc::new(AtomicU32::new(0));
    let handler = TestHandler {
        count: counter.clone(),
    };
    bus.subscribe("runtime.shutdown", Box::new(handler))
        .unwrap();

    let (runtime, _) = create_test_runtime();
    runtime.initialize(Some(&bus)).unwrap();
    runtime.start(Some(&bus)).unwrap();
    runtime.shutdown(Some(&bus)).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_22_multiple_contexts() {
    let (runtime, _) = create_test_runtime();
    let c1 = runtime.create_context("ctx1", None).unwrap();
    let c2 = runtime.create_context("ctx2", None).unwrap();

    assert_ne!(c1, c2);
}

#[test]
fn test_23_concurrent_context_operations() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));

    let mut handles = Vec::new();
    for i in 0..10 {
        let r = Arc::clone(&runtime);
        handles.push(thread::spawn(move || {
            let cid = r.create_context(format!("worker_{i}"), None).unwrap();
            r.execute_context(cid).unwrap();
            r.get_context_state(cid).unwrap()
        }));
    }

    for h in handles {
        assert_eq!(h.join().unwrap(), ContextState::Completed);
    }
}

#[test]
fn test_24_arc_runtime_sharing() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    let runtime = Arc::new(Runtime::new(RuntimeConfig, kernel));

    let r2 = Arc::clone(&runtime);
    assert_eq!(r2.state(), RuntimeState::Uninitialized);
}

#[test]
fn test_25_kernel_integration() {
    let (runtime, kernel) = create_test_runtime();
    kernel.boot(None).unwrap();
    kernel.start(None).unwrap();

    let cid = runtime.create_context("k_task", None).unwrap();
    runtime.close_context(cid).unwrap();

    assert_eq!(
        kernel.get_process_state(kernel::ProcessId(cid.0)).unwrap(),
        kernel::ProcessState::Stopped
    );
}

#[test]
fn test_26_capability_failure() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("user", "CAP_OTHER", None).unwrap();

    let (runtime, _) = create_test_runtime();
    let err = runtime
        .create_context("ctx", Some((&registry, &token)))
        .unwrap_err();

    assert!(err.to_string().contains("CAP_RUNTIME_EXECUTE"));
}

#[test]
fn test_27_eventbus_integration() {
    let bus = EventBus::new(EventBusConfig);
    let init_count = Arc::new(AtomicU32::new(0));
    let ready_count = Arc::new(AtomicU32::new(0));
    let shutdown_count = Arc::new(AtomicU32::new(0));

    bus.subscribe(
        "runtime.initializing",
        Box::new(TestHandler {
            count: init_count.clone(),
        }),
    )
    .unwrap();
    bus.subscribe(
        "runtime.ready",
        Box::new(TestHandler {
            count: ready_count.clone(),
        }),
    )
    .unwrap();
    bus.subscribe(
        "runtime.shutdown",
        Box::new(TestHandler {
            count: shutdown_count.clone(),
        }),
    )
    .unwrap();

    let (runtime, _) = create_test_runtime();
    runtime.initialize(Some(&bus)).unwrap();
    runtime.start(Some(&bus)).unwrap();
    runtime.shutdown(Some(&bus)).unwrap();

    assert_eq!(init_count.load(Ordering::SeqCst), 1);
    assert_eq!(ready_count.load(Ordering::SeqCst), 1);
    assert_eq!(shutdown_count.load(Ordering::SeqCst), 1);
}
