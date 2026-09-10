//! Primary SDKFacade client routing interface implementation for NAINA OS.

use crate::builder::SDKBuilder;
use crate::config::SDKConfig;
use crate::error::{SDKError, SDKResult};
use crate::session::SDKSession;
use crate::types::SDKState;
use logging::{LogLevel, Logger};
use runtime::Runtime;
use services::ServiceRegistry;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

/// Primary client-facing facade interface for NAINA OS applications and tooling.
pub struct SDKFacade {
    config: SDKConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    sessions: RwLock<HashMap<String, SDKSession>>,
    logger: Mutex<Logger>,
    state: RwLock<SDKState>,
}

unsafe impl Send for SDKFacade {}
unsafe impl Sync for SDKFacade {}

impl fmt::Debug for SDKFacade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SDKFacade")
            .field("config", &self.config)
            .field("runtime", &self.runtime)
            .field("services", &self.services)
            .field("state", &self.state)
            .finish()
    }
}

impl SDKFacade {
    /// Constructs a new [`SDKFacade`] instance.
    pub fn new(
        config: SDKConfig,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
        logger: Logger,
    ) -> Self {
        Self {
            config,
            runtime,
            services,
            sessions: RwLock::new(HashMap::new()),
            logger: Mutex::new(logger),
            state: RwLock::new(SDKState::Ready),
        }
    }

    /// Returns a new [`SDKBuilder`] for constructing an `SDKFacade`.
    pub fn builder() -> SDKBuilder {
        SDKBuilder::new()
    }

    /// Returns a reference to the active configuration.
    pub fn config(&self) -> &SDKConfig {
        &self.config
    }

    /// Returns the current lifecycle state of the SDK facade.
    pub fn state(&self) -> SDKState {
        self.state.read().map(|s| *s).unwrap_or(SDKState::Shutdown)
    }

    /// Returns a reference to the inner microkernel runtime handle.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Returns a reference to the inner service registry handle.
    pub fn services(&self) -> &Arc<ServiceRegistry> {
        &self.services
    }

    /// Creates a new client session bound to a microkernel execution context.
    pub fn create_session(&self, session_id: &str) -> SDKResult<SDKSession> {
        if session_id.trim().is_empty() {
            return Err(SDKError::InitializationFailed {
                message: "Session ID cannot be empty".to_string(),
            });
        }

        // Delegate context creation to microkernel Runtime
        let context_id = self.runtime.create_context(session_id, None)?;
        let created_at_ms = 1_000; // Deterministic initial timestamp
        let session = SDKSession::new(session_id.to_string(), context_id, created_at_ms);

        let mut lock = self.sessions.write().map_err(|_| SDKError::LockError {
            message: "Failed to acquire sessions write lock".to_string(),
        })?;

        lock.insert(session_id.to_string(), session.clone());

        if let Ok(logger) = self.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                format!("Created new SDK session '{session_id}' bound to context {context_id:?}"),
            );
        }

        Ok(session)
    }

    /// Retrieves an active client session by ID.
    pub fn get_session(&self, session_id: &str) -> SDKResult<SDKSession> {
        let lock = self.sessions.read().map_err(|_| SDKError::LockError {
            message: "Failed to acquire sessions read lock".to_string(),
        })?;

        let session = lock
            .get(session_id)
            .cloned()
            .ok_or_else(|| SDKError::SessionNotFound {
                session_id: session_id.to_string(),
            })?;

        if !session.is_active {
            return Err(SDKError::SessionExpired {
                session_id: session_id.to_string(),
            });
        }

        Ok(session)
    }

    /// Closes an active client session and releases microkernel context resources.
    pub fn close_session(&self, session_id: &str) -> SDKResult<()> {
        let mut lock = self.sessions.write().map_err(|_| SDKError::LockError {
            message: "Failed to acquire sessions write lock".to_string(),
        })?;

        if let Some(mut session) = lock.remove(session_id) {
            session.is_active = false;
            let _ = self.runtime.close_context(session.context_id);

            if let Ok(logger) = self.logger.lock() {
                let _ = logger.log(LogLevel::Info, format!("Closed SDK session '{session_id}'"));
            }
            Ok(())
        } else {
            Err(SDKError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Cleans up expired sessions based on configured session timeout duration.
    pub fn cleanup_expired_sessions(&self, current_time_ms: u64) {
        if let Ok(mut lock) = self.sessions.write() {
            let timeout = self.config.session_timeout_ms;
            lock.retain(|_, session| !session.is_expired(current_time_ms, timeout));
        }
    }

    /// Shuts down the SDK facade and underlying microkernel runtime.
    pub fn shutdown(&self) -> SDKResult<()> {
        let mut state_lock = self.state.write().map_err(|_| SDKError::LockError {
            message: "Failed to acquire state write lock".to_string(),
        })?;

        if *state_lock == SDKState::Shutdown {
            return Ok(());
        }

        // Close all active sessions
        if let Ok(mut lock) = self.sessions.write() {
            for (_, mut session) in lock.drain() {
                session.is_active = false;
                let _ = self.runtime.close_context(session.context_id);
            }
        }

        self.runtime.shutdown(None)?;
        *state_lock = SDKState::Shutdown;

        if let Ok(logger) = self.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                "SDKFacade shut down successfully".to_string(),
            );
        }

        Ok(())
    }
}
