//! Primary BrowserRuntime supervisor implementation for NAINA OS.

use crate::config::BrowserRuntimeConfig;
use crate::error::{BrowserRuntimeError, Result};
use crate::platform::CdpBrowserEngine;
use crate::traits::BrowserAutomationEngine;
use crate::types::{BrowserActionResult, BrowserState, DomNode, PageInfo};
use logging::{LogLevel, Logger, LoggerConfig};
use runtime::Runtime;
use services::ServiceRegistry;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

/// Primary top-level browser automation runtime supervisor.
pub struct BrowserRuntime {
    config: BrowserRuntimeConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    state: RwLock<BrowserState>,
    engine: RwLock<Arc<dyn BrowserAutomationEngine>>,
    cancel_flag: Arc<AtomicBool>,
    logger: Mutex<Logger>,
}

unsafe impl Send for BrowserRuntime {}
unsafe impl Sync for BrowserRuntime {}

impl fmt::Debug for BrowserRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrowserRuntime")
            .field("config", &self.config)
            .field("runtime", &self.runtime)
            .field("services", &self.services)
            .field("state", &self.state)
            .field("cancel_flag", &self.cancel_flag)
            .finish()
    }
}

impl BrowserRuntime {
    /// Constructs a new [`BrowserRuntime`] supervisor instance.
    pub fn new(
        config: BrowserRuntimeConfig,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let default_engine: Arc<dyn BrowserAutomationEngine> = Arc::new(CdpBrowserEngine::new());
        let logger = Logger::new(LoggerConfig::default())
            .unwrap()
            .with_component("browser-runtime");

        Self {
            config,
            runtime,
            services,
            state: RwLock::new(BrowserState::Idle),
            engine: RwLock::new(default_engine),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            logger: Mutex::new(logger),
        }
    }

    /// Constructs a [`BrowserRuntime`] with default configuration.
    pub fn with_default_config(runtime: Arc<Runtime>, services: Arc<ServiceRegistry>) -> Self {
        Self::new(BrowserRuntimeConfig::default(), runtime, services)
    }

    /// Constructs a [`BrowserRuntime`] from root [`configuration::Config`].
    pub fn from_root_config(
        root_config: &configuration::Config,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        let config = BrowserRuntimeConfig::from_browser_config(&root_config.browser);
        Self::new(config, runtime, services)
    }

    /// Registers a custom browser automation engine implementation.
    pub fn register_engine(&self, engine: Arc<dyn BrowserAutomationEngine>) {
        if let Ok(mut lock) = self.engine.write() {
            *lock = engine;
        }
    }

    /// Returns the active configuration.
    pub fn config(&self) -> &BrowserRuntimeConfig {
        &self.config
    }

    /// Returns the current browser runtime state.
    pub fn state(&self) -> BrowserState {
        self.state.read().map(|s| *s).unwrap_or(BrowserState::Error)
    }

    /// Sets the browser state safely.
    pub fn set_state(&self, new_state: BrowserState) -> Result<()> {
        let mut lock = self
            .state
            .write()
            .map_err(|_| BrowserRuntimeError::LockError {
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

    /// Navigates the active browser tab to a specified URL.
    pub fn navigate_to(&self, url: &str) -> Result<BrowserActionResult> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(BrowserRuntimeError::NavigationFailed {
                url: url.to_string(),
                message: "Operation cancelled".to_string(),
            });
        }

        self.set_state(BrowserState::Navigating)?;

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| BrowserRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.navigate_to(url) {
            Ok(res) => {
                self.set_state(BrowserState::Idle)?;
                self.check_latency_target(start_time, "navigate_to");
                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(BrowserState::Error);
                Err(err)
            }
        }
    }

    /// Opens a new browser tab/target.
    pub fn open_tab(&self, url: &str) -> Result<PageInfo> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(BrowserRuntimeError::LockError {
                message: "Operation cancelled".to_string(),
            });
        }

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| BrowserRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.open_tab(url) {
            Ok(res) => {
                self.check_latency_target(start_time, "open_tab");
                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(BrowserState::Error);
                Err(err)
            }
        }
    }

    /// Closes a target browser tab by ID.
    pub fn close_tab(&self, target_id: &str) -> Result<()> {
        let start_time = Instant::now();

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| BrowserRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.close_tab(target_id) {
            Ok(()) => {
                self.check_latency_target(start_time, "close_tab");
                Ok(())
            }
            Err(err) => {
                let _ = self.set_state(BrowserState::Error);
                Err(err)
            }
        }
    }

    /// Inspects semantic DOM nodes within a target tab.
    pub fn inspect_dom(&self, target_id: &str) -> Result<Vec<DomNode>> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(BrowserRuntimeError::DomInspectionFailed {
                message: "Operation cancelled".to_string(),
            });
        }

        self.set_state(BrowserState::InspectingDom)?;

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| BrowserRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.inspect_dom(target_id) {
            Ok(res) => {
                self.set_state(BrowserState::Idle)?;
                self.check_latency_target(start_time, "inspect_dom");
                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(BrowserState::Error);
                Err(err)
            }
        }
    }

    /// Extracts clean page text payload from a target tab.
    pub fn extract_page_text(&self, target_id: &str) -> Result<String> {
        let start_time = Instant::now();

        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(BrowserRuntimeError::LockError {
                message: "Operation cancelled".to_string(),
            });
        }

        self.set_state(BrowserState::ExtractingText)?;

        let engine = {
            let lock = self
                .engine
                .read()
                .map_err(|_| BrowserRuntimeError::LockError {
                    message: "Failed to acquire engine read lock".to_string(),
                })?;
            Arc::clone(&*lock)
        };

        match engine.extract_page_text(target_id) {
            Ok(res) => {
                self.set_state(BrowserState::Idle)?;
                self.check_latency_target(start_time, "extract_page_text");
                Ok(res)
            }
            Err(err) => {
                let _ = self.set_state(BrowserState::Error);
                Err(err)
            }
        }
    }

    /// Cancels any active or pending browser operation.
    pub fn cancel_operation(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Resets the browser runtime state back to `Idle` after error or cancellation.
    pub fn reset_state(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
        let _ = self.set_state(BrowserState::Idle);
    }

    fn check_latency_target(&self, start_time: Instant, op_name: &str) {
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        if elapsed_ms > self.config.latency_target_ms {
            if let Ok(logger) = self.logger.lock() {
                let _ = logger.log(
                    LogLevel::Warn,
                    format!(
                        "Browser command '{}' latency target exceeded: {}ms (target: {}ms)",
                        op_name, elapsed_ms, self.config.latency_target_ms
                    ),
                );
            }
        }
    }
}
