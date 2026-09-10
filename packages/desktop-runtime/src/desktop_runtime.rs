//! Primary DesktopRuntime supervisor implementation for NAINA OS.

use crate::config::DesktopRuntimeConfig;
use crate::error::{DesktopRuntimeError, Result};
use crate::platform::Win32DesktopEngine;
use crate::traits::DesktopAutomationEngine;
use crate::types::{AppLaunchRequest, AppLaunchResult, DesktopState, UiElement, WindowInfo};
use logging::{LogLevel, Logger, LoggerConfig};
use runtime::Runtime;
use services::ServiceRegistry;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

/// Primary top-level desktop overlay and system automation runtime supervisor.
pub struct DesktopRuntime {
    config: DesktopRuntimeConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    state: RwLock<DesktopState>,
    engine: RwLock<Arc<dyn DesktopAutomationEngine>>,
    cancel_flag: Arc<AtomicBool>,
    logger: Mutex<Logger>,
}

unsafe impl Send for DesktopRuntime {}
unsafe impl Sync for DesktopRuntime {}

impl fmt::Debug for DesktopRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopRuntime")
            .field("config", &self.config)
            .field("runtime", &self.runtime)
            .field("services", &self.services)
            .field("state", &self.state)
            .field("cancel_flag", &self.cancel_flag)
            .finish()
    }
}

impl DesktopRuntime {
    /// Constructs a new [`DesktopRuntime`] instance.
    pub fn new(
        config: DesktopRuntimeConfig,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let default_engine: Arc<dyn DesktopAutomationEngine> = Arc::new(Win32DesktopEngine::new());
        let logger = Logger::new(LoggerConfig::default())
            .unwrap()
            .with_component("desktop-runtime");

        Self {
            config,
            runtime,
            services,
            state: RwLock::new(DesktopState::Idle),
            engine: RwLock::new(default_engine),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            logger: Mutex::new(logger),
        }
    }

    /// Constructs a [`DesktopRuntime`] with default configuration.
    pub fn with_default_config(runtime: Arc<Runtime>, services: Arc<ServiceRegistry>) -> Self {
        Self::new(DesktopRuntimeConfig::default(), runtime, services)
    }

    /// Constructs a [`DesktopRuntime`] from root [`configuration::Config`].
    pub fn from_root_config(
        root_config: &configuration::Config,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let config = DesktopRuntimeConfig::from_desktop_config(&root_config.desktop);
        Self::new(config, runtime, services)
    }

    /// Registers a custom desktop automation engine implementation.
    pub fn register_engine(&self, engine: Arc<dyn DesktopAutomationEngine>) {
        if let Ok(mut lock) = self.engine.write() {
            *lock = engine;
        }
    }

    /// Returns the active configuration.
    pub fn config(&self) -> &DesktopRuntimeConfig {
        &self.config
    }

    /// Returns the current desktop runtime state.
    pub fn state(&self) -> DesktopState {
        self.state.read().map(|s| *s).unwrap_or(DesktopState::Error)
    }

    /// Sets the desktop state safely.
    pub fn set_state(&self, new_state: DesktopState) -> Result<()> {
        let mut lock = self
            .state
            .write()
            .map_err(|_| DesktopRuntimeError::LockError {
                message: "Failed to acquire state write lock".to_string(),
            })?;
        *lock = new_state;
        Ok(())
    }

    /// Returns a reference to the inner runtime handle.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Returns a reference to the inner service registry handle.
    pub fn services(&self) -> &Arc<ServiceRegistry> {
        &self.services
    }

    /// Launches a native Windows application process.
    pub fn launch_app(&self, request: AppLaunchRequest) -> Result<AppLaunchResult> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(DesktopRuntimeError::AppLaunchFailed {
                message: "Operation cancelled".to_string(),
            });
        }

        self.set_state(DesktopState::LaunchingApp)?;

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| DesktopRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.launch_app(&request) {
            Ok(res) => {
                self.set_state(DesktopState::Idle)?;
                self.check_latency_target(start_time, "launch_app");
                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(DesktopState::Error);
                Err(err)
            }
        }
    }

    /// Enumerates active top-level desktop windows.
    pub fn enumerate_windows(&self) -> Result<Vec<WindowInfo>> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(DesktopRuntimeError::LockError {
                message: "Operation cancelled".to_string(),
            });
        }

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| DesktopRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.enumerate_windows() {
            Ok(res) => {
                self.check_latency_target(start_time, "enumerate_windows");
                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(DesktopState::Error);
                Err(err)
            }
        }
    }

    /// Inspects semantic UI elements within a target window handle.
    pub fn inspect_ui_elements(&self, window_handle: u64) -> Result<Vec<UiElement>> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(DesktopRuntimeError::LockError {
                message: "Operation cancelled".to_string(),
            });
        }

        self.set_state(DesktopState::InspectingUi)?;

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| DesktopRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.inspect_ui_elements(
            window_handle,
            self.config.max_search_depth,
            self.config.max_element_count,
        ) {
            Ok(res) => {
                self.set_state(DesktopState::Idle)?;
                self.check_latency_target(start_time, "inspect_ui_elements");
                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(DesktopState::Error);
                Err(err)
            }
        }
    }

    /// Inject synthetic input events into a target window.
    pub fn inject_input(&self, window_handle: u64, action: &str) -> Result<()> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(DesktopRuntimeError::InputInjectionFailed {
                message: "Operation cancelled".to_string(),
            });
        }

        self.set_state(DesktopState::InjectingInput)?;

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| DesktopRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.inject_input(window_handle, action) {
            Ok(()) => {
                self.set_state(DesktopState::Idle)?;
                self.check_latency_target(start_time, "inject_input");
                Ok(())
            }
            Err(err) => {
                let _ = self.set_state(DesktopState::Error);
                Err(err)
            }
        }
    }

    /// Cancels any active or pending desktop operation.
    pub fn cancel_operation(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Resets the desktop runtime state back to `Idle` after error or cancellation.
    pub fn reset_state(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
        let _ = self.set_state(DesktopState::Idle);
    }

    fn check_latency_target(&self, start_time: Instant, op_name: &str) {
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        if elapsed_ms > self.config.latency_target_ms {
            if let Ok(logger) = self.logger.lock() {
                let _ = logger.log(
                    LogLevel::Warn,
                    format!(
                        "Desktop command '{}' latency target exceeded: {}ms (target: {}ms)",
                        op_name, elapsed_ms, self.config.latency_target_ms
                    ),
                );
            }
        }
    }
}
