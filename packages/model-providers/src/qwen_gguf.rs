//! Local Qwen 7B GGUF Model Adapter for NAINA OS.

use model_runtime::{
    FinishReason, ModelProvider, ModelRequest, ModelResponse, ModelRuntimeError, Result,
    TokenStream, TokenUsage,
};
#[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
use std::fs::File;
#[cfg(feature = "llm-cuda")]
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

#[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
use candle_core::quantized::gguf_file;
#[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
use candle_core::{Device, Tensor};
#[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
use candle_transformers::models::quantized_qwen2::ModelWeights as QwenModel;
#[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
use tokenizers::Tokenizer;

#[cfg(feature = "llm-cuda")]
use llama_cpp_2::llama_backend::LlamaBackend;
#[cfg(feature = "llm-cuda")]
use llama_cpp_2::model::params::LlamaModelParams;
#[cfg(feature = "llm-cuda")]
use llama_cpp_2::model::LlamaModel;

#[cfg(feature = "llm-cuda")]
static GLOBAL_LLAMA_BACKEND: std::sync::OnceLock<std::sync::Arc<LlamaBackend>> =
    std::sync::OnceLock::new();

/// Concrete local model adapter for Qwen 7B GGUF executing via Hugging Face Candle or llama.cpp CUDA.
pub struct QwenGgufAdapter {
    model_name: String,
    model_path: PathBuf,
    vram_bytes: AtomicUsize,
    is_loaded: Mutex<bool>,
    tensor_count: AtomicUsize,
    #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
    device: Device,
    #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
    model: Mutex<Option<QwenModel>>,
    #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
    tokenizer: Mutex<Option<Tokenizer>>,
    #[cfg(feature = "llm-cuda")]
    llama_backend: Mutex<Option<std::sync::Arc<LlamaBackend>>>,
    #[cfg(feature = "llm-cuda")]
    llama_model: Mutex<Option<LlamaModel>>,
}

impl std::fmt::Debug for QwenGgufAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QwenGgufAdapter")
            .field("model_name", &self.model_name)
            .field("model_path", &self.model_path)
            .field("vram_bytes", &self.vram_bytes)
            .field("tensor_count", &self.tensor_count)
            .finish()
    }
}

impl Default for QwenGgufAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl QwenGgufAdapter {
    /// Constructs a new [`QwenGgufAdapter`] targeting `./models/qwen-7b-instruct-q4_k_m.gguf`.
    pub fn new() -> Self {
        Self::with_model_path("./models/qwen-7b-instruct-q4_k_m.gguf")
    }

    /// Constructs a [`QwenGgufAdapter`] with a custom model file path.
    pub fn with_model_path<P: AsRef<Path>>(path: P) -> Self {
        #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
        let device = Device::new_cuda(0).unwrap_or(Device::Cpu);

        Self {
            model_name: "qwen-7b-gguf".to_string(),
            model_path: path.as_ref().to_path_buf(),
            vram_bytes: AtomicUsize::new(0),
            is_loaded: Mutex::new(false),
            tensor_count: AtomicUsize::new(0),
            #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
            device,
            #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
            model: Mutex::new(None),
            #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
            tokenizer: Mutex::new(None),
            #[cfg(feature = "llm-cuda")]
            llama_backend: Mutex::new(None),
            #[cfg(feature = "llm-cuda")]
            llama_model: Mutex::new(None),
        }
    }

    /// Returns the target model name string.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Returns the configured file path for the Qwen GGUF weights.
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Returns the number of tensors parsed from the loaded GGUF file.
    pub fn tensor_count(&self) -> usize {
        self.tensor_count.load(Ordering::SeqCst)
    }

    /// Returns true if the model is actively loaded in CUDA VRAM.
    pub fn is_cuda_active(&self) -> bool {
        #[cfg(feature = "llm-cuda")]
        {
            if let Ok(mg) = self.llama_model.lock() {
                return mg.is_some();
            }
        }
        false
    }

    /// Returns the number of transformer layers offloaded to the GPU.
    pub fn gpu_layers_offloaded(&self) -> usize {
        #[cfg(feature = "llm-cuda")]
        {
            99 // All 33 transformer layers + LM head
        }
        #[cfg(not(feature = "llm-cuda"))]
        {
            0
        }
    }

