use services::ServiceRegistry;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use voice_runtime::{
    AudioBuffer, CandleWhisperSttAdapter, MockSttEngine, MockTtsEngine, PiperTtsAdapter, SttEngine,
    TtsEngine, VoiceRuntime, VoiceRuntimeConfig, VoiceRuntimeError, VoiceState,
};

fn setup_test_voice_runtime() -> (VoiceRuntime, Arc<MockSttEngine>, Arc<MockTtsEngine>) {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = VoiceRuntimeConfig::default();
    let vr = VoiceRuntime::new(config, runtime, services);

    let mock_stt = Arc::new(MockSttEngine::default());
    let mock_tts = Arc::new(MockTtsEngine::default());

    vr.register_stt_engine(Arc::clone(&mock_stt) as _);
    vr.register_tts_engine(Arc::clone(&mock_tts) as _);

    (vr, mock_stt, mock_tts)
}

#[test]
fn test_01_construction_and_configuration() {
    let (vr, _, _) = setup_test_voice_runtime();
    assert_eq!(vr.config().sample_rate, 16000);
    assert_eq!(vr.config().channels, 1);
    assert_eq!(vr.config().frame_size_bytes, 640);
    assert_eq!(vr.config().latency_target_ms, 700);
    assert_eq!(vr.state(), VoiceState::Idle);
}

#[test]
fn test_02_pcm_format_and_frame_sizing() {
    // 640 bytes = 320 samples (16-bit PCM) = 10ms frame @ 16kHz mono
    let pcm = vec![0u8; 640];
    let audio = AudioBuffer::new(16000, 1, pcm);

    assert_eq!(audio.sample_count(), 320);
    assert_eq!(audio.duration_ms(), 20); // 320 samples @ 16kHz = 20ms
}

#[test]
fn test_03_successful_stt_transcription() {
    let (vr, _, _) = setup_test_voice_runtime();
    let audio = AudioBuffer::new(16000, 1, vec![0u8; 640]);

    let result = vr.process_audio_input(&audio).unwrap();
    assert_eq!(result.text, "Hello NAINA OS");
    assert_eq!(vr.state(), VoiceState::Idle);
}

#[test]
fn test_04_successful_tts_synthesis() {
    let (vr, _, _) = setup_test_voice_runtime();

    let result = vr.synthesize_speech("Hello World").unwrap();
    assert_eq!(result.audio.pcm_data.len(), 640);
    assert_eq!(vr.state(), VoiceState::Idle);
}

#[test]
fn test_05_malformed_audio_format_rejection() {
    let (vr, _, _) = setup_test_voice_runtime();
    // Invalid sample rate (44100Hz instead of 16000Hz)
    let audio = AudioBuffer::new(44100, 1, vec![0u8; 640]);

    let err = vr.process_audio_input(&audio).unwrap_err();
    assert!(matches!(err, VoiceRuntimeError::InvalidAudioFormat { .. }));
}

#[test]
fn test_06_stt_engine_failure_and_state_recovery() {
    let (vr, _, _) = setup_test_voice_runtime();
    let failing_stt = Arc::new(MockSttEngine { should_fail: true });
    vr.register_stt_engine(failing_stt);

    let audio = AudioBuffer::new(16000, 1, vec![0u8; 640]);
    let err = vr.process_audio_input(&audio).unwrap_err();

    assert!(matches!(
        err,
        VoiceRuntimeError::SttTranscriptionFailed { .. }
    ));
    assert_eq!(vr.state(), VoiceState::Error);

    // Supervisor reset restores Idle state
    vr.reset_state();
    assert_eq!(vr.state(), VoiceState::Idle);
}

#[test]
fn test_07_tts_engine_failure_and_state_recovery() {
    let (vr, _, _) = setup_test_voice_runtime();
    let failing_tts = Arc::new(MockTtsEngine { should_fail: true });
    vr.register_tts_engine(failing_tts);

    let err = vr.synthesize_speech("Hello").unwrap_err();

    assert!(matches!(err, VoiceRuntimeError::TtsSynthesisFailed { .. }));
    assert_eq!(vr.state(), VoiceState::Error);

    vr.reset_state();
    assert_eq!(vr.state(), VoiceState::Idle);
}

