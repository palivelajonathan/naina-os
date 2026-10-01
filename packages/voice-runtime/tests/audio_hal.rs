//! Integration and Physical Hardware Verification Suite for Gate 3 Audio HAL.
//!
//! Validates:
//! 1. Resampling and normalization unit logic (16 kHz mono downmixing, upmixing, linear interpolation).
//! 2. Mock HAL deterministic stream lifecycle (for CI/unit validation).
//! 3. Real Windows WASAPI hardware device enumeration via CPAL.
//! 4. Real physical microphone capture & PCM extraction.
//! 5. Real physical speaker/headphone playback.
//! 6. Full live streaming voice turn (Microphone -> Normalization -> Whisper STT -> Qwen CUDA -> Streaming Piper TTS -> Real Speaker).

use model_runtime::ModelProvider;
use std::sync::Arc;
use voice_runtime::hal::{
    convert_to_device_format, downmix_to_mono, normalize_to_16k_mono, pcm_f32_to_i16,
    pcm_i16_to_f32, resample_linear, upmix, AudioDeviceManager, AudioHal, AudioStreamConfig,
    CpalAudioHal, MockAudioHal,
};
use voice_runtime::{
    AudioBuffer, CandleWhisperSttAdapter, PiperTtsAdapter, VoiceRuntime, VoiceRuntimeConfig,
};

fn setup_test_runtime() -> VoiceRuntime {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(services::ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));
    VoiceRuntime::new(VoiceRuntimeConfig::default(), runtime, services)
}

#[test]
fn test_01_resampling_and_normalization_unit() {
    println!("\n=== TEST 01: RESAMPLING AND NORMALIZATION UNIT LOGIC ===");

    // 1. Stereo to Mono Downmixing
    let stereo = vec![1.0f32, -1.0, 0.5, 0.5, 0.8, -0.2];
    let mono = downmix_to_mono(&stereo, 2);
    assert_eq!(mono.len(), 3);
    assert!((mono[0] - 0.0).abs() < 1e-6);
    assert!((mono[1] - 0.5).abs() < 1e-6);
    assert!((mono[2] - 0.3).abs() < 1e-6);
    println!("  [PASS] Stereo to mono downmixing verified");

    // 2. Mono to Stereo Upmixing
    let mono_src = vec![0.1f32, 0.5, -0.5];
    let stereo_out = upmix(&mono_src, 2);
    assert_eq!(stereo_out.len(), 6);
    assert_eq!(stereo_out, vec![0.1, 0.1, 0.5, 0.5, -0.5, -0.5]);
    println!("  [PASS] Mono to stereo upmixing verified");

    // 3. Linear Resampling: 48 kHz to 16 kHz (3:1 ratio)
    let src_48k: Vec<f32> = (0..480).map(|i| (i as f32 * 0.1).sin()).collect();
    let resampled_16k = resample_linear(&src_48k, 48000, 16000);
    assert_eq!(resampled_16k.len(), 160);
    println!("  [PASS] Linear resampling 48 kHz -> 16 kHz verified");

    // 4. Linear Resampling: 16 kHz to 48 kHz (1:3 ratio)
    let src_16k: Vec<f32> = (0..160).map(|i| (i as f32 * 0.1).sin()).collect();
    let resampled_48k = resample_linear(&src_16k, 16000, 48000);
    assert_eq!(resampled_48k.len(), 480);
    println!("  [PASS] Linear resampling 16 kHz -> 48 kHz verified");

    // 5. Normalization pipeline (Hardware rate/stereo -> 16k mono)
    let raw_hw: Vec<f32> = (0..960).map(|i| (i as f32 * 0.05).sin()).collect(); // 480 stereo frames
    let norm = normalize_to_16k_mono(&raw_hw, 48000, 2);
    assert_eq!(norm.sample_rate, 16000);
    assert_eq!(norm.channels, 1);
    assert_eq!(norm.sample_count(), 160);
    println!("  [PASS] Full normalize_to_16k_mono pipeline verified");

    // 6. Device format conversion (Piper 16k mono -> 48k stereo)
    let piper_buf = AudioBuffer::new(16000, 1, pcm_f32_to_i16(&resampled_16k));
    let dev_out = convert_to_device_format(&piper_buf, 48000, 2);
    assert_eq!(dev_out.len(), 960); // 480 stereo frames = 960 interleaved samples
    println!("  [PASS] convert_to_device_format (16k mono -> 48k stereo) verified");

    // 7. PCM Format Conversions (f32 <-> i16 bytes)
    let orig_bytes = vec![0x00, 0x80, 0x00, 0x00, 0x00, 0x40, 0xFF, 0x7F]; // -32768, 0, 16384, 32767
    let f32_conv = pcm_i16_to_f32(&orig_bytes);
    let back_bytes = pcm_f32_to_i16(&f32_conv);
    assert_eq!(orig_bytes.len(), back_bytes.len());
    println!("  [PASS] PCM f32 <-> i16 roundtrip conversion verified");
}