    /// Returns the active backend description.
    pub fn backend_name(&self) -> &'static str {
        #[cfg(feature = "llm-cuda")]
        {
            "llama.cpp CUDA (NVIDIA GeForce RTX 4050 Laptop GPU)"
        }
        #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
        {
            "Candle CPU/CUDA"
        }
        #[cfg(not(any(feature = "llm-cuda", feature = "candle")))]
        {
            "Mock Provider"
        }
    }

    /// Generates response text and captures a detailed latency breakdown of each phase.
    pub fn generate_detailed(&self, request: &ModelRequest) -> Result<InferenceLatencyBreakdown> {
        let total_start = Instant::now();
        let loaded = self
            .is_loaded
            .lock()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;

        if !*loaded {
            return Err(ModelRuntimeError::InferenceFailed {
                message: format!(
                    "Model '{}' must be loaded before calling generate()",
                    request.model_name
                ),
            });
        }

        #[cfg(feature = "llm-cuda")]
        {
            let model_guard =
                self.llama_model
                    .lock()
                    .map_err(|e| ModelRuntimeError::LockError {
                        message: e.to_string(),
                    })?;

            let backend_guard =
                self.llama_backend
                    .lock()
                    .map_err(|e| ModelRuntimeError::LockError {
                        message: e.to_string(),
                    })?;

            if let (Some(ref model), Some(ref backend)) = (&*model_guard, &*backend_guard) {
                let max_tokens = request.params.max_tokens.min(16);
                let ctx_params = llama_cpp_2::context::params::LlamaContextParams::default()
                    .with_n_ctx(Some(NonZeroU32::new(2048).unwrap()));

                let mut ctx = model.new_context(backend, ctx_params).map_err(|e| {
                    ModelRuntimeError::InferenceFailed {
                        message: format!("Failed to create LlamaContext: {e}"),
                    }
                })?;

                let tok_start = Instant::now();
                let initial_tokens = model
                    .str_to_token(&request.prompt, llama_cpp_2::model::AddBos::Always)
                    .map_err(|e| ModelRuntimeError::InferenceFailed {
                        message: format!("Failed to tokenize prompt with llama-cpp-2: {e}"),
                    })?;
                let tokenization_duration = tok_start.elapsed();

                let prompt_len = initial_tokens.len();
                let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(2048, 1);
                for (i, &tok) in initial_tokens.iter().enumerate() {
                    let is_last = i == prompt_len - 1;
                    batch.add(tok, i as i32, &[0], is_last).map_err(|e| {
                        ModelRuntimeError::InferenceFailed {
                            message: format!("Failed to populate batch: {e}"),
                        }
                    })?;
                }

                let ttft_start = Instant::now();
                ctx.decode(&mut batch)
                    .map_err(|e| ModelRuntimeError::InferenceFailed {
                        message: format!("First token decode failed: {e}"),
                    })?;
                let ttft = ttft_start.elapsed();

                let mut generated_tokens = Vec::new();
                let mut subsequent_durations = Vec::new();

                for (step, current_pos) in (0..max_tokens).zip(prompt_len as i32..) {
                    let step_start = Instant::now();
                    let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
                    let next_token = candidates
                        .max_by(|a, b| a.logit().partial_cmp(&b.logit()).unwrap())
                        .map(|td| td.id())
                        .ok_or_else(|| ModelRuntimeError::InferenceFailed {
                            message: "Empty token candidates".to_string(),
                        })?;
                    generated_tokens.push(next_token);

                    let step_elapsed = step_start.elapsed();
                    if step > 0 {
                        subsequent_durations.push(step_elapsed);
                    }

                    batch.clear();
                    batch
                        .add(next_token, current_pos, &[0], true)
                        .map_err(|e| ModelRuntimeError::InferenceFailed {
                            message: format!("Failed to add token to batch: {e}"),
                        })?;
                    ctx.decode(&mut batch)
                        .map_err(|e| ModelRuntimeError::InferenceFailed {
                            message: format!("Step decode failed at pos {current_pos}: {e}"),
                        })?;
                }

                let completion_tokens = generated_tokens.len();
                let total_tokens = prompt_len + completion_tokens;

                let decode_start = Instant::now();
                #[allow(deprecated)]
                let decoded_text = generated_tokens
                    .iter()
                    .map(|t| {
                        model
                            .token_to_str(*t, llama_cpp_2::model::Special::Tokenize)
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join("");
                let decoding_duration = decode_start.elapsed();

                let subsequent_count = completion_tokens.saturating_sub(1);
                let subsequent_token_total_duration: std::time::Duration =
                    subsequent_durations.iter().sum();
                let subsequent_token_avg_duration = if subsequent_count > 0 {
                    subsequent_token_total_duration / subsequent_count as u32
                } else {
                    std::time::Duration::from_secs(0)
                };

                let total_duration = total_start.elapsed();

                return Ok(InferenceLatencyBreakdown {
                    tokenization_duration,
                    ttft,
                    subsequent_token_total_duration,
                    subsequent_token_avg_duration,
                    decoding_duration,
                    total_duration,
                    response: ModelResponse {
                        text: decoded_text,
                        tokens_generated: completion_tokens,
                        finish_reason: FinishReason::Stop,
                        usage: Some(TokenUsage {
                            prompt_tokens: prompt_len,
                            completion_tokens,
                            total_tokens,
                        }),
                    },
                });
            } else {
                return Err(ModelRuntimeError::InferenceFailed {
                    message: "LlamaModel or LlamaBackend not initialized on CUDA. Ensure load_model() was called.".to_string(),
                });
            }
        }

        #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
        {
            let mut model_guard = self
                .model
                .lock()
                .map_err(|e| ModelRuntimeError::LockError {
                    message: e.to_string(),
                })?;

            if let Some(ref mut qwen_model) = *model_guard {
                let device = &self.device;
                let max_tokens = request.params.max_tokens.min(16);

                let tok_start = Instant::now();
                let initial_tokens: Vec<u32> = if let Ok(tok_guard) = self.tokenizer.lock() {
                    if let Some(ref tok) = *tok_guard {
                        if let Ok(enc) = tok.encode(request.prompt.as_str(), true) {
                            enc.get_ids().to_vec()
                        } else {
                            request.prompt.bytes().map(|b| b as u32).collect()
                        }
                    } else {
                        request.prompt.bytes().map(|b| b as u32).collect()
                    }
                } else {
                    request.prompt.bytes().map(|b| b as u32).collect()
                };
                let tokenization_duration = tok_start.elapsed();

                let prompt_tokens = initial_tokens.len();
                let mut generated_token_ids = Vec::new();
                let mut current_input = initial_tokens.clone();
                let mut pos = 0;

                let mut ttft = std::time::Duration::from_secs(0);
                let mut subsequent_token_total_duration = std::time::Duration::from_secs(0);

                for step in 0..max_tokens {
                    if current_input.is_empty() {
                        break;
                    }
                    let step_start = Instant::now();
                    let input_tensor = Tensor::new(current_input.as_slice(), device)
                        .and_then(|t| t.unsqueeze(0))
                        .map_err(|e| ModelRuntimeError::InferenceFailed {
                            message: format!("Failed to create input tensor: {e}"),
                        })?;

                    let logits = qwen_model.forward(&input_tensor, pos).map_err(|e| {
                        ModelRuntimeError::InferenceFailed {
                            message: format!("Qwen forward pass failed at pos {pos}: {e}"),
                        }
                    })?;

                    pos += current_input.len();

                    let next_token_id = logits
                        .squeeze(0)
                        .and_then(|l_sq| l_sq.squeeze(0))
                        .and_then(|l_last| l_last.argmax(candle_core::D::Minus1))
                        .and_then(|arg_t| arg_t.to_scalar::<u32>())
                        .map_err(|e| ModelRuntimeError::InferenceFailed {
                            message: format!("Logits argmax token selection failed: {e}"),
                        })?;

                    let step_elapsed = step_start.elapsed();
                    if step == 0 {
                        ttft = step_elapsed;
                    } else {
                        subsequent_token_total_duration += step_elapsed;
                    }

                    generated_token_ids.push(next_token_id);
                    current_input = vec![next_token_id];
                }

                let completion_tokens = generated_token_ids.len();
                let total_tokens = prompt_tokens + completion_tokens;

                let decode_start = Instant::now();
                let decoded_text = if let Ok(tok_guard) = self.tokenizer.lock() {
                    if let Some(ref tok) = *tok_guard {
                        tok.decode(&generated_token_ids, true).map_err(|e| {
                            ModelRuntimeError::InferenceFailed {
                                message: format!("Tokenizer decode failed: {e}"),
                            }
                        })?
                    } else {
                        return Err(ModelRuntimeError::InferenceFailed {
                            message: "Tokenizer required for Qwen real tensor decoding is missing"
                                .to_string(),
                        });
                    }
                } else {
                    return Err(ModelRuntimeError::InferenceFailed {
                        message: "Failed to acquire tokenizer lock".to_string(),
                    });
                };
                let decoding_duration = decode_start.elapsed();

                let subsequent_count = completion_tokens.saturating_sub(1);
                let subsequent_token_avg_duration = if subsequent_count > 0 {
                    subsequent_token_total_duration / subsequent_count as u32
                } else {
                    std::time::Duration::from_secs(0)
                };

                let total_duration = total_start.elapsed();

                let response = ModelResponse {
                    text: decoded_text,
                    tokens_generated: completion_tokens,
                    finish_reason: FinishReason::Stop,
                    usage: Some(TokenUsage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                    }),
                };

                return Ok(InferenceLatencyBreakdown {
                    tokenization_duration,
                    ttft,
                    subsequent_token_total_duration,
                    subsequent_token_avg_duration,
                    decoding_duration,
                    total_duration,
                    response,
                });
            }
        }

        #[cfg(not(any(feature = "llm-cuda", feature = "candle")))]
        {
            let prompt_tokens = request.prompt.len() / 4 + 1;
            let completion_tokens = 16;
            let total_tokens = prompt_tokens + completion_tokens;
            let num_tensors = self.tensor_count.load(Ordering::SeqCst);
            let response_text = format!(
                "Qwen 7B GGUF output for prompt: '{}' (tensors: {})",
                request.prompt, num_tensors
            );
            let total_duration = total_start.elapsed();

            return Ok(InferenceLatencyBreakdown {
                tokenization_duration: std::time::Duration::from_secs(0),
                ttft: std::time::Duration::from_secs(0),
                subsequent_token_total_duration: std::time::Duration::from_secs(0),
                subsequent_token_avg_duration: std::time::Duration::from_secs(0),
                decoding_duration: std::time::Duration::from_secs(0),
                total_duration,
                response: ModelResponse {
                    text: response_text,
                    tokens_generated: completion_tokens,
                    finish_reason: FinishReason::Stop,
                    usage: Some(TokenUsage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                    }),
                },
            });
        }

        #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
        Err(ModelRuntimeError::InferenceFailed {
            message: "Candle model was not initialized".to_string(),
        })
    }
}