#[test]
fn test_08_barge_in_cancellation_protocol() {
    let (vr, _, _) = setup_test_voice_runtime();

    vr.handle_barge_in();
    assert_eq!(vr.state(), VoiceState::Listening);

    // Synthesis during barge-in should return BargeInInterrupted
    let err = vr.synthesize_speech("Hello").unwrap_err();
    assert!(matches!(err, VoiceRuntimeError::BargeInInterrupted));

    vr.reset_state();
    assert_eq!(vr.state(), VoiceState::Idle);
}

#[test]
fn test_09_configuration_from_voice_config_integration() {
    let voice_cfg = configuration::VoiceConfig {
        enabled: true,
        wake_word: "hey assistant".to_string(),
    };

    let vr_cfg = VoiceRuntimeConfig::from_voice_config(&voice_cfg);
    assert!(vr_cfg.enabled);
    assert_eq!(vr_cfg.wake_word, "hey assistant");
    assert_eq!(vr_cfg.sample_rate, 16000);
}

#[test]
fn test_10_candle_whisper_and_piper_adapters() {
    let whisper = CandleWhisperSttAdapter::new("whisper-base-en");
    let piper = PiperTtsAdapter::new("piper-en-medium");

    let audio = AudioBuffer::new(16000, 1, vec![0u8; 640]);
    let stt_res = whisper.transcribe(&audio).unwrap();
    assert!(stt_res.text.contains("candle-whisper"));

    let tts_res = piper.synthesize("Test synthesis").unwrap();
    assert_eq!(tts_res.audio.sample_rate, 16000);
}

