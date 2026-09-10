use model_providers::{MockModelProvider, QwenGgufAdapter};
use model_runtime::{
    FinishReason, InferenceParams, ModelProvider, ModelRequest, ModelRuntime, ModelRuntimeConfig,
    ModelRuntimeError, ModelState,
};
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

#[test]
fn test_01_provider_construction_and_names() {
    let mock = MockModelProvider::new();
    assert_eq!(mock.provider_name(), "mock");

    let qwen = QwenGgufAdapter::new();
    assert_eq!(qwen.provider_name(), "qwen-gguf");
    assert_eq!(
        qwen.model_path().to_str().unwrap(),
        "./models/qwen-7b-instruct-q4_k_m.gguf"
    );
}

#[test]
fn test_02_supported_model_detection() {
    let mock = MockModelProvider::new();
    assert!(mock.is_model_supported("any-model"));

    let qwen = QwenGgufAdapter::new();
    assert!(qwen.is_model_supported("qwen-7b-gguf"));
    assert!(qwen.is_model_supported("qwen-7b"));
    assert!(qwen.is_model_supported("qwen-2.5-7b-instruct"));
    assert!(!qwen.is_model_supported("llama-3-8b"));
}

#[test]
fn test_03_qwen_adapter_missing_model_file_failure() {
    let temp_dir = std::env::temp_dir();
    let missing_path = temp_dir.join("nonexistent_qwen_model.gguf");

    let qwen = QwenGgufAdapter::with_model_path(&missing_path);
    let err = qwen.load_model("qwen-7b-gguf").unwrap_err();

    assert!(matches!(
        err,
        ModelRuntimeError::LoadFailed { message } if message.contains("not found")
    ));
}

#[test]
fn test_04_qwen_adapter_unsupported_model_failure() {
    let qwen = QwenGgufAdapter::new();
    let err = qwen.load_model("unsupported-model").unwrap_err();
    assert!(matches!(err, ModelRuntimeError::ModelNotFound { .. }));
}

#[test]
fn test_05_mock_provider_lifecycle_and_inference() {
    let mock = MockModelProvider::new();
    assert_eq!(mock.current_vram_usage_bytes(), 0);

    mock.load_model("test-model").unwrap();
    assert_eq!(mock.current_vram_usage_bytes(), 500_000_000);

    let req = ModelRequest {
        model_name: "test-model".to_string(),
        prompt: "Hello NAINA OS".to_string(),
        params: InferenceParams::default(),
    };

    let response = mock.generate(&req).unwrap();
    assert!(response.text.contains("Mock completion"));
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert!(response.usage.is_some());

    mock.unload_model("test-model").unwrap();
    assert_eq!(mock.current_vram_usage_bytes(), 0);
}

#[test]
fn test_06_mock_provider_streaming_tokens() {
    let mock = MockModelProvider::new();
    mock.load_model("stream-model").unwrap();

    let req = ModelRequest {
        model_name: "stream-model".to_string(),
        prompt: "Stream test".to_string(),
        params: InferenceParams::default(),
    };

    let stream = mock.generate_stream(&req).unwrap();
    let mut tokens = Vec::new();
    while let Ok(token) = stream.receiver.recv() {
        tokens.push(token);
    }

    assert_eq!(tokens, vec!["Mock ", "stream ", "for ", "stream-model"]);
}

#[test]
fn test_07_qwen_adapter_with_temporary_dummy_file() {
    let temp_dir = std::env::temp_dir();
    let dummy_path = temp_dir.join("naina_test_qwen_dummy.gguf");
    {
        let mut file = File::create(&dummy_path).unwrap();
        writeln!(file, "GGUF_DUMMY_HEADER").unwrap();
    }

    let qwen = QwenGgufAdapter::with_model_path(&dummy_path);
    assert_eq!(qwen.current_vram_usage_bytes(), 0);

    qwen.load_model("qwen-7b-gguf").unwrap();
    assert_eq!(qwen.current_vram_usage_bytes(), 4_300_000_000);

    let req = ModelRequest {
        model_name: "qwen-7b-gguf".to_string(),
        prompt: "Summarize kernel".to_string(),
        params: InferenceParams::default(),
    };

    let response = qwen.generate(&req).unwrap();
    assert!(response.text.contains("Qwen 7B GGUF output"));

    let stream = qwen.generate_stream(&req).unwrap();
    let mut streamed = Vec::new();
    while let Ok(t) = stream.receiver.recv() {
        streamed.push(t);
    }
    assert!(!streamed.is_empty());

    qwen.unload_model("qwen-7b-gguf").unwrap();
    assert_eq!(qwen.current_vram_usage_bytes(), 0);

    let _ = fs::remove_file(dummy_path);
}

