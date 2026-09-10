//! User-space service registry for NAINA OS.

use crate::config::ServicesConfig;
use crate::error::{Result, ServicesError};
use crate::types::{ServiceHealth, ServiceId, ServiceName, ServiceRecord, ServiceState};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Central registry for managing user-space service registration and discovery.
#[derive(Debug)]
pub struct ServiceRegistry {
    _config: ServicesConfig,
    runtime: Arc<runtime::Runtime>,
    services: RwLock<BTreeMap<ServiceId, ServiceRecord>>,
    by_name: RwLock<BTreeMap<String, ServiceId>>,
    next_service_id: AtomicU64,
}

impl ServiceRegistry {
    /// Creates a new [`ServiceRegistry`] with the given configuration and runtime reference.
    pub fn new(config: ServicesConfig, runtime: Arc<runtime::Runtime>) -> Self {
        Self {
            _config: config,
            runtime,
            services: RwLock::new(BTreeMap::new()),
            by_name: RwLock::new(BTreeMap::new()),
            next_service_id: AtomicU64::new(1),
        }
    }

    /// Registers a new user-space service.
    ///
    /// Authorizes `CAP_SERVICE_REGISTER` when `capability_registry` and token are supplied.
    pub fn register(
        &self,
        name: impl Into<String>,
        execution_context_id: Option<runtime::ExecutionContextId>,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<ServiceId> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_SERVICE_REGISTER")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        ServicesError::Unauthorized { capability_id }
                    }
                    other => ServicesError::Capability(other),
                })?;
        }

        let name_str = name.into();

        let by_name_map = self.by_name.read().map_err(|e| ServicesError::LockError {
            message: e.to_string(),
        })?;

        if by_name_map.contains_key(&name_str) {
            return Err(ServicesError::ServiceAlreadyExists { name: name_str });
        }
        drop(by_name_map);

        let sid = ServiceId(self.next_service_id.fetch_add(1, Ordering::SeqCst));
        let record = ServiceRecord {
            id: sid,
            name: ServiceName(name_str.clone()),
            state: ServiceState::Registered,
            health: ServiceHealth::Healthy,
            execution_context_id,
        };

        let mut services_map = self
            .services
            .write()
            .map_err(|e| ServicesError::LockError {
                message: e.to_string(),
            })?;
        let mut by_name_map = self.by_name.write().map_err(|e| ServicesError::LockError {
            message: e.to_string(),
        })?;

        by_name_map.insert(name_str, sid);
        services_map.insert(sid, record);

        Ok(sid)
    }

    /// Discovers a service by its registered name.
    ///
    /// Authorizes `CAP_SERVICE_LOOKUP` when `capability_registry` and token are supplied.
    pub fn lookup(
        &self,
        name: &str,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<ServiceRecord> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_SERVICE_LOOKUP")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        ServicesError::Unauthorized { capability_id }
                    }
                    other => ServicesError::Capability(other),
                })?;
        }

        let by_name_map = self.by_name.read().map_err(|e| ServicesError::LockError {
            message: e.to_string(),
        })?;

        let sid = by_name_map
            .get(name)
            .copied()
            .ok_or_else(|| ServicesError::ServiceNotFound {
                name_or_id: name.to_string(),
            })?;

        drop(by_name_map);
        self.get_service(sid)
    }

    /// Retrieves a service record by its unique [`ServiceId`].
    pub fn get_service(&self, id: ServiceId) -> Result<ServiceRecord> {
        let services_map = self.services.read().map_err(|e| ServicesError::LockError {
            message: e.to_string(),
        })?;

        services_map
            .get(&id)
            .cloned()
            .ok_or_else(|| ServicesError::ServiceNotFound {
                name_or_id: id.0.to_string(),
            })
    }

    /// Unregisters a service by its [`ServiceId`].
    ///
    /// Authorizes `CAP_SERVICE_REGISTER` when `capability_registry` and token are supplied.
    pub fn unregister(
        &self,
        id: ServiceId,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<()> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_SERVICE_REGISTER")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        ServicesError::Unauthorized { capability_id }
                    }
                    other => ServicesError::Capability(other),
                })?;
        }

        let mut services_map = self
            .services
            .write()
            .map_err(|e| ServicesError::LockError {
                message: e.to_string(),
            })?;
        let mut by_name_map = self.by_name.write().map_err(|e| ServicesError::LockError {
            message: e.to_string(),
        })?;

        let record = services_map
            .remove(&id)
            .ok_or_else(|| ServicesError::ServiceNotFound {
                name_or_id: id.0.to_string(),
            })?;

        by_name_map.remove(&record.name.0);
        Ok(())
    }

    /// Performs a health check for a registered service.
    pub fn health_check(&self, id: ServiceId) -> Result<ServiceHealth> {
        let record = self.get_service(id)?;

        if let Some(ctx_id) = record.execution_context_id {
            let is_unhealthy = matches!(
                self.runtime.get_context_state(ctx_id),
                Ok(runtime::ContextState::Closed) | Ok(runtime::ContextState::Failed { .. })
            );
            if is_unhealthy {
                return Ok(ServiceHealth::Unhealthy {
                    reason: "Underlying execution context closed or failed".to_string(),
                });
            }
        }

        Ok(record.health)
    }

    /// Updates the health state of a registered service.
    pub fn update_health(&self, id: ServiceId, health: ServiceHealth) -> Result<()> {
        let mut services_map = self
            .services
            .write()
            .map_err(|e| ServicesError::LockError {
                message: e.to_string(),
            })?;

        let record = services_map
            .get_mut(&id)
            .ok_or_else(|| ServicesError::ServiceNotFound {
                name_or_id: id.0.to_string(),
            })?;

        record.health = health;
        Ok(())
    }

    /// Updates the lifecycle state of a registered service.
    pub fn update_state(&self, id: ServiceId, state: ServiceState) -> Result<()> {
        let mut services_map = self
            .services
            .write()
            .map_err(|e| ServicesError::LockError {
                message: e.to_string(),
            })?;

        let record = services_map
            .get_mut(&id)
            .ok_or_else(|| ServicesError::ServiceNotFound {
                name_or_id: id.0.to_string(),
            })?;

        record.state = state;
        Ok(())
    }
}