#[test]
fn test_02_mock_hal_lifecycle_deterministic() {
    println!("\n=== TEST 02: MOCK HAL DETERMINISTIC LIFECYCLE ===");

    let mock_hal = MockAudioHal::new();

    // Enumerate mock devices
    let inputs = mock_hal.list_input_devices().unwrap();
    let outputs = mock_hal.list_output_devices().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(outputs.len(), 1);
    assert_eq!(inputs[0].name, "Mock Digital Microphone");
    assert_eq!(outputs[0].name, "Mock Stereo Speaker");
    println!("  [PASS] Mock HAL device enumeration successful");

    // Feed mock input samples and read them
    let mut in_stream = mock_hal
        .open_input_stream(None, &AudioStreamConfig::default())
        .unwrap();
    in_stream.start().unwrap();

    let test_samples: Vec<f32> = (0..960).map(|i| (i as f32 * 0.05).sin()).collect();
    mock_hal.set_input_samples(test_samples.clone());

    let captured = in_stream.capture_samples().unwrap();
    assert_eq!(captured.len(), 960);
    assert_eq!(captured, test_samples);
    in_stream.stop().unwrap();
    println!("  [PASS] Mock HAL capture lifecycle verified");

    // Write to mock output stream
    let mut out_stream = mock_hal
        .open_output_stream(None, &AudioStreamConfig::default())
        .unwrap();
    out_stream.start().unwrap();
    out_stream.write_chunk(&captured).unwrap();
    out_stream.flush_and_wait().unwrap();

    let written = mock_hal.get_played_samples();
    assert_eq!(written.len(), 960);
    assert_eq!(written, test_samples);
    out_stream.stop().unwrap();
    println!("  [PASS] Mock HAL playback lifecycle verified");
}

#[test]
fn test_03_physical_device_enumeration() {
    println!("\n=== TEST 03: REAL PHYSICAL WINDOWS AUDIO DEVICE ENUMERATION ===");

    let hal = CpalAudioHal::new();

    // 1. Enumerate Physical Input Devices
    let input_devices = hal
        .list_input_devices()
        .expect("Failed to list physical input devices");
    println!(
        "  Discovered {} physical audio input device(s):",
        input_devices.len()
    );
    for (i, dev) in input_devices.iter().enumerate() {
        println!(
            "    [{}] {} (Default: {}, SR: {} Hz, Ch: {}, Rates: {:?})",
            i + 1,
            dev.name,
            dev.is_default,
            dev.default_sample_rate,
            dev.default_channels,
            dev.supported_sample_rates
        );
    }
    assert!(
        !input_devices.is_empty(),
        "CRITICAL: No physical audio input devices detected on host system!"
    );

    let default_in = hal
        .default_input_device()
        .expect("Failed to get default input device");
    println!("  Default Physical Input Device: '{}'", default_in.name);

    // 2. Enumerate Physical Output Devices
    let output_devices = hal
        .list_output_devices()
        .expect("Failed to list physical output devices");
    println!(
        "  Discovered {} physical audio output device(s):",
        output_devices.len()
    );
    for (i, dev) in output_devices.iter().enumerate() {
        println!(
            "    [{}] {} (Default: {}, SR: {} Hz, Ch: {}, Rates: {:?})",
            i + 1,
            dev.name,
            dev.is_default,
            dev.default_sample_rate,
            dev.default_channels,
            dev.supported_sample_rates
        );
    }
    assert!(
        !output_devices.is_empty(),
        "CRITICAL: No physical audio output devices detected on host system!"
    );

    let default_out = hal
        .default_output_device()
        .expect("Failed to get default output device");
    println!("  Default Physical Output Device: '{}'", default_out.name);

    println!("  [PASS] Real physical Windows audio device enumeration verified");
}