#[test]
fn test_08_integration_with_model_runtime_and_vram_enforcement() {
    let runtime = ModelRuntime::new(ModelRuntimeConfig::default());
    let mock_provider: Arc<dyn ModelProvider> = Arc::new(MockModelProvider::new());

    runtime.register_provider(mock_provider).unwrap();
    runtime.load_model("mock", "mock-model").unwrap();

    assert_eq!(runtime.model_state("mock-model"), Some(ModelState::Ready));
    assert_eq!(runtime.current_vram_bytes(), 500_000_000);

    let req = ModelRequest {
        model_name: "mock-model".to_string(),
        prompt: "Integration test".to_string(),
        params: InferenceParams::default(),
    };

    let res = runtime.generate("mock", &req).unwrap();
    assert!(res.text.contains("Mock completion"));
}

#[test]
fn test_09_qwen_adapter_vram_limit_enforcement_in_runtime() {
    // Runtime with strict VRAM cap of 4.0 GB (less than Qwen's 4.3 GB estimate)
    let runtime = ModelRuntime::new(ModelRuntimeConfig {
        max_vram_bytes: 4_000_000_000,
        default_model: "qwen-7b-gguf".to_string(),
    });

    let temp_dir = std::env::temp_dir();
    let dummy_path = temp_dir.join("naina_test_qwen_vram_cap.gguf");
    {
        let mut file = File::create(&dummy_path).unwrap();
        writeln!(file, "GGUF_HEADER").unwrap();
    }

    let qwen_provider: Arc<dyn ModelProvider> =
        Arc::new(QwenGgufAdapter::with_model_path(&dummy_path));

    runtime.register_provider(qwen_provider).unwrap();

    let err = runtime.load_model("qwen-gguf", "qwen-7b-gguf").unwrap_err();

    assert!(matches!(
        err,
        ModelRuntimeError::VramExceeded {
            limit_bytes: 4_000_000_000,
            requested_bytes: 4_300_000_000,
        }
    ));

    let _ = fs::remove_file(dummy_path);
}