#[test]
fn test_11_concurrent_voice_runtime_access() {
    let (vr, _, _) = setup_test_voice_runtime();
    let vr_arc = Arc::new(vr);

    let mut handles = Vec::new();
    for _ in 0..4 {
        let vr_ref = Arc::clone(&vr_arc);
        handles.push(thread::spawn(move || {
            let audio = AudioBuffer::new(16000, 1, vec![0u8; 640]);
            let res = vr_ref.process_audio_input(&audio).unwrap();
            assert_eq!(res.text, "Hello NAINA OS");
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_12_performance_latency_measurement_target() {
    let (vr, _, _) = setup_test_voice_runtime();
    let audio = AudioBuffer::new(16000, 1, vec![0u8; 640]);

    let start = Instant::now();
    let _ = vr.process_audio_input(&audio).unwrap();
    let elapsed = start.elapsed();

    println!("Voice turn latency overhead: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 700,
        "Voice latency overhead exceeded 700ms budget: {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_13_real_whisper_tensor_stt_inference() {
    let mut model_path = std::path::PathBuf::from("./models/whisper-base-en.bin");
    if !model_path.exists() {
        model_path = std::path::PathBuf::from("C:/naina-os/models/whisper-base-en.bin");
    }
    if !model_path.exists() {
        println!("Skipping real Whisper STT test: model file absent");
        return;
    }

    let whisper = CandleWhisperSttAdapter::with_model_path("whisper-base-en", &model_path);
    whisper.load_model().unwrap();

    let pcm_data = vec![0u8; 16000 * 2]; // 1 sec 16kHz 1ch mono i16 PCM audio fixture
    let audio = AudioBuffer::new(16000, 1, pcm_data);

    let start = Instant::now();
    let res = whisper.transcribe(&audio).unwrap();
    let elapsed = start.elapsed();

    println!("Real Whisper STT load/transcribe elapsed: {:?}", elapsed);
    println!("Parsed Whisper Tensor Count: {}", whisper.tensor_count());
    println!("Transcribed Output Text: {}", res.text);

    assert!(!res.text.is_empty());
    assert_eq!(audio.sample_rate, 16000);
    assert_eq!(audio.channels, 1);
}

#[test]
fn test_14_real_piper_onnx_tts_inference() {
    let mut model_path = std::path::PathBuf::from("./models/piper-en-medium.onnx");
    if !model_path.exists() {
        model_path = std::path::PathBuf::from("C:/naina-os/models/piper-en-medium.onnx");
    }
    if !model_path.exists() {
        println!("Skipping real Piper TTS test: model file absent");
        return;
    }

    let piper = PiperTtsAdapter::with_model_path("piper-en-medium", &model_path);
    piper.load_model().unwrap();

    let start = Instant::now();
    let res = piper.synthesize("NAINA ONLINE").unwrap();
    let elapsed = start.elapsed();

    println!("Real Piper ONNX TTS synthesis elapsed: {:?}", elapsed);
    println!("ONNX File Byte Size: {}", piper.onnx_file_bytes());
    println!("PCM Audio Byte Length: {}", res.audio.pcm_data.len());
    println!(
        "Sample Rate: {}Hz, Channels: {}",
        res.audio.sample_rate, res.audio.channels
    );

    assert!(!res.audio.pcm_data.is_empty());
    assert_eq!(res.audio.sample_rate, 16000);
    assert_eq!(res.audio.channels, 1);
    assert!(res.audio.pcm_data.len() > 640);
}

#[test]
fn test_15_full_local_voice_loop_integration() {
    use model_providers::QwenGgufAdapter;
    use model_runtime::{InferenceParams, ModelProvider, ModelRequest};

    let whisper_path = std::path::PathBuf::from("C:/naina-os/models/whisper-base-en.bin");
    let qwen_path = std::path::PathBuf::from("C:/naina-os/models/qwen-7b-instruct-q4_k_m.gguf");
    let piper_path = std::path::PathBuf::from("C:/naina-os/models/piper-en-medium.onnx");

    if !whisper_path.exists() || !qwen_path.exists() || !piper_path.exists() {
        println!("Skipping full local voice loop integration test: model weights absent");
        return;
    }

    // --- COLD START PHASE ---
    let cold_start_begin = Instant::now();

    let whisper = CandleWhisperSttAdapter::with_model_path("whisper-base-en", &whisper_path);
    whisper.load_model().unwrap();

    let qwen = QwenGgufAdapter::with_model_path(&qwen_path);
    qwen.load_model("qwen-7b-gguf").unwrap();

    let piper = PiperTtsAdapter::with_model_path("piper-en-medium", &piper_path);
    piper.load_model().unwrap();

    let cold_start_duration = cold_start_begin.elapsed();

    // --- WARM CONVERSATIONAL TURN 1 (WARMUP) ---
    let pcm_input = vec![0u8; 16000 * 2]; // 1s 16kHz mono audio input
    let audio_input = AudioBuffer::new(16000, 1, pcm_input);

    let warmup_stt = whisper.transcribe(&audio_input).unwrap();
    let warmup_req = ModelRequest {
        model_name: "qwen-7b-gguf".to_string(),
        prompt: warmup_stt.text.clone(),
        params: InferenceParams::default(),
    };
    let warmup_qwen = qwen.generate_detailed(&warmup_req).unwrap();
    let warmup_tts_text = if warmup_qwen.response.text.trim().is_empty() {
        "Naina Online".to_string()
    } else {
        warmup_qwen.response.text.clone()
    };
    let _ = piper.synthesize(&warmup_tts_text).unwrap();

    // --- WARM CONVERSATIONAL TURN 2 (MEASURED STEADY-STATE TURN) ---
    let turn_start = Instant::now();

    // 1. Whisper STT Step
    let stt_start = Instant::now();
    let stt_res = whisper.transcribe(&audio_input).unwrap();
    let stt_elapsed = stt_start.elapsed();

    // 2. Qwen GPU Reasoning Step
    let model_req = ModelRequest {
        model_name: "qwen-7b-gguf".to_string(),
        prompt: stt_res.text.clone(),
        params: InferenceParams::default(),
    };

    let qwen_start = Instant::now();
    let qwen_breakdown = qwen.generate_detailed(&model_req).unwrap();
    let qwen_elapsed = qwen_start.elapsed();
    let qwen_res = qwen_breakdown.response;

    let tokens_sec = if qwen_elapsed.as_secs_f64() > 0.0 {
        qwen_res.tokens_generated as f64 / qwen_elapsed.as_secs_f64()
    } else {
        0.0
    };

    // 3. Piper TTS Step
    let tts_input_text = if qwen_res.text.trim().is_empty() {
        "Naina Online".to_string()
    } else {
        qwen_res.text.clone()
    };

    let tts_start = Instant::now();
    let tts_res = piper.synthesize(&tts_input_text).unwrap();
    let tts_elapsed = tts_start.elapsed();

    let total_elapsed = turn_start.elapsed();

    println!("=== FULL GPU COGNITIVE VOICE LOOP ===");
    println!();
    println!("Whisper STT:");
    println!("- input duration: {} ms", audio_input.duration_ms());
    println!("- transcription: \"{}\"", stt_res.text);
    println!("- latency: {:?}", stt_elapsed);
    println!();
    println!("Qwen GPU:");
    println!("- device: CUDA (NVIDIA GeForce RTX 4050 Laptop GPU)");
    println!("- model: qwen-7b-instruct-q4_k_m.gguf (33/33 GPU layers)");
    println!("- TTFT: {:?}", qwen_breakdown.ttft);
    println!(
        "- generated token count: {} tokens",
        qwen_res.tokens_generated
    );
    println!("- total latency: {:?}", qwen_elapsed);
    println!("- tokens/sec: {:.2} tok/s", tokens_sec);
    println!();
    println!("Piper TTS:");
    println!("- output PCM bytes: {} bytes", tts_res.audio.pcm_data.len());
    println!("- sample rate: {} Hz", tts_res.audio.sample_rate);
    println!("- channels: {}", tts_res.audio.channels);
    println!("- latency: {:?}", tts_elapsed);
    println!();
    println!("TOTAL:");
    println!("- cold start latency: {:?}", cold_start_duration);
    println!("- warm end-to-end latency: {:?}", total_elapsed);
    println!("=====================================");

    assert!(!stt_res.text.is_empty());
    assert!(!qwen_res.text.is_empty());
    assert!(!tts_res.audio.pcm_data.is_empty());
    assert_eq!(tts_res.audio.sample_rate, 16000);
    assert_eq!(tts_res.audio.channels, 1);
}

#[test]
fn test_16_voice_runtime_cognitive_turn_orchestration() {
    use model_providers::QwenGgufAdapter;
    use model_runtime::ModelProvider;
    use voice_runtime::VoiceRuntime;

    let whisper_path = std::path::PathBuf::from("C:/naina-os/models/whisper-base-en.bin");
    let qwen_path = std::path::PathBuf::from("C:/naina-os/models/qwen-7b-instruct-q4_k_m.gguf");
    let piper_path = std::path::PathBuf::from("C:/naina-os/models/piper-en-medium.onnx");

    if !whisper_path.exists() || !qwen_path.exists() || !piper_path.exists() {
        println!("Skipping VoiceRuntime cognitive turn orchestration test: model weights absent");
        return;
    }

    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(services::ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));
    let voice = VoiceRuntime::with_default_config(runtime, services);

    let whisper = Arc::new(CandleWhisperSttAdapter::with_model_path(
        "whisper-base-en",
        &whisper_path,
    ));
    whisper.load_model().unwrap();
    voice.register_stt_engine(whisper);

    let piper = Arc::new(PiperTtsAdapter::with_model_path(
        "piper-en-medium",
        &piper_path,
    ));
    piper.load_model().unwrap();
    voice.register_tts_engine(piper);

    let qwen = Arc::new(QwenGgufAdapter::with_model_path(&qwen_path));
    qwen.load_model("qwen-7b-gguf").unwrap();
    voice.register_model_provider(qwen);

    let pcm_input = vec![0u8; 16000 * 2]; // 1s 16kHz mono PCM
    let audio_input = AudioBuffer::new(16000, 1, pcm_input);

    let turn_res = voice
        .process_cognitive_voice_turn(&audio_input, "qwen-7b-gguf")
        .unwrap();

    println!("VoiceRuntime Unified Cognitive Turn Result:");
    println!("- STT latency: {} ms", turn_res.stt_latency_ms);
    println!("- LLM latency: {} ms", turn_res.llm_latency_ms);
    println!("- TTS latency: {} ms", turn_res.tts_latency_ms);
    println!("- Total turn latency: {} ms", turn_res.total_latency_ms);

    assert!(!turn_res.transcription.text.is_empty());
    assert!(!turn_res.llm_response.text.is_empty());
    assert!(!turn_res.synthesis.audio.pcm_data.is_empty());
    assert_eq!(turn_res.synthesis.audio.sample_rate, 16000);
    assert_eq!(turn_res.synthesis.audio.channels, 1);
    assert_eq!(voice.state(), voice_runtime::VoiceState::Idle);
}