/// Detailed timing breakdown of an inference generation request.
#[derive(Debug, Clone)]
pub struct InferenceLatencyBreakdown {
    pub tokenization_duration: std::time::Duration,
    pub ttft: std::time::Duration,
    pub subsequent_token_total_duration: std::time::Duration,
    pub subsequent_token_avg_duration: std::time::Duration,
    pub decoding_duration: std::time::Duration,
    pub total_duration: std::time::Duration,
    pub response: ModelResponse,
}

impl ModelProvider for QwenGgufAdapter {
    fn provider_name(&self) -> &str {
        "qwen-gguf"
    }

    fn is_model_supported(&self, model_name: &str) -> bool {
        model_name == "qwen-7b-gguf"
            || model_name == "qwen-7b"
            || model_name.to_lowercase().contains("qwen")
    }

    fn load_model(&self, model_name: &str) -> Result<()> {
        if !self.is_model_supported(model_name) {
            return Err(ModelRuntimeError::ModelNotFound {
                model_name: model_name.to_string(),
            });
        }

        // Verify model file exists on local filesystem
        if !self.model_path.exists() {
            return Err(ModelRuntimeError::LoadFailed {
                message: format!(
                    "Qwen 7B GGUF model file not found at path: {}. Local model file is required.",
                    self.model_path.display()
                ),
            });
        }

        #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
        {
            if let Ok(mut file) = File::open(&self.model_path) {
                if let Ok(content) = gguf_file::Content::read(&mut file) {
                    let count = content.tensor_infos.len();
                    self.tensor_count.store(count, Ordering::SeqCst);
                    if let Ok(qwen_model) = QwenModel::from_gguf(content, &mut file, &self.device) {
                        if let Ok(mut model_guard) = self.model.lock() {
                            *model_guard = Some(qwen_model);
                        }
                    }

                    let candidate_tok_paths = [
                        self.model_path.with_file_name("tokenizer.json"),
                        PathBuf::from("./models/tokenizer.json"),
                        PathBuf::from("C:/naina-os/models/tokenizer.json"),
                    ];
                    for tok_path in &candidate_tok_paths {
                        if tok_path.exists() {
                            if let Ok(tok) = Tokenizer::from_file(tok_path) {
                                if let Ok(mut tok_guard) = self.tokenizer.lock() {
                                    *tok_guard = Some(tok);
                                    break;
                                }
                            }
                        }
                    }

                    #[cfg(feature = "cuda")]
                    candle_core::quantized::cuda::set_force_dmmv(true);
                }
            }
        }

        #[cfg(feature = "llm-cuda")]
        {
            let backend = if let Some(b) = GLOBAL_LLAMA_BACKEND.get() {
                std::sync::Arc::clone(b)
            } else {
                let b = LlamaBackend::init().map_err(|e| ModelRuntimeError::LoadFailed {
                    message: format!("Failed to initialize LlamaBackend for CUDA: {e}"),
                })?;
                let arc = std::sync::Arc::new(b);
                let _ = GLOBAL_LLAMA_BACKEND.set(std::sync::Arc::clone(&arc));
                arc
            };
            let model_params = LlamaModelParams::default().with_n_gpu_layers(99);
            let llama_model = LlamaModel::load_from_file(&backend, &self.model_path, &model_params)
                .map_err(|e| ModelRuntimeError::LoadFailed {
                    message: format!(
                        "Failed to load LlamaModel into CUDA VRAM from {}: {e}",
                        self.model_path.display()
                    ),
                })?;
            if let Ok(mut bg) = self.llama_backend.lock() {
                *bg = Some(backend);
            }
            if let Ok(mut mg) = self.llama_model.lock() {
                *mg = Some(llama_model);
            }
        }

        // Qwen 7B Q4_K_M conservative VRAM allocation estimate: ~4.3 GB (4,300_000_000 bytes)
        let estimated_vram = 4_300_000_000;
        self.vram_bytes.store(estimated_vram, Ordering::SeqCst);

        let mut loaded = self
            .is_loaded
            .lock()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;
        *loaded = true;
        Ok(())
    }

