# ADR-001: NAINA OS Voice Runtime Architecture

- **Title:** ADR-001: NAINA OS Voice Runtime Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-23
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/voice-runtime`

---

## 1. Status
**APPROVED**

*Implementation Permitted: YES*

---

## 2. Context & Responsibility

NAINA OS uses a modular microkernel architecture. The `packages/voice-runtime` package acts as the top-level voice processing engine and pipeline manager:
```
apps → voice-runtime → (runtime, services) → (kernel, capabilities, configuration, logging)
```

The primary responsibilities of `voice-runtime` are:
1. Capturing user microphone audio input.
2. Transcribing audio to text via Whisper STT model integration.
3. Synthesizing assistant text output to spoken audio via Piper TTS engine integration.
4. Supervising voice processing state transitions (`Idle`, `Listening`, `Transcribing`, `Synthesizing`, `Speaking`, `Error`).
5. Enforcing the strict voice-to-voice turn latency budget of `< 700 ms`.
6. Isolating audio hardware I/O and model inference threads from the host microkernel supervisor.

---

## 3. Requirements Classification (Source of Truth vs. ADR Decisions)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **First Alpha Specification (`FIRST_ALPHA_SPEC.md`)**:
  - Section 2 (MVN Core Pipeline): Voice Input (Whisper) -> CARF Planner -> Qwen GGUF -> Obsidian Vault -> Voice Output (Piper TTS).
  - Section 3: Performance Target (< 700ms voice-to-voice turn latency).
  - Section 6: Milestone Tracker Week 4 (Voice OS Pipeline: Whisper STT + Piper TTS Integration).
  - Section 7 Checklist (Item 3): Captures microphone input, converts via Whisper STT, speaks output via Piper TTS.
  - Section 7 Checklist (Item 4): Total voice-to-voice turn latency < 700ms.
  - Section 7 Checklist (Item 10): Automatically restarts failed worker processes without crashing the NKRS supervisor.
- **Package Rules & Dependency Map (`engineering/DEPENDENCY_MAP.md`)**:
  - `voice-runtime` depends directly on `runtime`, `services`, `configuration`, and `logging`.
  - `orchestrator`, `model-runtime`, `model-providers`, `memory`, `context-engine`, and `tool-registry` MUST NOT be direct dependencies of `voice-runtime`.
- **Project Charter (`PROJECT_CHARTER.md`)**:
  - Zero-Trust Security & Privacy: No raw audio or text logged to disk plain-text; capability authorization enforced via `ServiceRegistry`.

### B. ARCHITECTURAL DECISIONS LOCKED BY THIS ADR:
1. **Locked PCM Representation**: 16,000 Hz sample rate, 1 channel (mono), 16-bit signed little-endian PCM (`i16` sample representation in `Vec<u8>`).
2. **Locked Frame Size**: 10ms frame size = 320 samples = 640 bytes.
3. **Locked Whisper Engine Strategy**: Hugging Face Candle safe Rust Whisper engine (`candle-transformers` / `candle-core`).
4. **Locked Piper Engine Strategy**: Piper C API FFI binding wrapper (`piper-rs`).
5. **Audio Thread Ownership & Lifecycle**: Dedicated background OS worker threads (`std::thread`) managed via `AtomicBool` cancellation tokens and standard library `std::sync::mpsc` channels.
6. **Synchronized Voice State**: Explicit state machine managed inside `std::sync::RwLock<VoiceState>` (`Idle`, `Listening`, `Transcribing`, `Synthesizing`, `Speaking`, `Error`).
7. **Barge-in & Stale-Worker Handling**: User speech audio energy during `Speaking` signals immediate playback cancellation via `AtomicBool` cancellation token and flushes audio buffers.
8. **Fault Recovery Semantics**: Audio hardware/engine failure transitions state to `VoiceState::Error` and returns `VoiceRuntimeError`, falling back gracefully to text UI mode without crashing the host supervisor process.
9. **Latency Budget Enforcement**: `LatencyTargetExceeded` is a **diagnostic logging metric** (warning sink event); it does NOT halt active playback or fail the execution turn.
10. **Configuration Integration**: `VoiceRuntimeConfig` populates from `configuration::VoiceConfig` (`enabled: bool`, `wake_word: String`).

---

## 4. Architectural Decisions (ADR-001)

### A. VoiceRuntime Structure
```rust
pub struct VoiceRuntime {
    config: VoiceRuntimeConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    state: RwLock<VoiceState>,
    stt_engine: Option<Arc<dyn SttEngine>>,
    tts_engine: Option<Arc<dyn TtsEngine>>,
}
```
State synchronization is managed thread-safely via `std::sync::RwLock` (`Send + Sync`).

---

### B. VoiceRuntimeConfig
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRuntimeConfig {
    pub sample_rate: u32,          // Locked: 16000 Hz
    pub channels: u16,              // Locked: 1 (Mono)
    pub frame_size_bytes: usize,   // Locked: 640 bytes (10ms @ 16kHz i16)
    pub latency_target_ms: u64,    // Default: 700 ms
    pub stt_model_id: String,      // Default: "whisper-base-en"
    pub tts_voice_id: String,      // Default: "piper-en-medium"
}
```

---

### C. AudioBuffer Payload
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub pcm_data: Vec<u8>, // 16-bit signed PCM bytes
}
```

---

### D. Voice Processing State Machine
Legal state transitions:
```
Idle → Listening → Transcribing → Synthesizing → Speaking → Idle
```
Error transition:
```
Any State → Error → Idle (upon reset)
```

---

### E. Barge-In & Cancellation Protocol
- When microphone input energy exceeds voice detection threshold while `VoiceState` is `Speaking`:
  1. Set `cancel_flag: Arc<AtomicBool>` to `true`.
  2. Flush output `mpsc` audio queues.
  3. Reset state to `Listening`.

---

### F. Error Model
```rust
pub enum VoiceRuntimeError {
    AudioCaptureFailed { message: String },
    SttTranscriptionFailed { message: String },
    TtsSynthesisFailed { message: String },
    EngineNotLoaded { engine_name: String },
    Runtime(runtime::RuntimeError),
    Service(services::ServicesError),
    Configuration(configuration::ConfigError),
    LockError { message: String },
}
```

---

## 5. Architectural Risks & Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **Worker Deadlocks** | Blocking audio driver calls in runtime loops | Isolate I/O in background `std::thread` workers with `mpsc` channel transport |
| **Audio Distortion** | Buffer sample format mismatch | Enforce locked 16kHz 16-bit mono PCM conversion at input boundary |
| **Supervisor Crashes** | Native C++ audio engine panics | Wrap native engine calls inside `catch_unwind` boundaries |
| **High Latency** | Full-buffer TTS synthesis delay | Stream chunked 10ms PCM slices via `mpsc` channels to output audio device |

---

## IMPLEMENTATION CONTRACT

- **Public Struct**: `pub struct VoiceRuntime`
- **Allowed Direct Dependencies**: `runtime`, `services`, `configuration`, `logging`.
- **Forbidden Dependencies**: `orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`, `kernel`, `capabilities`, `event-bus`.

Status: APPROVED  
Implementation permitted: YES
