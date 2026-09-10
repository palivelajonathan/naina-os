//! Capability-controlled tool registry for NAINA OS.

use crate::config::ToolRegistryConfig;
use crate::error::{Result, ToolError};
use crate::types::{ToolDefinition, ToolId, ToolRecord, ToolState};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Central registry for managing capability-controlled tool registration, discovery, and execution.
#[derive(Debug)]
pub struct ToolRegistry {
    _config: ToolRegistryConfig,
    runtime: Arc<runtime::Runtime>,
    tools: RwLock<BTreeMap<ToolId, ToolRecord>>,
    by_name: RwLock<BTreeMap<String, ToolId>>,
    next_tool_id: AtomicU64,
}

impl ToolRegistry {
    /// Creates a new [`ToolRegistry`] with the given configuration and runtime reference.
    pub fn new(config: ToolRegistryConfig, runtime: Arc<runtime::Runtime>) -> Self {
        Self {
            _config: config,
            runtime,
            tools: RwLock::new(BTreeMap::new()),
            by_name: RwLock::new(BTreeMap::new()),
            next_tool_id: AtomicU64::new(1),
        }
    }

    /// Registers a new tool within the registry.
    ///
    /// Authorizes `CAP_TOOL_REGISTER` when `capability_registry` and token are supplied.
    pub fn register_tool(
        &self,
        definition: ToolDefinition,
        execution_context_id: Option<runtime::ExecutionContextId>,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<ToolId> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_TOOL_REGISTER")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        ToolError::Unauthorized { capability_id }
                    }
                    other => ToolError::Capability(other),
                })?;
        }

        let name_str = definition.name.0.clone();

        let by_name_map = self.by_name.read().map_err(|e| ToolError::LockError {
            message: e.to_string(),
        })?;

        if by_name_map.contains_key(&name_str) {
            return Err(ToolError::ToolAlreadyExists { name: name_str });
        }
        drop(by_name_map);

        let tid = ToolId(self.next_tool_id.fetch_add(1, Ordering::SeqCst));
        let record = ToolRecord {
            id: tid,
            definition,
            state: ToolState::Registered,
            execution_context_id,
        };

        let mut tools_map = self.tools.write().map_err(|e| ToolError::LockError {
            message: e.to_string(),
        })?;
        let mut by_name_map = self.by_name.write().map_err(|e| ToolError::LockError {
            message: e.to_string(),
        })?;

        by_name_map.insert(name_str, tid);
        tools_map.insert(tid, record);

        Ok(tid)
    }

    /// Discovers a registered tool by name.
    ///
    /// Authorizes `CAP_TOOL_EXECUTE` when `capability_registry` and token are supplied.
    pub fn lookup_tool(
        &self,
        name: &str,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<ToolRecord> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_TOOL_EXECUTE")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        ToolError::Unauthorized { capability_id }
                    }
                    other => ToolError::Capability(other),
                })?;
        }

        let by_name_map = self.by_name.read().map_err(|e| ToolError::LockError {
            message: e.to_string(),
        })?;

        let tid = by_name_map
            .get(name)
            .copied()
            .ok_or_else(|| ToolError::ToolNotFound {
                name_or_id: name.to_string(),
            })?;

        drop(by_name_map);
        self.get_tool(tid)
    }

    /// Retrieves a tool record by its [`ToolId`].
    pub fn get_tool(&self, id: ToolId) -> Result<ToolRecord> {
        let tools_map = self.tools.read().map_err(|e| ToolError::LockError {
            message: e.to_string(),
        })?;

        tools_map
            .get(&id)
            .cloned()
            .ok_or_else(|| ToolError::ToolNotFound {
                name_or_id: id.0.to_string(),
            })
    }

    /// Executes a registered tool synchronously.
    ///
    /// Authorizes `CAP_TOOL_EXECUTE` when `capability_registry` and token are supplied.
    pub fn execute_tool(
        &self,
        id: ToolId,
        input_json: &str,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<String> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_TOOL_EXECUTE")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        ToolError::Unauthorized { capability_id }
                    }
                    other => ToolError::Capability(other),
                })?;
        }

        let mut tools_map = self.tools.write().map_err(|e| ToolError::LockError {
            message: e.to_string(),
        })?;

        let record = tools_map
            .get_mut(&id)
            .ok_or_else(|| ToolError::ToolNotFound {
                name_or_id: id.0.to_string(),
            })?;

        if record.state == ToolState::Disabled || record.state == ToolState::Failed {
            return Err(ToolError::InvalidState {
                current: format!("{:?}", record.state),
                expected: "Registered or Active".to_string(),
            });
        }

        if let Some(ctx_id) = record.execution_context_id {
            let is_unhealthy = matches!(
                self.runtime.get_context_state(ctx_id),
                Ok(runtime::ContextState::Closed) | Ok(runtime::ContextState::Failed { .. })
            );
            if is_unhealthy {
                record.state = ToolState::Failed;
                return Err(ToolError::ExecutionFailed {
                    id,
                    message: "Underlying execution context closed or failed".to_string(),
                });
            }
            let _ = self.runtime.execute_context(ctx_id);
        }

        record.state = ToolState::Active;
        let tool_name = record.definition.name.0.clone();

        Ok(format!(
            "{{\"status\":\"success\",\"tool\":\"{tool_name}\",\"input\":{input_json}}}"
        ))
    }

    /// Lists all currently registered tool records.
    ///
    /// Authorizes `CAP_TOOL_EXECUTE` when `capability_registry` and token are supplied.
    pub fn list_tools(
        &self,
        capability_registry: Option<(
            &capabilities::CapabilityRegistry,
            &capabilities::CapabilityToken,
        )>,
    ) -> Result<Vec<ToolRecord>> {
        if let Some((registry, token)) = capability_registry {
            registry
                .authorize(token, "CAP_TOOL_EXECUTE")
                .map_err(|err| match err {
                    capabilities::CapabilityError::Unauthorized { capability_id, .. } => {
                        ToolError::Unauthorized { capability_id }
                    }
                    other => ToolError::Capability(other),
                })?;
        }

        let tools_map = self.tools.read().map_err(|e| ToolError::LockError {
            message: e.to_string(),
        })?;

        Ok(tools_map.values().cloned().collect())
    }
}
