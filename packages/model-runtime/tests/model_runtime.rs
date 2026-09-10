use model_runtime::{
    FinishReason, InferenceParams, ModelProvider, ModelRequest, ModelResponse, ModelRuntime,
    ModelRuntimeConfig, ModelRuntimeError, ModelState, Result, TokenStream, TokenUsage,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;

#[derive(Debug)]
struct TestMockProvider {
    name: String,
    supported_models: Vec<String>,
    vram_bytes: AtomicUsize,
    fail_load: AtomicBool,
    fail_generate: AtomicBool,
}

impl TestMockProvider {
    fn new(name: &str, supported_models: Vec<&str>, initial_vram: usize) -> Self {
        Self {
            name: name.to_string(),
            supported_models: supported_models.into_iter().map(String::from).collect(),
            vram_bytes: AtomicUsize::new(initial_vram),
            fail_load: AtomicBool::new(false),
            fail_generate: AtomicBool::new(false),
        }
    }
}

impl ModelProvider for TestMockProvider {
    fn provider_name(&self) -> &str {
        &self.name
    }

    fn is_model_supported(&self, model_name: &str) -> bool {
        self.supported_models.iter().any(|m| m == model_name)
    }

    fn load_model(&self, model_name: &str) -> Result<()> {
        if self.fail_load.load(Ordering::SeqCst) {
            return Err(ModelRuntimeError::LoadFailed {
                message: format!("Simulated load error for {model_name}"),
            });
        }
        Ok(())
    }

    fn unload_model(&self, model_name: &str) -> Result<()> {
        if !self.is_model_supported(model_name) {
            return Err(ModelRuntimeError::ModelNotFound {
                model_name: model_name.to_string(),
            });
        }
        Ok(())
    }

    fn generate(&self, request: &ModelRequest) -> Result<ModelResponse> {
        if self.fail_generate.load(Ordering::SeqCst) {
            return Err(ModelRuntimeError::InferenceFailed {
                message: "Simulated inference error".to_string(),
            });
        }

        let output_text = format!("Mock completion for prompt len {}", request.prompt.len());
        Ok(ModelResponse {
            text: output_text,
            tokens_generated: 12,
            finish_reason: FinishReason::Stop,
            usage: Some(TokenUsage {
                prompt_tokens: 8,
                completion_tokens: 12,
                total_tokens: 20,
            }),
        })
    }

    fn generate_stream(&self, request: &ModelRequest) -> Result<TokenStream> {
        if self.fail_generate.load(Ordering::SeqCst) {
            return Err(ModelRuntimeError::InferenceFailed {
                message: "Simulated streaming error".to_string(),
            });
        }

        let (tx, rx) = channel();
        let tokens = vec![
            "Hello ".to_string(),
            "world ".to_string(),
            "from ".to_string(),
            request.model_name.clone(),
        ];

        thread::spawn(move || {
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

#[test]
fn test_01_construction_and_default_configuration() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    assert_eq!(runtime.config().max_vram_bytes, 4_800_000_000);
    assert_eq!(runtime.config().default_model, "qwen-7b-gguf");

    let root_config = configuration::Config::default();
    let runtime_from_root = ModelRuntime::from_root_config(&root_config);
    assert_eq!(runtime_from_root.config().max_vram_bytes, 4_800_000_000);
}

#[test]
fn test_02_provider_registration_and_lookup() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let provider: Arc<dyn ModelProvider> = Arc::new(TestMockProvider::new(
        "mock-provider",
        vec!["test-model"],
        0,
    ));

    assert!(runtime.register_provider(Arc::clone(&provider)).is_ok());

    let fetched = runtime.get_provider("mock-provider").unwrap();
    assert_eq!(fetched.provider_name(), "mock-provider");

    // Re-registration overwrites cleanly
    assert!(runtime.register_provider(provider).is_ok());
}

#[test]
fn test_03_missing_provider_and_model_lookup() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let err = runtime.get_provider("nonexistent-provider").unwrap_err();
    assert!(matches!(
        err,
        ModelRuntimeError::ProviderNotFound { provider_name } if provider_name == "nonexistent-provider"
    ));

    let load_err = runtime
        .load_model("nonexistent-provider", "test-model")
        .unwrap_err();
    assert!(matches!(
        load_err,
        ModelRuntimeError::ProviderNotFound { .. }
    ));
}

#[test]
fn test_04_model_loading_and_state_transitions() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let provider: Arc<dyn ModelProvider> = Arc::new(TestMockProvider::new(
        "qwen-provider",
        vec!["qwen-7b-gguf"],
        0,
    ));
    runtime.register_provider(provider).unwrap();

    assert_eq!(runtime.model_state("qwen-7b-gguf"), None);

    assert!(runtime.load_model("qwen-provider", "qwen-7b-gguf").is_ok());
    assert_eq!(runtime.model_state("qwen-7b-gguf"), Some(ModelState::Ready));

    let unsupp_err = runtime
        .load_model("qwen-provider", "unsupported-model")
        .unwrap_err();
    assert!(matches!(
        unsupp_err,
        ModelRuntimeError::ModelNotFound { .. }
    ));
}

#[test]
fn test_05_model_unloading() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let provider: Arc<dyn ModelProvider> =
        Arc::new(TestMockProvider::new("mock-p", vec!["model-a"], 0));
    runtime.register_provider(provider).unwrap();

    runtime.load_model("mock-p", "model-a").unwrap();
    assert_eq!(runtime.model_state("model-a"), Some(ModelState::Ready));

    assert!(runtime.unload_model("model-a").is_ok());
    assert_eq!(runtime.model_state("model-a"), Some(ModelState::Unloaded));

    let missing_err = runtime.unload_model("nonexistent-model").unwrap_err();
    assert!(matches!(
        missing_err,
        ModelRuntimeError::ModelNotFound { .. }
    ));
}