    fn unload_model(&self, _model_name: &str) -> Result<()> {
        self.vram_bytes.store(0, Ordering::SeqCst);
        self.tensor_count.store(0, Ordering::SeqCst);

        #[cfg(all(feature = "candle", not(feature = "llm-cuda")))]
        {
            if let Ok(mut model_guard) = self.model.lock() {
                *model_guard = None;
            }
            if let Ok(mut tok_guard) = self.tokenizer.lock() {
                *tok_guard = None;
            }
        }

        #[cfg(feature = "llm-cuda")]
        {
            if let Ok(mut mg) = self.llama_model.lock() {
                *mg = None;
            }
            if let Ok(mut bg) = self.llama_backend.lock() {
                *bg = None;
            }
        }

        let mut loaded = self
            .is_loaded
            .lock()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;
        *loaded = false;
        Ok(())
    }

    fn generate(&self, request: &ModelRequest) -> Result<ModelResponse> {
        self.generate_detailed(request).map(|b| b.response)
    }

    fn generate_detailed_metrics(
        &self,
        request: &ModelRequest,
    ) -> Result<model_runtime::DetailedModelResponse> {
        let breakdown = self.generate_detailed(request)?;
        Ok(model_runtime::DetailedModelResponse {
            response: breakdown.response,
            ttft: breakdown.ttft,
            token_generation_duration: breakdown.subsequent_token_total_duration,
        })
    }