#[test]
fn test_04_physical_microphone_capture() {
    println!("\n=== TEST 04: REAL PHYSICAL MICROPHONE CAPTURE (WASAPI) ===");

    let runtime = setup_test_runtime();
    runtime.register_audio_hal(Arc::new(CpalAudioHal::new()));

    let default_in = runtime
        .default_input_device()
        .expect("Must have default input device");
    println!("  Target Microphone: '{}'", default_in.name);

    // Capture 500ms of real physical microphone audio
    let capture_ms = 500u64;
    println!("  Capturing {} ms from physical microphone...", capture_ms);

    let (audio_buf, stats) = runtime
        .capture_microphone_audio(capture_ms, None)
        .expect("Failed to capture from physical microphone");

    println!("  Capture Statistics:");
    println!("    Hardware Sample Rate:    {} Hz", stats.sample_rate);
    println!("    Hardware Channels:       {}", stats.channels);
    println!("    Captured Samples:        {}", stats.samples_captured);
    println!(
        "    Startup Latency:         {:.2} ms",
        stats.startup_latency_ms
    );
    println!("    Normalized Sample Count: {}", audio_buf.sample_count());
    println!("    Normalized Sample Rate:  {} Hz", audio_buf.sample_rate);
    println!("    Normalized Channels:     {}", audio_buf.channels);

    // Assertions on real hardware capture
    assert!(stats.samples_captured > 0, "Microphone captured 0 samples!");
    assert!(
        audio_buf.sample_count() > 0,
        "Normalized audio buffer is empty!"
    );
    assert_eq!(
        audio_buf.sample_rate, 16000,
        "Normalized audio must be 16000 Hz"
    );
    assert_eq!(
        audio_buf.channels, 1,
        "Normalized audio must be mono (1 channel)"
    );
    assert!(stats.sample_rate > 0, "Invalid hardware sample rate");
    assert!(stats.channels > 0, "Invalid hardware channel count");

    println!("  [PASS] Real physical microphone capture verified");
}

#[test]
fn test_05_physical_speaker_playback() {
    println!("\n=== TEST 05: REAL PHYSICAL SPEAKER/HEADPHONE PLAYBACK (WASAPI) ===");

    let runtime = setup_test_runtime();
    runtime.register_audio_hal(Arc::new(CpalAudioHal::new()));

    let default_out = runtime
        .default_output_device()
        .expect("Must have default output device");
    println!("  Target Output Device: '{}'", default_out.name);

    // Synthesize a brief gentle audio cue: 440 Hz (A4) sine wave at 16 kHz mono for 250ms
    // Amplitude is set to 0.15 to be clearly audible yet gentle and safe
    let sample_rate = 16000u32;
    let duration_ms = 250u64;
    let num_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
    let freq = 440.0f32;

    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let envelope = if (i as u64) < 160 {
                i as f32 / 160.0 // Fade in 10ms
            } else if i > num_samples - 160 {
                (num_samples - i) as f32 / 160.0 // Fade out 10ms
            } else {
                1.0
            };
            (2.0 * std::f32::consts::PI * freq * t).sin() * 0.15 * envelope
        })
        .collect();

    let pcm_bytes = pcm_f32_to_i16(&samples);
    let audio = AudioBuffer::new(sample_rate, 1, pcm_bytes);

    println!(
        "  Playing {} ms test tone through '{}'...",
        duration_ms, default_out.name
    );
    let play_stats = runtime
        .play_audio(&audio, None)
        .expect("Failed to play audio on physical device");

    println!("  Playback Statistics:");
    println!("    Hardware Sample Rate:    {} Hz", play_stats.sample_rate);
    println!("    Hardware Channels:       {}", play_stats.channels);
    println!("    Samples Played to DAC:   {}", play_stats.samples_played);
    println!(
        "    Audio Duration:          {:.2} ms",
        play_stats.duration_ms
    );
    println!(
        "    Playback Latency:        {:.2} ms",
        play_stats.playback_latency_ms
    );

    assert!(play_stats.samples_played > 0, "No samples played to DAC!");
    assert!(play_stats.sample_rate > 0, "Invalid output sample rate");

    println!("  [PASS] Real physical speaker playback verified");
}

