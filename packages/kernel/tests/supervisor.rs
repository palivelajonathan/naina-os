use capabilities::{CapabilityConfig, CapabilityRegistry};
use event_bus::{EventBus, EventBusConfig, EventHandler};
use kernel::{Kernel, KernelConfig, KernelState, ProcessState, RestartPolicy};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

struct TestHandler {
    count: Arc<AtomicU32>,
}

impl EventHandler for TestHandler {
    fn handle(&self, _event: &event_bus::Event) -> event_bus::Result<()> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn test_01_kernel_construction() {
    let kernel = Kernel::new(KernelConfig::default());
    assert!(format!("{:?}", kernel).contains("Kernel"));
}

#[test]
fn test_02_initial_state() {
    let kernel = Kernel::new(KernelConfig::default());
    assert_eq!(kernel.state(), KernelState::Uninitialized);
}

#[test]
fn test_03_valid_boot_transition() {
    let kernel = Kernel::new(KernelConfig::default());
    assert!(kernel.boot(None).is_ok());
    assert_eq!(kernel.state(), KernelState::Booting);
}

#[test]
fn test_04_boot_event_publication() {
    let bus = EventBus::new(EventBusConfig);
    let counter = Arc::new(AtomicU32::new(0));
    let handler = TestHandler {
        count: counter.clone(),
    };
    bus.subscribe("system.booting", Box::new(handler)).unwrap();

    let kernel = Kernel::new(KernelConfig::default());
    kernel.boot(Some(&bus)).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_05_running_state() {
    let kernel = Kernel::new(KernelConfig::default());
    kernel.boot(None).unwrap();
    assert!(kernel.start(None).is_ok());
    assert_eq!(kernel.state(), KernelState::Running);
}

#[test]
fn test_06_invalid_lifecycle_transition() {
    let kernel = Kernel::new(KernelConfig::default());
    assert!(kernel.start(None).is_err());
}

#[test]
fn test_07_shutdown_transition() {
    let kernel = Kernel::new(KernelConfig::default());
    kernel.boot(None).unwrap();
    kernel.start(None).unwrap();
    assert!(kernel.shutdown(None).is_ok());
    assert_eq!(kernel.state(), KernelState::Stopped);
}

#[test]
fn test_08_shutdown_event() {
    let bus = EventBus::new(EventBusConfig);
    let counter = Arc::new(AtomicU32::new(0));
    let handler = TestHandler {
        count: counter.clone(),
    };
    bus.subscribe("system.shutdown", Box::new(handler)).unwrap();

    let kernel = Kernel::new(KernelConfig::default());
    kernel.boot(Some(&bus)).unwrap();
    kernel.start(Some(&bus)).unwrap();
    kernel.shutdown(Some(&bus)).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_09_process_registration() {
    let kernel = Kernel::new(KernelConfig::default());
    let pid = kernel
        .register_process("worker", RestartPolicy::Never, None)
        .unwrap();

    assert_eq!(
        kernel.get_process_state(pid).unwrap(),
        ProcessState::Registered
    );
}

#[test]
fn test_10_process_ids() {
    let kernel = Kernel::new(KernelConfig::default());
    let p1 = kernel
        .register_process("w1", RestartPolicy::Never, None)
        .unwrap();
    let p2 = kernel
        .register_process("w2", RestartPolicy::Never, None)
        .unwrap();

    assert_eq!(p1.0, 1);
    assert_eq!(p2.0, 2);
}

#[test]
fn test_11_process_state_transitions() {
    let kernel = Kernel::new(KernelConfig::default());
    let pid = kernel
        .register_process("worker", RestartPolicy::Never, None)
        .unwrap();

    kernel.boot(None).unwrap();
    kernel.start(None).unwrap();

    assert_eq!(
        kernel.get_process_state(pid).unwrap(),
        ProcessState::Running
    );
}

#[test]
fn test_12_process_failure() {
    let kernel = Kernel::new(KernelConfig::default());
    let pid = kernel
        .register_process("worker", RestartPolicy::Never, None)
        .unwrap();

    assert!(kernel.report_process_failure(pid, "crash", None).is_err());
    assert!(matches!(
        kernel.get_process_state(pid).unwrap(),
        ProcessState::Failed { .. }
    ));
}

#[test]
fn test_13_process_failed_event() {
    let bus = EventBus::new(EventBusConfig);
    let counter = Arc::new(AtomicU32::new(0));
    let handler = TestHandler {
        count: counter.clone(),
    };
    bus.subscribe("process.failed", Box::new(handler)).unwrap();

    let kernel = Kernel::new(KernelConfig::default());
    let pid = kernel
        .register_process("worker", RestartPolicy::Never, None)
        .unwrap();

    let _ = kernel.report_process_failure(pid, "crash", Some(&bus));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_14_restart_policy_on_failure() {
    let kernel = Kernel::new(KernelConfig::default());
    let pid = kernel
        .register_process("worker", RestartPolicy::OnFailure { max_retries: 2 }, None)
        .unwrap();

    assert!(kernel.report_process_failure(pid, "retry_1", None).is_ok());
    assert_eq!(
        kernel.get_process_state(pid).unwrap(),
        ProcessState::Running
    );
}

#[test]
fn test_15_max_retry_enforcement() {
    let kernel = Kernel::new(KernelConfig::default());
    let pid = kernel
        .register_process("worker", RestartPolicy::OnFailure { max_retries: 2 }, None)
        .unwrap();

    assert!(kernel.report_process_failure(pid, "retry_1", None).is_ok());
    assert!(kernel.report_process_failure(pid, "retry_2", None).is_ok());

    let res = kernel.report_process_failure(pid, "retry_3", None);
    assert!(res.is_err());
    assert!(matches!(
        kernel.get_process_state(pid).unwrap(),
        ProcessState::Failed { .. }
    ));
}

#[test]
fn test_16_crash_recovery_without_supervisor_crash() {
    let kernel = Kernel::new(KernelConfig::default());
    kernel.boot(None).unwrap();
    kernel.start(None).unwrap();

    let p1 = kernel
        .register_process("worker1", RestartPolicy::OnFailure { max_retries: 1 }, None)
        .unwrap();
    let p2 = kernel
        .register_process("worker2", RestartPolicy::Never, None)
        .unwrap();

    assert!(kernel.report_process_failure(p1, "err", None).is_ok());
    let _ = kernel.report_process_failure(p2, "err", None);

    assert_eq!(kernel.state(), KernelState::Running);
}

#[test]
fn test_17_capability_authorization() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry
        .grant("kernel_admin", "CAP_KERNEL_SUPERVISE", None)
        .unwrap();

    let kernel = Kernel::new(KernelConfig::default());
    let res = kernel.register_process(
        "auth_worker",
        RestartPolicy::Never,
        Some((&registry, &token)),
    );

    assert!(res.is_ok());
}

#[test]
fn test_18_unauthorized_supervisor_operation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("user", "CAP_DESKTOP_CONTROL", None).unwrap();

    let kernel = Kernel::new(KernelConfig::default());
    let res = kernel.register_process(
        "auth_worker",
        RestartPolicy::Never,
        Some((&registry, &token)),
    );

    assert!(res.is_err());
}

#[test]
fn test_19_eventbus_integration() {
    let bus = EventBus::new(EventBusConfig);
    let counter = Arc::new(AtomicU32::new(0));
    let handler = TestHandler {
        count: counter.clone(),
    };
    bus.subscribe("system.started", Box::new(handler)).unwrap();

    let kernel = Kernel::new(KernelConfig::default());
    kernel.boot(Some(&bus)).unwrap();
    kernel.start(Some(&bus)).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_20_concurrent_kernel_access() {
    let kernel = Arc::new(Kernel::new(KernelConfig::default()));
    kernel.boot(None).unwrap();
    kernel.start(None).unwrap();

    let mut handles = Vec::new();
    for i in 0..10 {
        let k = Arc::clone(&kernel);
        handles.push(thread::spawn(move || {
            let pid = k
                .register_process(format!("worker_{i}"), RestartPolicy::Never, None)
                .unwrap();
            k.get_process_state(pid).unwrap()
        }));
    }

    for h in handles {
        assert_eq!(h.join().unwrap(), ProcessState::Running);
    }
}

#[test]
fn test_21_error_propagation() {
    let kernel = Kernel::new(KernelConfig::default());
    let pid = kernel
        .register_process("w", RestartPolicy::Never, None)
        .unwrap();

    let err = kernel
        .report_process_failure(pid, "fatal_error", None)
        .unwrap_err();
    assert!(err.to_string().contains("fatal_error"));
}

#[test]
fn test_22_logging_state_transitions() {
    let logger = logging::Logger::new(logging::LoggerConfig::default()).unwrap();
    let kernel = Kernel::new(KernelConfig::default());

    let _ = logger.info("Booting kernel...");
    assert!(kernel.boot(None).is_ok());

    let _ = logger.info("Starting kernel...");
    assert!(kernel.start(None).is_ok());

    let _ = logger.info("Kernel state: Running");
    assert_eq!(kernel.state(), KernelState::Running);
}