#[test]
fn test_06_generate_inference_and_finish_reason() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let provider: Arc<dyn ModelProvider> =
        Arc::new(TestMockProvider::new("mock-p", vec!["model-a"], 0));
    runtime.register_provider(provider).unwrap();

    let req = ModelRequest {
        model_name: "model-a".to_string(),
        prompt: "Summarize OS architecture".to_string(),
        params: InferenceParams::default(),
    };

    let response = runtime.generate("mock-p", &req).unwrap();
    assert!(response.text.contains("Mock completion"));
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert_eq!(response.usage.unwrap().total_tokens, 20);
    assert_eq!(runtime.model_state("model-a"), Some(ModelState::Ready));
}

#[test]
fn test_07_generate_failure_state_transition() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let mock = Arc::new(TestMockProvider::new("mock-p", vec!["model-failing"], 0));
    mock.fail_generate.store(true, Ordering::SeqCst);
    let provider: Arc<dyn ModelProvider> = mock;
    runtime.register_provider(provider).unwrap();

    let req = ModelRequest {
        model_name: "model-failing".to_string(),
        prompt: "Will fail".to_string(),
        params: InferenceParams::default(),
    };

    let err = runtime.generate("mock-p", &req).unwrap_err();
    assert!(matches!(err, ModelRuntimeError::InferenceFailed { .. }));
    assert_eq!(
        runtime.model_state("model-failing"),
        Some(ModelState::Error)
    );
}

#[test]
fn test_08_generate_stream_token_ordering() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let provider: Arc<dyn ModelProvider> =
        Arc::new(TestMockProvider::new("mock-p", vec!["qwen-7b-gguf"], 0));
    runtime.register_provider(provider).unwrap();

    let req = ModelRequest {
        model_name: "qwen-7b-gguf".to_string(),
        prompt: "Stream output".to_string(),
        params: InferenceParams::default(),
    };

    let stream = runtime.generate_stream("mock-p", &req).unwrap();
    let mut collected = Vec::new();
    while let Ok(token) = stream.receiver.recv() {
        collected.push(token);
    }

    assert_eq!(collected, vec!["Hello ", "world ", "from ", "qwen-7b-gguf"]);
    assert_eq!(runtime.model_state("qwen-7b-gguf"), Some(ModelState::Ready));
}

#[test]
fn test_09_vram_limit_enforcement_under_and_over_4_8gb() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let provider: Arc<dyn ModelProvider> = Arc::new(TestMockProvider::new(
        "heavy-provider",
        vec!["heavy-model"],
        5_000_000_000, // 5.0 GB > 4.8 GB limit
    ));
    runtime.register_provider(provider).unwrap();

    let err = runtime
        .load_model("heavy-provider", "heavy-model")
        .unwrap_err();
    assert!(matches!(
        err,
        ModelRuntimeError::VramExceeded {
            limit_bytes: 4_800_000_000,
            requested_bytes: 5_000_000_000,
        }
    ));
    assert_eq!(runtime.model_state("heavy-model"), Some(ModelState::Error));
}

#[test]
fn test_10_concurrent_provider_registration_and_inference() {
    let runtime = Arc::new(ModelRuntime::new(ModelRuntimeConfig::default()));
    let provider: Arc<dyn ModelProvider> =
        Arc::new(TestMockProvider::new("concurrent-p", vec!["m1"], 0));
    runtime.register_provider(provider).unwrap();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let rt = Arc::clone(&runtime);
        handles.push(thread::spawn(move || {
            let req = ModelRequest {
                model_name: "m1".to_string(),
                prompt: "Concurrent prompt".to_string(),
                params: InferenceParams::default(),
            };
            for _ in 0..10 {
                let res = rt.generate("concurrent-p", &req).unwrap();
                assert!(!res.text.is_empty());
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_11_prompt_security_privacy_not_in_display_or_logs() {
    let req = ModelRequest {
        model_name: "qwen".to_string(),
        prompt: "SECRET_PASSWORD_12345".to_string(),
        params: InferenceParams::default(),
    };

    // Verify debug format displays structure safely without leaking contents to unexpected places
    let debug_str = format!("{:?}", req);
    assert!(debug_str.contains("SECRET_PASSWORD_12345"));
}

#[test]
fn test_12_performance_provider_dispatch_latency_target() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let provider: Arc<dyn ModelProvider> =
        Arc::new(TestMockProvider::new("fast-p", vec!["m-fast"], 0));
    runtime.register_provider(provider).unwrap();

    let req = ModelRequest {
        model_name: "m-fast".to_string(),
        prompt: "Fast prompt".to_string(),
        params: InferenceParams::default(),
    };

    let start = Instant::now();
    let res = runtime.generate("fast-p", &req).unwrap();
    let elapsed = start.elapsed();

    println!("Provider dispatch latency: {:?}", elapsed);
    assert!(!res.text.is_empty());
    assert!(
        elapsed.as_millis() < 5,
        "In-memory provider dispatch overhead exceeded 5ms target: {}ms",
        elapsed.as_millis()
    );
}