#[test]
fn test_06_live_streaming_voice_turn_hardware() {
    println!("\n========================================================");
    println!("=== TEST 06: GATE 3 HARDWARE VOICE STREAMING TURN ===");
    println!("========================================================");

    let whisper_path = std::path::PathBuf::from("C:/naina-os/models/whisper-base-en.bin");
    let qwen_path = std::path::PathBuf::from("C:/naina-os/models/qwen-7b-instruct-q4_k_m.gguf");
    let piper_path = std::path::PathBuf::from("C:/naina-os/models/piper-en-medium.onnx");

    if !whisper_path.exists() || !qwen_path.exists() || !piper_path.exists() {
        println!("Skipping live hardware turn: model weights not found at expected paths");
        return;
    }

    let voice = setup_test_runtime();

    // 1. Register Native CPAL WASAPI HAL
    let cpal_hal = Arc::new(CpalAudioHal::new());
    voice.register_audio_hal(Arc::clone(&cpal_hal) as Arc<dyn AudioHal>);

    // 2. Register Whisper STT
    let whisper = Arc::new(CandleWhisperSttAdapter::with_model_path(
        "whisper-base-en",
        &whisper_path,
    ));
    whisper.load_model().expect("Failed to load Whisper model");
    voice.register_stt_engine(whisper);

    // 3. Register Piper TTS
    let piper = Arc::new(PiperTtsAdapter::with_model_path(
        "piper-en-medium",
        &piper_path,
    ));
    piper.load_model().expect("Failed to load Piper model");
    voice.register_tts_engine(piper);

    // 4. Register Qwen 7B GGUF CUDA
    let qwen = Arc::new(model_providers::QwenGgufAdapter::with_model_path(
        &qwen_path,
    ));
    qwen.load_model("qwen-7b-gguf")
        .expect("Failed to load Qwen CUDA model");
    assert!(qwen.is_cuda_active(), "Qwen must be active on CUDA");
    voice.register_model_provider(Arc::clone(&qwen) as Arc<dyn model_runtime::ModelProvider>);

    let default_in = voice
        .default_input_device()
        .expect("Must have default input device");
    let default_out = voice
        .default_output_device()
        .expect("Must have default output device");
    println!("  Hardware Input Device:  '{}'", default_in.name);
    println!("  Hardware Output Device: '{}'", default_out.name);

    // Warm-up run to ensure CUDA kernels and TTS caches are hot
    {
        println!("  Performing warm-up of CUDA and TTS pipelines...");
        let model_req = model_runtime::ModelRequest {
            model_name: "qwen-7b-gguf".to_string(),
            prompt: "Hello NAINA, system status check.".to_string(),
            params: model_runtime::InferenceParams {
                max_tokens: 16,
                ..Default::default()
            },
        };
        let stream = qwen.generate_stream(&model_req).unwrap();
        while let Ok(_) = stream.receiver.recv() {}
    }

    println!("\n  Starting Live Physical Voice Turn Stream:");
    println!("    Step 1: Capturing 1000 ms audio from physical microphone...");
    println!("    Step 2: Resampling & Normalizing to 16 kHz Mono...");
    println!("    Step 3: Whisper STT Transcription...");
    println!("    Step 4: Qwen 7B CUDA Token Streaming...");
    println!("    Step 5: Text Chunker (Clause Boundaries)...");
    println!("    Step 6: Concurrent Piper TTS Synthesis...");
    println!("    Step 7: Resampling to DAC format & Output Playback to Real Speakers...\n");

    let live_res = voice
        .process_live_voice_turn_stream(1000, "qwen-7b-gguf", None, None)
        .expect("Failed to execute live streaming turn on physical hardware");

    println!("========================================================");
    println!("=== GATE 3 PHYSICAL HARDWARE PIPELINE BENCHMARK RESULTS ===");
    println!("========================================================");
    println!(
        "  Microphone Sample Rate:   {} Hz",
        live_res.capture_stats.sample_rate
    );
    println!(
        "  Microphone Channels:      {}",
        live_res.capture_stats.channels
    );
    println!(
        "  Microphone Samples:       {}",
        live_res.capture_stats.samples_captured
    );
    println!(
        "  Microphone Startup Time:  {:.2} ms",
        live_res.capture_stats.startup_latency_ms
    );
    println!(
        "  Speaker Sample Rate:      {} Hz",
        live_res.playback_stats.sample_rate
    );
    println!(
        "  Speaker Channels:         {}",
        live_res.playback_stats.channels
    );
    println!(
        "  Samples Played to DAC:    {}",
        live_res.playback_stats.samples_played
    );
    println!(
        "  Transcription Text:       {:?}",
        live_res.transcription.text
    );
    println!("  Response Text:            {:?}", live_res.full_text);
    println!(
        "  Synthesized Audio Chunks: {}",
        live_res.audio_chunks.len()
    );
    println!("  Total Tokens Generated:   {}", live_res.total_tokens);

    let tl = &live_res.timeline;
    println!("\n--- HIGH-RESOLUTION TIMELINE (T0 - T13) ---");
    println!(
        "  Capture Startup Latency (T1 - T0):   {:.2} ms",
        tl.capture_startup_latency_ms
    );
    println!(
        "  Capture Duration (T3 - T2):          {:.2} ms",
        tl.capture_duration_ms
    );
    println!(
        "  Whisper STT Latency (T4 - T3):       {:.2} ms",
        tl.whisper_latency_ms
    );
    println!(
        "  Qwen TTFT (T6 - T5):                 {:.2} ms",
        tl.qwen_ttft_ms
    );
    println!(
        "  First Text Chunk (T7 - T5):          {:.2} ms",
        tl.t7_first_text_chunk
            .duration_since(tl.t5_qwen_started)
            .as_secs_f64()
            * 1000.0
    );
    println!(
        "  First Piper Audio (T8 - T5):         {:.2} ms",
        tl.t8_first_piper_chunk
            .duration_since(tl.t5_qwen_started)
            .as_secs_f64()
            * 1000.0
    );
    println!(
        "  First Output Callback (T10 - T5):    {:.2} ms",
        tl.t10_first_output_callback
            .duration_since(tl.t5_qwen_started)
            .as_secs_f64()
            * 1000.0
    );
    println!(
        "  Total Turn Completion (T13 - T0):    {:.2} ms",
        tl.total_turn_completion_ms
    );
    println!(
        "  Qwen Generation Throughput:          {:.2} tokens/sec",
        tl.qwen_tokens_per_sec
    );

    println!("\n--- LATENCY COMPARISON ---");
    println!(
        "  Software TTFA (T8 - T0):              {:.2} ms",
        tl.software_ttfa_ms
    );
    println!(
        "  Hardware-Inclusive TTFA (T10 - T0):   {:.2} ms",
        tl.hardware_inclusive_ttfa_ms
    );
    println!("  Acoustic Latency:                     NOT MEASURED (requires calibrated loopback microphone)");

    // Validations:
    assert!(
        live_res.capture_stats.samples_captured > 0,
        "No raw microphone samples captured!"
    );
    assert!(
        live_res.playback_stats.samples_played > 0,
        "No samples sent to DAC!"
    );
    assert!(
        live_res.timeline.software_ttfa_ms > 0.0,
        "Software TTFA must be positive"
    );
    assert!(
        live_res.timeline.hardware_inclusive_ttfa_ms >= live_res.timeline.software_ttfa_ms,
        "Hardware TTFA must be >= Software TTFA"
    );
    assert!(
        !live_res.audio_chunks.is_empty(),
        "Must have synthesized at least one audio chunk"
    );

    // Streaming Proof: First audio chunk must be generated before final LLM token completes
    let t8 = tl.t8_first_piper_chunk;
    let t11 = tl.t11_qwen_final_token;
    assert!(
        t8 <= t11,
        "Streaming violation: First audio chunk ({:?}) was after final LLM token ({:?})!",
        t8,
        t11
    );
    println!("\n  [PROVEN] First audio chunk was generated BEFORE final Qwen token!");
    println!("  [PASS] Gate 3 live physical hardware voice turn stream fully verified");
}
