//! Microkernel process supervisor and lifecycle management for NAINA OS.

use crate::config::KernelConfig;
use crate::error::{KernelError, Result};
use crate::types::{KernelState, ProcessId, ProcessRecord, ProcessState, RestartPolicy};
use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Microkernel NKRS process supervisor and lifecycle manager.
#[derive(Debug)]
pub struct Kernel {
    config: KernelConfig,
    state: RwLock<KernelState>,
    processes: RwLock<BTreeMap<ProcessId, ProcessRecord>>,
    next_process_id: AtomicU64,
}

impl Kernel {
    /// Creates a new [`Kernel`] supervisor instance with the specified configuration.
    pub fn new(config: KernelConfig) -> Self {
        Self {
            config,
            state: RwLock::new(KernelState::Uninitialized),
            processes: RwLock::new(BTreeMap::new()),
            next_process_id: AtomicU64::new(1),
        }
    }

    /// Returns the current lifecycle state of the kernel.
    pub fn state(&self) -> KernelState {
        *self.state.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Boots the microkernel supervisor, validating configuration and emitting `"system.booting"`.
    pub fn boot(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()> {
        let mut current = self.state.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        if *current != KernelState::Uninitialized {
            return Err(KernelError::InvalidState {
                current: format!("{:?}", *current),
                expected: "Uninitialized".to_string(),
            });
        }

        if self.config.boot_timeout > std::time::Duration::from_secs(2) {
            *current = KernelState::Failed;
            return Err(KernelError::BootFailed {
                message: format!(
                    "Boot timeout {:?} exceeds 2.0s cold boot budget",
                    self.config.boot_timeout
                ),
            });
        }

        *current = KernelState::Booting;

        if let Some(bus) = event_bus {
            let mut payload = BTreeMap::new();
            payload.insert("state".to_string(), "booting".to_string());
            let event = event_bus::Event::new("system.booting", "kernel").with_payload(payload);
            bus.publish(&event)?;
        }

        Ok(())
    }

    /// Starts the microkernel supervisor, transitioning state to `Running` and emitting `"system.started"`.
    pub fn start(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()> {
        let mut current = self.state.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        if *current != KernelState::Booting {
            return Err(KernelError::InvalidState {
                current: format!("{:?}", *current),
                expected: "Booting".to_string(),
            });
        }

        *current = KernelState::Running;

        let mut procs = self.processes.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        for record in procs.values_mut() {
            if record.state == ProcessState::Registered || record.state == ProcessState::Starting {
                record.state = ProcessState::Running;
            }
        }

        if let Some(bus) = event_bus {
            let mut payload = BTreeMap::new();
            payload.insert("state".to_string(), "running".to_string());
            let event = event_bus::Event::new("system.started", "kernel").with_payload(payload);
            bus.publish(&event)?;
        }

        Ok(())
    }

    /// Shuts down the microkernel supervisor and all registered processes.
    pub fn shutdown(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()> {
        let mut current = self.state.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        if *current == KernelState::Stopped {
            return Ok(());
        }

        *current = KernelState::ShuttingDown;

        if let Some(bus) = event_bus {
            let mut payload = BTreeMap::new();
            payload.insert("state".to_string(), "shutdown".to_string());
            let event = event_bus::Event::new("system.shutdown", "kernel").with_payload(payload);
            let _ = bus.publish(&event);
        }

        let mut procs = self.processes.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        for record in procs.values_mut() {
            record.state = ProcessState::Stopped;
        }

        *current = KernelState::Stopped;
        Ok(())
    }

    /// Registers a new process with the supervisor under a specified [`RestartPolicy`].
    ///
    /// If `capability_registry` is provided as `Some((registry, token))`, it is authorized
    /// against `CAP_KERNEL_SUPERVISE`.
    pub fn register_process(
        &self,
        name: impl Into<String>,
        policy: RestartPolicy,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<ProcessId> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_KERNEL_SUPERVISE")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        KernelError::Unauthorized { capability_id }
                    }
                    other => KernelError::Capability(other),
                })?;
        }

        let id_val = self.next_process_id.fetch_add(1, Ordering::SeqCst);
        let id = ProcessId(id_val);

        let current_state = self.state();
        let initial_process_state = if current_state == KernelState::Running {
            ProcessState::Running
        } else {
            ProcessState::Registered
        };

        let record = ProcessRecord {
            id,
            name: name.into(),
            state: initial_process_state,
            policy,
            restart_count: 0,
        };

        let mut procs = self.processes.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        procs.insert(id, record);
        Ok(id)
    }

    /// Stops a registered process by its [`ProcessId`].
    pub fn stop_process(&self, id: ProcessId) -> Result<bool> {
        let mut procs = self.processes.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        if let Some(record) = procs.get_mut(&id) {
            record.state = ProcessState::Stopped;
            Ok(true)
        } else {
            Err(KernelError::ProcessNotFound { id })
        }
    }

    /// Returns the current state of a registered process.
    pub fn get_process_state(&self, id: ProcessId) -> Result<ProcessState> {
        let procs = self.processes.read().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        procs
            .get(&id)
            .map(|r| r.state.clone())
            .ok_or(KernelError::ProcessNotFound { id })
    }

    /// Reports a process failure and executes automatic crash recovery according to its [`RestartPolicy`].
    ///
    /// Automatic crash recovery restarts worker processes without crashing the `Kernel` supervisor
    /// (`FIRST_ALPHA_SPEC.md` Acceptance Criterion #10).
    pub fn report_process_failure(
        &self,
        id: ProcessId,
        error: impl Into<String>,
        event_bus: Option<&event_bus::EventBus>,
    ) -> Result<()> {
        let err_msg = error.into();
        let mut procs = self.processes.write().map_err(|e| KernelError::LockError {
            message: e.to_string(),
        })?;

        let record = procs
            .get_mut(&id)
            .ok_or(KernelError::ProcessNotFound { id })?;

        match record.policy {
            RestartPolicy::OnFailure { max_retries } => {
                if record.restart_count < max_retries {
                    record.restart_count += 1;
                    record.state = ProcessState::Running;
                    Ok(())
                } else {
                    record.state = ProcessState::Failed {
                        error: err_msg.clone(),
                    };
                    if let Some(bus) = event_bus {
                        let mut payload = BTreeMap::new();
                        payload.insert("process_id".to_string(), id.0.to_string());
                        payload.insert("error".to_string(), err_msg);
                        let event =
                            event_bus::Event::new("process.failed", "kernel").with_payload(payload);
                        let _ = bus.publish(&event);
                    }
                    Err(KernelError::ProcessFailed {
                        id,
                        message: format!("Exhausted max retries ({max_retries})"),
                    })
                }
            }
            RestartPolicy::Never => {
                record.state = ProcessState::Failed {
                    error: err_msg.clone(),
                };
                if let Some(bus) = event_bus {
                    let mut payload = BTreeMap::new();
                    payload.insert("process_id".to_string(), id.0.to_string());
                    payload.insert("error".to_string(), err_msg.clone());
                    let event =
                        event_bus::Event::new("process.failed", "kernel").with_payload(payload);
                    let _ = bus.publish(&event);
                }
                Err(KernelError::ProcessFailed {
                    id,
                    message: err_msg,
                })
            }
        }
    }
}
