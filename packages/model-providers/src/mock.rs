//! Deterministic Mock Model Provider for NAINA OS.

use model_runtime::{
    FinishReason, ModelProvider, ModelRequest, ModelResponse, ModelRuntimeError, Result,
    TokenStream, TokenUsage,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::thread;

/// Deterministic mock model provider for offline testing.
#[derive(Debug)]
pub struct MockModelProvider {
    name: String,
    vram_bytes: AtomicUsize,
    is_loaded: AtomicBool,
}

impl Default for MockModelProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockModelProvider {
    /// Constructs a new [`MockModelProvider`] named "mock".
    pub fn new() -> Self {
        Self::with_name("mock")
    }

    /// Constructs a [`MockModelProvider`] with a custom provider name.
    pub fn with_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            vram_bytes: AtomicUsize::new(0),
            is_loaded: AtomicBool::new(false),
        }
    }
}

impl ModelProvider for MockModelProvider {
    fn provider_name(&self) -> &str {
        &self.name
    }

    fn is_model_supported(&self, _model_name: &str) -> bool {
        true
    }

    fn load_model(&self, _model_name: &str) -> Result<()> {
        // Mock VRAM allocation: 500 MB (500_000_000 bytes)
        self.vram_bytes.store(500_000_000, Ordering::SeqCst);
        self.is_loaded.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn unload_model(&self, _model_name: &str) -> Result<()> {
        self.vram_bytes.store(0, Ordering::SeqCst);
        self.is_loaded.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn generate(&self, request: &ModelRequest) -> Result<ModelResponse> {
        if !self.is_loaded.load(Ordering::SeqCst) {
            return Err(ModelRuntimeError::InferenceFailed {
                message: format!("Mock model '{}' not loaded", request.model_name),
            });
        }

        let prompt_tokens = request.prompt.len() / 4 + 1;
        let completion_tokens = 8;
        let total_tokens = prompt_tokens + completion_tokens;

        Ok(ModelResponse {
            text: format!("Mock completion for prompt: '{}'", request.prompt),
            tokens_generated: completion_tokens,
            finish_reason: FinishReason::Stop,
            usage: Some(TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            }),
        })
    }

    fn generate_stream(&self, request: &ModelRequest) -> Result<TokenStream> {
        if !self.is_loaded.load(Ordering::SeqCst) {
            return Err(ModelRuntimeError::InferenceFailed {
                message: format!("Mock model '{}' not loaded", request.model_name),
            });
        }

        let (tx, rx) = channel();
        let model_name = request.model_name.clone();

        thread::spawn(move || {
            let tokens = vec![
                "Mock ".to_string(),
                "stream ".to_string(),
                "for ".to_string(),
                model_name,
            ];
            for token in tokens {
                let _ = tx.send(token);
            }
        });

        Ok(TokenStream { receiver: rx })
    }

    fn current_vram_usage_bytes(&self) -> usize {
        self.vram_bytes.load(Ordering::SeqCst)
    }
}
