//! Trait abstractions for NAINA OS model providers.

use crate::error::Result;
use crate::types::{DetailedModelResponse, ModelRequest, ModelResponse, TokenStream};
use std::fmt::Debug;

/// Interface contract (`IModelAdapter`) implemented by concrete model adapters in `model-providers`.
pub trait ModelProvider: Send + Sync + Debug {
    /// Returns the provider's unique identifier (e.g. "qwen-gguf", "mock").
    fn provider_name(&self) -> &str;

    /// Checks if the provider supports the requested model.
    fn is_model_supported(&self, model_name: &str) -> bool;

    /// Loads the model into memory/VRAM.
    fn load_model(&self, model_name: &str) -> Result<()>;

    /// Unloads the model from memory/VRAM.
    fn unload_model(&self, model_name: &str) -> Result<()>;

    /// Executes a synchronous inference generation request.
    fn generate(&self, request: &ModelRequest) -> Result<ModelResponse>;

    /// Executes an inference generation request returning detailed latency breakdown metrics.
    fn generate_detailed_metrics(&self, request: &ModelRequest) -> Result<DetailedModelResponse> {
        let resp = self.generate(request)?;
        Ok(DetailedModelResponse {
            response: resp,
            ttft: std::time::Duration::from_secs(0),
            token_generation_duration: std::time::Duration::from_secs(0),
        })
    }

    /// Executes a streaming inference request returning a [`TokenStream`].
    fn generate_stream(&self, request: &ModelRequest) -> Result<TokenStream>;

    /// Returns the current estimated GPU VRAM memory usage in bytes.
    fn current_vram_usage_bytes(&self) -> usize;
}
