//! Unified runtime execution framework for NAINA OS.

use crate::config::RuntimeConfig;
use crate::error::{Result, RuntimeError};
use crate::types::{ContextState, ExecutionContext, ExecutionContextId, RuntimeState};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unified runtime execution framework.
#[derive(Debug)]
pub struct Runtime {
    _config: RuntimeConfig,
    kernel: Arc<kernel::Kernel>,
    state: RwLock<RuntimeState>,
    contexts: RwLock<BTreeMap<ExecutionContextId, ExecutionContext>>,
    next_context_id: AtomicU64,
}

impl Runtime {
    /// Creates a new [`Runtime`] instance with the specified configuration and microkernel reference.
    pub fn new(config: RuntimeConfig, kernel: Arc<kernel::Kernel>) -> Self {
        Self {
            _config: config,
            kernel,
            state: RwLock::new(RuntimeState::Uninitialized),
            contexts: RwLock::new(BTreeMap::new()),
            next_context_id: AtomicU64::new(1),
        }
    }

    /// Creates a new [`Runtime`] instance with default microkernel initialization.
    pub fn with_default_kernel(config: RuntimeConfig) -> Self {
        let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
        Self::new(config, kernel)
    }

    /// Returns the current lifecycle state of the runtime framework.
    pub fn state(&self) -> RuntimeState {
        *self.state.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Initializes the runtime framework and emits `"runtime.initializing"`.
    pub fn initialize(&self, event_bus: Option<&kernel::event_bus::EventBus>) -> Result<()> {
        let mut current = self.state.write().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        if *current != RuntimeState::Uninitialized {
            return Err(RuntimeError::InvalidState {
                current: format!("{:?}", *current),
                expected: "Uninitialized".to_string(),
            });
        }

        *current = RuntimeState::Initializing;

        if let Some(bus) = event_bus {
            let mut payload = BTreeMap::new();
            payload.insert("state".to_string(), "initializing".to_string());
            let event = kernel::event_bus::Event::new("runtime.initializing", "runtime")
                .with_payload(payload);
            let _ = bus.publish(&event);
        }

        Ok(())
    }

    /// Starts the runtime framework, transitioning to `Ready` and emitting `"runtime.ready"`.
    pub fn start(&self, event_bus: Option<&kernel::event_bus::EventBus>) -> Result<()> {
        let mut current = self.state.write().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        if *current != RuntimeState::Initializing {
            return Err(RuntimeError::InvalidState {
                current: format!("{:?}", *current),
                expected: "Initializing".to_string(),
            });
        }

        *current = RuntimeState::Ready;

        if let Some(bus) = event_bus {
            let mut payload = BTreeMap::new();
            payload.insert("state".to_string(), "ready".to_string());
            let event =
                kernel::event_bus::Event::new("runtime.ready", "runtime").with_payload(payload);
            let _ = bus.publish(&event);
        }

        Ok(())
    }

    /// Shuts down the runtime framework and closes all active execution contexts.
    pub fn shutdown(&self, event_bus: Option<&kernel::event_bus::EventBus>) -> Result<()> {
        let mut current = self.state.write().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        if *current == RuntimeState::Stopped {
            return Ok(());
        }

        *current = RuntimeState::ShuttingDown;

        if let Some(bus) = event_bus {
            let mut payload = BTreeMap::new();
            payload.insert("state".to_string(), "shutdown".to_string());
            let event =
                kernel::event_bus::Event::new("runtime.shutdown", "runtime").with_payload(payload);
            let _ = bus.publish(&event);
        }

        let mut contexts_map = self.contexts.write().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        for ctx in contexts_map.values_mut() {
            let _ = self.kernel.stop_process(ctx.kernel_process_id);
            ctx.state = ContextState::Closed;
        }

        *current = RuntimeState::Stopped;
        Ok(())
    }

    /// Creates a new [`ExecutionContext`] and registers an underlying process with `Kernel`.
    ///
    /// Authorizes `CAP_RUNTIME_EXECUTE` when `capability_registry` and token are supplied.
    pub fn create_context(
        &self,
        name: impl Into<String>,
        capability_registry: Option<(
            &kernel::capabilities::CapabilityRegistry,
            &kernel::capabilities::CapabilityToken,
        )>,
    ) -> Result<ExecutionContextId> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_RUNTIME_EXECUTE")
                .map_err(|err| match err {
                    kernel::capabilities::CapabilityError::Unauthorized {
                        capability_id, ..
                    } => RuntimeError::Unauthorized { capability_id },
                    other => RuntimeError::Kernel(kernel::KernelError::Capability(other)),
                })?;
        }

        let name_str = name.into();
        let k_pid = self.kernel.register_process(
            &name_str,
            kernel::RestartPolicy::OnFailure { max_retries: 3 },
            None,
        )?;

        let cid = ExecutionContextId(self.next_context_id.fetch_add(1, Ordering::SeqCst));
        let ctx = ExecutionContext {
            id: cid,
            name: name_str,
            state: ContextState::Created,
            kernel_process_id: k_pid,
        };

        let mut contexts_map = self.contexts.write().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        contexts_map.insert(cid, ctx);
        Ok(cid)
    }

    /// Executes a created context, transitioning its state to `Running` and then `Completed`.
    pub fn execute_context(&self, id: ExecutionContextId) -> Result<()> {
        let mut contexts_map = self.contexts.write().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        let ctx = contexts_map
            .get_mut(&id)
            .ok_or(RuntimeError::ContextNotFound { id })?;

        match ctx.state {
            ContextState::Created => {
                ctx.state = ContextState::Running;
                ctx.state = ContextState::Completed;
                Ok(())
            }
            ContextState::Running => {
                ctx.state = ContextState::Completed;
                Ok(())
            }
            ref other => Err(RuntimeError::ContextFailed {
                id,
                message: format!("Cannot execute context in state {:?}", other),
            }),
        }
    }

    /// Closes an active execution context and stops its corresponding Kernel-supervised process.
    pub fn close_context(&self, id: ExecutionContextId) -> Result<()> {
        let mut contexts_map = self.contexts.write().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        let ctx = contexts_map
            .get_mut(&id)
            .ok_or(RuntimeError::ContextNotFound { id })?;

        let _ = self.kernel.stop_process(ctx.kernel_process_id);
        ctx.state = ContextState::Closed;
        Ok(())
    }

    /// Returns the current state of an execution context.
    pub fn get_context_state(&self, id: ExecutionContextId) -> Result<ContextState> {
        let contexts_map = self.contexts.read().map_err(|e| RuntimeError::LockError {
            message: e.to_string(),
        })?;

        contexts_map
            .get(&id)
            .map(|c| c.state.clone())
            .ok_or(RuntimeError::ContextNotFound { id })
    }
}