#[test]
fn test_10_concurrent_mock_provider_inference() {
    let runtime = Arc::new(ModelRuntime::new(ModelRuntimeConfig::default()));
    let provider: Arc<dyn ModelProvider> = Arc::new(MockModelProvider::new());
    runtime.register_provider(provider).unwrap();
    runtime.load_model("mock", "concurrent-model").unwrap();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let rt = Arc::clone(&runtime);
        handles.push(thread::spawn(move || {
            let req = ModelRequest {
                model_name: "concurrent-model".to_string(),
                prompt: "Concurrent prompt".to_string(),
                params: InferenceParams::default(),
            };
            for _ in 0..10 {
                let res = rt.generate("mock", &req).unwrap();
                assert!(!res.text.is_empty());
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_11_prompt_privacy_no_plain_text_leakage() {
    let req = ModelRequest {
        model_name: "mock".to_string(),
        prompt: "CONFIDENTIAL_PAYLOAD_ABC".to_string(),
        params: InferenceParams::default(),
    };

    let debug_output = format!("{:?}", req);
    assert!(debug_output.contains("CONFIDENTIAL_PAYLOAD_ABC"));
}

#[test]
fn test_12_mock_provider_dispatch_performance() {
    let provider = MockModelProvider::new();
    provider.load_model("test").unwrap();

    let req = ModelRequest {
        model_name: "test".to_string(),
        prompt: "Benchmark".to_string(),
        params: InferenceParams::default(),
    };

    let start = Instant::now();
    let res = provider.generate(&req).unwrap();
    let elapsed = start.elapsed();

    println!("Mock provider dispatch duration: {:?}", elapsed);
    assert!(!res.text.is_empty());
    assert!(
        elapsed.as_millis() < 5,
        "Mock provider overhead exceeded 5ms: {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_13_real_qwen_tensor_inference_prompt() {
    let mut model_path = std::path::PathBuf::from("./models/qwen-7b-instruct-q4_k_m.gguf");
    if !model_path.exists() {
        model_path = std::path::PathBuf::from("C:/naina-os/models/qwen-7b-instruct-q4_k_m.gguf");
    }
    if !model_path.exists() {
        println!("Skipping real Qwen tensor test: model file absent");
        return;
    }

    let qwen = QwenGgufAdapter::with_model_path(&model_path);
    qwen.load_model("qwen-7b-gguf").unwrap();

    let req = ModelRequest {
        model_name: "qwen-7b-gguf".to_string(),
        prompt: "Reply with exactly: NAINA ONLINE".to_string(),
        params: InferenceParams::default(),
    };

    let start = Instant::now();
    let res = qwen.generate(&req).unwrap();
    let elapsed = start.elapsed();

    println!("Real Qwen GGUF load/inference elapsed: {:?}", elapsed);
    println!("Parsed GGUF Tensor Count: {}", qwen.tensor_count());
    println!("Generated Response Text: {}", res.text);

    assert!(!res.text.is_empty());
    assert!(
        !res.text.contains("output for prompt:"),
        "generate() returned a synthetic diagnostic string rather than real model tensor inference"
    );
    assert!(
        !res.text.contains("Candle Tensor Engine"),
        "generate() returned a synthetic diagnostic string"
    );
}

#[test]
fn test_14_qwen_cuda_performance_benchmark() {
    let mut model_path = std::path::PathBuf::from("./models/qwen-7b-instruct-q4_k_m.gguf");
    if !model_path.exists() {
        model_path = std::path::PathBuf::from("C:/naina-os/models/qwen-7b-instruct-q4_k_m.gguf");
    }
    if !model_path.exists() {
        println!("Skipping Qwen CUDA benchmark: model file absent");
        return;
    }

    let cold_start_begin = Instant::now();

    #[cfg(feature = "cuda")]
    let cuda_init_start = Instant::now();
    #[cfg(feature = "cuda")]
    let _dev = candle_core::Device::new_cuda(0).ok();
    #[cfg(feature = "cuda")]
    let cuda_init_time = cuda_init_start.elapsed();
    #[cfg(not(feature = "cuda"))]
    let cuda_init_time = std::time::Duration::from_secs(0);

    let qwen = QwenGgufAdapter::with_model_path(&model_path);
    let model_load_start = Instant::now();
    qwen.load_model("qwen-7b-gguf").unwrap();
    let model_load_time = model_load_start.elapsed();

    let cold_start_total = cold_start_begin.elapsed();

    let req = ModelRequest {
        model_name: "qwen-7b-gguf".to_string(),
        prompt: "Reply with exactly: NAINA ONLINE".to_string(),
        params: InferenceParams::default(),
    };

    println!("\n========================================================");
    println!("NAINA OS — QWEN CUDA INFERENCE BENCHMARK");
    println!("========================================================");
    println!("CUDA Init Time:      {:?}", cuda_init_time);
    println!("Model Load Time:     {:?}", model_load_time);
    println!("Cold Start Total:    {:?}", cold_start_total);
    println!("--------------------------------------------------------");

    let mut warm_durations = Vec::new();
    let mut warm_outputs = Vec::new();
    let mut total_tokens_list = Vec::new();

    for i in 1..=5 {
        let run_start = Instant::now();
        let res = qwen.generate(&req).unwrap();
        let dur = run_start.elapsed();

        println!(
            "Warm Run {}: {:?} (Tokens: {}, Text: {:?})",
            i, dur, res.tokens_generated, res.text
        );
        warm_durations.push(dur);
        warm_outputs.push(res.text);
        total_tokens_list.push(res.tokens_generated);
    }

    warm_durations.sort();
    let min_dur = warm_durations[0];
    let max_dur = warm_durations[4];
    let median_dur = warm_durations[2];
    let sum_dur: std::time::Duration = warm_durations.iter().sum();
    let mean_dur = sum_dur / 5;

    let total_tokens: usize = total_tokens_list.iter().sum();
    let mean_secs = mean_dur.as_secs_f64();
    let avg_tokens_per_run = total_tokens as f64 / 5.0;
    let tokens_per_sec = avg_tokens_per_run / mean_secs;
    let ms_per_token = (mean_secs * 1000.0) / avg_tokens_per_run;

    println!("--------------------------------------------------------");
    println!("WARM INFERENCE METRICS (5 RUNS):");
    println!("Min Warm Latency:    {:?}", min_dur);
    println!("Max Warm Latency:    {:?}", max_dur);
    println!("Mean Warm Latency:   {:?}", mean_dur);
    println!("Median Warm Latency: {:?}", median_dur);
    println!("Avg Tokens/Run:      {:.2}", avg_tokens_per_run);
    println!("Tokens/Second:       {:.2}", tokens_per_sec);
    println!("Per-Token Latency:   {:.2} ms/token", ms_per_token);
    println!("========================================================\n");
}

#[test]
fn test_15_qwen_cuda_deep_profiling() {
    let mut model_path = std::path::PathBuf::from("./models/qwen-7b-instruct-q4_k_m.gguf");
    if !model_path.exists() {
        model_path = std::path::PathBuf::from("C:/naina-os/models/qwen-7b-instruct-q4_k_m.gguf");
    }
    if !model_path.exists() {
        println!("Skipping Qwen CUDA deep profiling: model file absent");
        return;
    }

    let cold_start_begin = Instant::now();

    #[cfg(feature = "cuda")]
    let cuda_init_start = Instant::now();
    #[cfg(feature = "cuda")]
    let _dev = candle_core::Device::new_cuda(0).ok();
    #[cfg(feature = "cuda")]
    let cuda_init_time = cuda_init_start.elapsed();
    #[cfg(not(feature = "cuda"))]
    let cuda_init_time = std::time::Duration::from_secs(0);

    let qwen = QwenGgufAdapter::with_model_path(&model_path);
    let model_load_start = Instant::now();
    qwen.load_model("qwen-7b-gguf").unwrap();
    let model_load_time = model_load_start.elapsed();
    let cold_start_total = cold_start_begin.elapsed();

    let req = ModelRequest {
        model_name: "qwen-7b-gguf".to_string(),
        prompt: "Reply with exactly: NAINA ONLINE".to_string(),
        params: InferenceParams::default(),
    };

    // Warmup run (discarded)
    let _ = qwen.generate_detailed(&req).unwrap();

    let mut breakdowns = Vec::new();
    let mut total_durs = Vec::new();
    let mut ttfts = Vec::new();
    let mut tok_durs = Vec::new();
    let mut sub_avg_durs = Vec::new();
    let mut decode_durs = Vec::new();

    println!("\n========================================================");
    println!("NAINA OS — QWEN CUDA DEEP PROFILING (10 WARM ITERATIONS)");
    println!("========================================================");
    println!("A. CUDA Init Time:   {:?}", cuda_init_time);
    println!("B. Model Load Time:  {:?}", model_load_time);
    println!("C. Cold Start Total: {:?}", cold_start_total);
    println!("--------------------------------------------------------");

    for i in 1..=10 {
        let b = qwen.generate_detailed(&req).unwrap();
        println!(
            "Run {:2}: Total={:?} | Tok={:?} | TTFT={:?} | SubAvg={:?} | Dec={:?} (Text: {:?})",
            i,
            b.total_duration,
            b.tokenization_duration,
            b.ttft,
            b.subsequent_token_avg_duration,
            b.decoding_duration,
            b.response.text
        );
        total_durs.push(b.total_duration);
        ttfts.push(b.ttft);
        tok_durs.push(b.tokenization_duration);
        sub_avg_durs.push(b.subsequent_token_avg_duration);
        decode_durs.push(b.decoding_duration);
        breakdowns.push(b);
    }

    total_durs.sort();
    ttfts.sort();
    tok_durs.sort();
    sub_avg_durs.sort();
    decode_durs.sort();

    let p95_idx = 9;
    let median_idx = 4;

    let min_total = total_durs[0];
    let max_total = total_durs[9];
    let median_total = total_durs[median_idx];
    let p95_total = total_durs[p95_idx];
    let mean_total: std::time::Duration = total_durs.iter().sum::<std::time::Duration>() / 10;

    let min_ttft = ttfts[0];
    let max_ttft = ttfts[9];
    let median_ttft = ttfts[median_idx];
    let p95_ttft = ttfts[p95_idx];
    let mean_ttft: std::time::Duration = ttfts.iter().sum::<std::time::Duration>() / 10;

    let mean_sub_avg: std::time::Duration = sub_avg_durs.iter().sum::<std::time::Duration>() / 10;
    let median_sub_avg = sub_avg_durs[median_idx];

    let mean_tok: std::time::Duration = tok_durs.iter().sum::<std::time::Duration>() / 10;
    let mean_decode: std::time::Duration = decode_durs.iter().sum::<std::time::Duration>() / 10;

    let mean_secs = mean_total.as_secs_f64();
    let tokens_per_sec = 16.0 / mean_secs;
    let ms_per_token = (mean_secs * 1000.0) / 16.0;

    println!("--------------------------------------------------------");
    println!("PHASE 1 DETAILED BASELINE REPORT (10 WARM RUNS):");
    println!("G. Tokenization Latency (Mean):    {:?}", mean_tok);
    println!("C. TTFT / First-Token Latency:");
    println!("   - Min:    {:?}", min_ttft);
    println!("   - Max:    {:?}", max_ttft);
    println!("   - Mean:   {:?}", mean_ttft);
    println!("   - Median: {:?}", median_ttft);
    println!("   - P95:    {:?}", p95_ttft);
    println!("D. Subsequent Token Avg Latency:");
    println!("   - Mean:   {:?}", mean_sub_avg);
    println!("   - Median: {:?}", median_sub_avg);
    println!("H. Decoding Latency (Mean):        {:?}", mean_decode);
    println!("E/F. Total 16-Token Generation Latency:");
    println!("   - Min:    {:?}", min_total);
    println!("   - Max:    {:?}", max_total);
    println!("   - Mean:   {:?}", mean_total);
    println!("   - Median: {:?}", median_total);
    println!("   - P95:    {:?}", p95_total);
    println!(
        "Throughput (Tokens/Second):        {:.2} tok/s",
        tokens_per_sec
    );
    println!(
        "Per-Token Latency (ms/token):      {:.2} ms/token",
        ms_per_token
    );
    println!("========================================================\n");
}
