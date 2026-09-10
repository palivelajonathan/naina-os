//! Primary UIFramework implementation for NAINA OS.

use crate::builder::UIBuilder;
use crate::config::UIConfig;
use crate::error::{UIError, UIResult};
use crate::types::{UIEvent, UIState, WindowRecord};
use logging::{LogLevel, Logger};
use runtime::{ExecutionContextId, Runtime};
use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};

/// Primary client-side UI framework interface for NAINA OS desktop host applications.
pub struct UIFramework {
    config: UIConfig,
    runtime: Arc<Runtime>,
    logger: Mutex<Logger>,
    state: RwLock<UIState>,
    windows: RwLock<HashMap<String, WindowRecord>>,
    context_id: RwLock<Option<ExecutionContextId>>,
    event_sender: Sender<UIEvent>,
    event_receiver: Mutex<Receiver<UIEvent>>,
}

unsafe impl Send for UIFramework {}
unsafe impl Sync for UIFramework {}

impl fmt::Debug for UIFramework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UIFramework")
            .field("config", &self.config)
            .field("runtime", &self.runtime)
            .field("state", &self.state)
            .field("context_id", &self.context_id)
            .finish()
    }
}

impl UIFramework {
    /// Constructs a new [`UIFramework`] instance.
    pub fn new(config: UIConfig, runtime: Arc<Runtime>, logger: Logger) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            config,
            runtime,
            logger: Mutex::new(logger),
            state: RwLock::new(UIState::Uninitialized),
            windows: RwLock::new(HashMap::new()),
            context_id: RwLock::new(None),
            event_sender: tx,
            event_receiver: Mutex::new(rx),
        }
    }

    /// Returns a new [`UIBuilder`] instance.
    pub fn builder() -> UIBuilder {
        UIBuilder::new()
    }

    /// Returns a reference to the active configuration.
    pub fn config(&self) -> &UIConfig {
        &self.config
    }

    /// Returns the current lifecycle state of the UI framework.
    pub fn state(&self) -> UIState {
        self.state.read().map(|s| *s).unwrap_or(UIState::Shutdown)
    }

    /// Returns a reference to the inner microkernel runtime handle.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Initializes the UI framework and creates the `"ui_overlay_host"` execution context.
    pub fn initialize(&self) -> UIResult<()> {
        let mut state_lock = self.state.write().map_err(|_| UIError::LockError {
            message: "Failed to acquire state write lock".to_string(),
        })?;

        if *state_lock != UIState::Uninitialized {
            return Err(UIError::InitializationFailed {
                message: format!("Invalid transition from state {:?}", *state_lock),
            });
        }

        *state_lock = UIState::Initializing;

        // Register dedicated microkernel runtime execution context
        let cid = self.runtime.create_context("ui_overlay_host", None)?;

        if let Ok(mut ctx_lock) = self.context_id.write() {
            *ctx_lock = Some(cid);
        }

        let main_window = WindowRecord::new(
            "overlay-main".to_string(),
            self.config.title.clone(),
            self.config.width,
            self.config.height,
        );

        if let Ok(mut win_lock) = self.windows.write() {
            win_lock.insert("overlay-main".to_string(), main_window);
        }

        *state_lock = UIState::Ready;

        if let Ok(logger) = self.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                format!("UIFramework initialized with execution context {cid:?}"),
            );
        }

        if self.config.auto_show {
            drop(state_lock);
            self.show_overlay()?;
        }

        Ok(())
    }

    /// Transitions the overlay host to `OverlayVisible` state and emits `UIEvent::OverlayShown`.
    pub fn show_overlay(&self) -> UIResult<()> {
        let mut state_lock = self.state.write().map_err(|_| UIError::LockError {
            message: "Failed to acquire state write lock".to_string(),
        })?;

        match *state_lock {
            UIState::Ready | UIState::OverlayHidden => {
                if let Ok(mut win_lock) = self.windows.write() {
                    if let Some(win) = win_lock.get_mut("overlay-main") {
                        win.is_visible = true;
                    }
                }

                *state_lock = UIState::OverlayVisible;
                let _ = self.event_sender.send(UIEvent::OverlayShown);

                if let Ok(logger) = self.logger.lock() {
                    let _ = logger.log(
                        LogLevel::Info,
                        "Desktop overlay host window shown".to_string(),
                    );
                }
                Ok(())
            }
            UIState::OverlayVisible => Ok(()),
            other => Err(UIError::RenderError {
                message: format!("Cannot show overlay from state {:?}", other),
            }),
        }
    }

    /// Transitions the overlay host to `OverlayHidden` state and emits `UIEvent::OverlayHidden`.
    pub fn hide_overlay(&self) -> UIResult<()> {
        let mut state_lock = self.state.write().map_err(|_| UIError::LockError {
            message: "Failed to acquire state write lock".to_string(),
        })?;

        match *state_lock {
            UIState::OverlayVisible => {
                if let Ok(mut win_lock) = self.windows.write() {
                    if let Some(win) = win_lock.get_mut("overlay-main") {
                        win.is_visible = false;
                    }
                }

                *state_lock = UIState::OverlayHidden;
                let _ = self.event_sender.send(UIEvent::OverlayHidden);

                if let Ok(logger) = self.logger.lock() {
                    let _ = logger.log(
                        LogLevel::Info,
                        "Desktop overlay host window hidden".to_string(),
                    );
                }
                Ok(())
            }
            UIState::OverlayHidden | UIState::Ready => Ok(()),
            other => Err(UIError::RenderError {
                message: format!("Cannot hide overlay from state {:?}", other),
            }),
        }
    }

    /// Dispatches a custom [`UIEvent`] into the channel event queue.
    pub fn dispatch_event(&self, event: UIEvent) -> UIResult<()> {
        self.event_sender
            .send(event)
            .map_err(|_| UIError::RenderError {
                message: "Failed to dispatch UI event over channel".to_string(),
            })
    }

    /// Non-blocking attempt to receive the next [`UIEvent`] from the channel queue.
    pub fn try_recv_event(&self) -> Option<UIEvent> {
        if let Ok(receiver) = self.event_receiver.lock() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }

    /// Shuts down the UI framework and closes the underlying microkernel context.
    pub fn shutdown(&self) -> UIResult<()> {
        let mut state_lock = self.state.write().map_err(|_| UIError::LockError {
            message: "Failed to acquire state write lock".to_string(),
        })?;

        if *state_lock == UIState::Shutdown {
            return Ok(());
        }

        if let Ok(mut win_lock) = self.windows.write() {
            for win in win_lock.values_mut() {
                win.is_visible = false;
            }
        }

        let cid_opt = self.context_id.write().ok().and_then(|mut c| c.take());
        if let Some(cid) = cid_opt {
            let _ = self.runtime.close_context(cid);
        }

        *state_lock = UIState::Shutdown;

        if let Ok(logger) = self.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                "UIFramework shut down successfully".to_string(),
            );
        }

        Ok(())
    }
}
