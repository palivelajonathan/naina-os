//! Primary ModelRuntime manager implementation for NAINA OS.

use crate::config::ModelRuntimeConfig;
use crate::error::{ModelRuntimeError, Result};
use crate::traits::ModelProvider;
use crate::types::{ModelRequest, ModelResponse, TokenStream};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;

/// Lifecycle state of a model within the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelState {
    Unloaded,
    Loading,
    Ready,
    Inferring,
    Error,
}

/// Primary model execution runtime manager for NAINA OS.
#[derive(Debug)]
pub struct ModelRuntime {
    config: ModelRuntimeConfig,
    providers: RwLock<BTreeMap<String, Arc<dyn ModelProvider>>>,
    model_states: RwLock<BTreeMap<String, ModelState>>,
}

impl ModelRuntime {
    /// Creates a new [`ModelRuntime`] with the given configuration.
    pub fn new(config: ModelRuntimeConfig) -> Self {
        Self {
            config,
            providers: RwLock::new(BTreeMap::new()),
            model_states: RwLock::new(BTreeMap::new()),
        }
    }

    /// Constructs a [`ModelRuntime`] directly from root [`configuration::Config`].
    pub fn from_root_config(_root_config: &configuration::Config) -> Self {
        Self::new(ModelRuntimeConfig::default())
    }

    /// Registers a concrete [`ModelProvider`] implementation.
    pub fn register_provider(&self, provider: Arc<dyn ModelProvider>) -> Result<()> {
        let mut map = self
            .providers
            .write()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;

        let name = provider.provider_name().to_string();
        map.insert(name, provider);
        Ok(())
    }

    /// Retrieves a registered [`ModelProvider`] by name.
    pub fn get_provider(&self, provider_name: &str) -> Result<Arc<dyn ModelProvider>> {
        let map = self
            .providers
            .read()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;

        map.get(provider_name)
            .cloned()
            .ok_or_else(|| ModelRuntimeError::ProviderNotFound {
                provider_name: provider_name.to_string(),
            })
    }

    /// Returns the total GPU VRAM usage in bytes across all registered providers.
    pub fn current_vram_bytes(&self) -> usize {
        if let Ok(map) = self.providers.read() {
            map.values().map(|p| p.current_vram_usage_bytes()).sum()
        } else {
            0
        }
    }

    /// Loads a model into memory/VRAM via the specified provider adapter.
    pub fn load_model(&self, provider_name: &str, model_name: &str) -> Result<()> {
        let provider = self.get_provider(provider_name)?;

        if !provider.is_model_supported(model_name) {
            return Err(ModelRuntimeError::ModelNotFound {
                model_name: model_name.to_string(),
            });
        }

        self.set_model_state(model_name, ModelState::Loading)?;

        // VRAM limit check (< 4.8 GB limit)
        let vram_usage = self.current_vram_bytes();
        if vram_usage > self.config.max_vram_bytes {
            self.set_model_state(model_name, ModelState::Error)?;
            return Err(ModelRuntimeError::VramExceeded {
                limit_bytes: self.config.max_vram_bytes,
                requested_bytes: vram_usage,
            });
        }

        match provider.load_model(model_name) {
            Ok(()) => {
                // Verify VRAM again post-load
                let post_vram = self.current_vram_bytes();
                if post_vram > self.config.max_vram_bytes {
                    let _ = provider.unload_model(model_name);
                    self.set_model_state(model_name, ModelState::Error)?;
                    return Err(ModelRuntimeError::VramExceeded {
                        limit_bytes: self.config.max_vram_bytes,
                        requested_bytes: post_vram,
                    });
                }
                self.set_model_state(model_name, ModelState::Ready)?;
                Ok(())
            }
            Err(err) => {
                self.set_model_state(model_name, ModelState::Error)?;
                Err(err)
            }
        }
    }

    /// Unloads a model from memory/VRAM.
    pub fn unload_model(&self, model_name: &str) -> Result<()> {
        let map = self
            .providers
            .read()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;

        let mut unloaded_any = false;
        for provider in map.values() {
            if provider.is_model_supported(model_name) {
                let _ = provider.unload_model(model_name);
                unloaded_any = true;
            }
        }

        if !unloaded_any {
            return Err(ModelRuntimeError::ModelNotFound {
                model_name: model_name.to_string(),
            });
        }

        self.set_model_state(model_name, ModelState::Unloaded)?;
        Ok(())
    }

    /// Executes a synchronous prompt inference request.
    pub fn generate(&self, provider_name: &str, request: &ModelRequest) -> Result<ModelResponse> {
        let provider = self.get_provider(provider_name)?;
        let model_name = &request.model_name;

        self.set_model_state(model_name, ModelState::Inferring)?;

        let result = provider.generate(request);

        match &result {
            Ok(_) => {
                let _ = self.set_model_state(model_name, ModelState::Ready);
            }
            Err(_) => {
                let _ = self.set_model_state(model_name, ModelState::Error);
            }
        }

        result
    }

    /// Executes a streaming prompt inference request returning a [`TokenStream`].
    pub fn generate_stream(
        &self,
        provider_name: &str,
        request: &ModelRequest,
    ) -> Result<TokenStream> {
        let provider = self.get_provider(provider_name)?;
        let model_name = &request.model_name;

        self.set_model_state(model_name, ModelState::Inferring)?;

        let result = provider.generate_stream(request);

        match &result {
            Ok(_) => {
                let _ = self.set_model_state(model_name, ModelState::Ready);
            }
            Err(_) => {
                let _ = self.set_model_state(model_name, ModelState::Error);
            }
        }

        result
    }

    /// Retrieves the current [`ModelState`] for a given model.
    pub fn model_state(&self, model_name: &str) -> Option<ModelState> {
        self.model_states
            .read()
            .ok()
            .and_then(|map| map.get(model_name).copied())
    }

    /// Returns the active configuration.
    pub fn config(&self) -> &ModelRuntimeConfig {
        &self.config
    }

    fn set_model_state(&self, model_name: &str, state: ModelState) -> Result<()> {
        let mut map = self
            .model_states
            .write()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;
        map.insert(model_name.to_string(), state);
        Ok(())
    }
}