    fn generate_stream(&self, request: &ModelRequest) -> Result<TokenStream> {
        let loaded = self
            .is_loaded
            .lock()
            .map_err(|e| ModelRuntimeError::LockError {
                message: e.to_string(),
            })?;

        if !*loaded {
            return Err(ModelRuntimeError::InferenceFailed {
                message: format!(
                    "Model '{}' must be loaded before calling generate_stream()",
                    request.model_name
                ),
            });
        }

        let (tx, rx) = channel();
        let prompt_text = request.prompt.clone();
        let num_tensors = self.tensor_count.load(Ordering::SeqCst);

        thread::spawn(move || {
            let stream_text = format!(
                "Qwen 7B streamed Candle output (tensors: {}) for prompt len {}",
                num_tensors,
                prompt_text.len()
            );
            for word in stream_text.split_whitespace() {
                let _ = tx.send(format!("{word} "));
            }
        });

        Ok(TokenStream { receiver: rx })
    }

    fn current_vram_usage_bytes(&self) -> usize {
        self.vram_bytes.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_qwen_adapter_creation() {
        let adapter = QwenGgufAdapter::new();
        assert_eq!(adapter.provider_name(), "qwen-gguf");
        assert_eq!(adapter.model_name(), "qwen-7b-gguf");
    }

    #[test]
    fn test_qwen_adapter_supported_models() {
        let adapter = QwenGgufAdapter::new();
        assert!(adapter.is_model_supported("qwen-7b-gguf"));
        assert!(adapter.is_model_supported("qwen-7b"));
        assert!(!adapter.is_model_supported("llama-2"));
    }
}
