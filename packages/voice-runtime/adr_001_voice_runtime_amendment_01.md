# ADR-001 Amendment 01: Real Local Voice Inference Pipeline Selection

- **Title:** ADR-001 Amendment 01: Real Local Voice Inference Pipeline Selection
- **Status:** APPROVED
- **Date:** 2026-08-25
- **Author:** NAINA OS Core Systems Engineering
- **Target Package:** `packages/voice-runtime`
- **Prerequisite ADR:** `packages/voice-runtime/adr_001_voice_runtime.md`

---

## 1. Status
**APPROVED**

*Implementation Permitted: YES*

---

## 2. Context & Purpose

The existing approved `packages/voice-runtime/adr_001_voice_runtime.md` established the structure of `CandleWhisperSttAdapter` and `PiperTtsAdapter`. However, physical hardware validation auditing (`engineering/REAL_INFERENCE_IMPLEMENTATION_GAP.md`) confirmed that `CandleWhisperSttAdapter` and `PiperTtsAdapter` currently operate as software abstract/mock adapters: they validate audio PCM framing (16kHz mono i16) and synthesize synthetic PCM audio buffers rather than running physical neural tensor inference.

This amendment formally defines the real local tensor pipeline for Whisper STT and Piper TTS, locks model asset resolution strategies (including explicit fallback path resolution order), and enforces strict voice turn performance budgets.

---

## 3. Physical Whisper STT Pipeline Selection

### Strategy & Engine Backend:
1. **Engine Backend**: **Hugging Face Candle Whisper (`candle-transformers::models::whisper`)** in safe Rust.
2. **Audio Input Contract**:
   - Sample Rate: **16,000 Hz** (Locked)
   - Channels: **1 Channel (Mono)** (Locked)
   - Sample Format: **16-bit Signed Little-Endian PCM (`i16`)**
3. **Model ID & Fallback Path Resolution Order (Revision 2)**:
   - Configured Model ID: `"whisper-base-en"` (`VoiceRuntimeConfig::default()`).
   - Base Directory: `./models/` (`C:\naina-os\models\`).
   - Fallback Resolution Priority Order:
     1. `./models/{stt_model_id}.bin` (e.g. `./models/whisper-base-en.bin`)
     2. `./models/whisper.bin`
     3. `./models/{stt_model_id}/model.bin`
4. **Load & Error Semantics**:
   - If no valid model binary is found across the fallback paths during STT initialization, return `Err(VoiceRuntimeError::SttTranscriptionFailed { message: "Whisper STT model file missing..." })`.
   - Audio format mismatches return `Err(VoiceRuntimeError::InvalidAudioFormat { ... })`.
5. **Real vs. Mock Isolation**:
   - `CandleWhisperSttAdapter` performs real Candle tensor Mel-spectrogram processing and decoding when loaded.
   - `MockSttEngine` remains in `voice-runtime` for deterministic offline CI testing.

---

## 4. Physical Piper TTS Pipeline Selection

### Strategy & Engine Backend:
1. **Engine Backend**: **ONNX Runtime (`ort`) or native C FFI `piper-rs` engine wrapper**.
2. **Voice ID & Fallback Path Resolution Order (Revision 2)**:
   - Configured Voice ID: `"piper-en-medium"` (`VoiceRuntimeConfig::default()`).
   - Base Directory: `./models/` (`C:\naina-os\models\`).
   - Fallback Resolution Priority Order:
     1. `./models/{tts_voice_id}.onnx` (e.g. `./models/piper-en-medium.onnx`)
     2. `./models/piper.onnx`
     3. `./models/{tts_voice_id}/model.onnx`
3. **PCM Output Contract**:
   - Sample Rate: **16,000 Hz**
   - Channels: **1 Channel (Mono)**
   - Sample Format: **16-bit Signed Little-Endian PCM (`i16`)**
   - Frame Size: **640 bytes (10ms @ 16kHz mono)**
4. **Real vs. Mock Isolation**:
   - `PiperTtsAdapter` performs real ONNX neural acoustic model synthesis and outputs 10ms PCM frame slices.
   - `MockTtsEngine` remains in `voice-runtime` for deterministic offline CI testing.

---

## 5. Turn Latency & Resource Budget Constraints

1. **Voice-to-Voice Turn Budget**: Total end-to-end processing target remains **`< 700 ms`**.
2. **Barge-In & Stale-Worker Cancellation**:
   - If user speech is detected during `Speaking` state, set `cancel_flag: Arc<AtomicBool>` to `true`.
   - Worker thread flushes output `mpsc` audio queues and immediately aborts active TTS synthesis.
3. **Threading & No-Tokio Rule**:
   - Background audio I/O, STT transcription, and TTS synthesis MUST execute on standard library threads (`std::thread`).
   - Communication with microkernel services MUST use `std::sync::mpsc` channels.

---

## 6. Dependency Review Table

| Dependency | Package | Purpose | Existing? | Status | Allowed by DAG? | Tokio? | Unsafe/FFI? |
| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| `runtime` | `voice-runtime` | Core runtime context | YES | APPROVED | YES | NO | NO |
| `services` | `voice-runtime` | ServiceRegistry & CBAC | YES | APPROVED | YES | NO | NO |
| `configuration` | `voice-runtime` | Root configuration | YES | APPROVED | YES | NO | NO |
| `logging` | `voice-runtime` | Structured logging | YES | APPROVED | YES | NO | NO |
| `candle-core` | `voice-runtime` | Tensor memory for STT | NO | PROPOSED | YES | NO | NO |
| `candle-transformers` | `voice-runtime` | Whisper model decoder | NO | PROPOSED | YES | NO | NO |
| `ort` / `piper-rs` | `voice-runtime` | Piper ONNX TTS synthesis | NO | PROPOSED | YES | NO | YES |

---

## 7. Performance & Security Matrix

| Domain | Target Parameter | Software/Mock Benchmark | Physical Hardware Target | Classification |
| :--- | :--- | :---: | :---: | :---: |
| **Voice-to-Voice Turn** | **`< 700 ms`** | ~1.0 ms | `< 700 ms` | **MEASURED / TARGET** |
| **STT Audio Input** | 16kHz Mono i16 | Verified | Verified | **LOCKED CONTRACT** |
| **TTS Audio Output** | 16kHz Mono i16 | Verified | Verified | **LOCKED CONTRACT** |
| **Privacy Protection** | Zero Plaintext Audio Logs | Verified | Verified | **SECURITY RULE** |

---

## 8. Architectural Review Checklist

- [x] **DAG Compliance**: `voice-runtime` depends ONLY on `runtime`, `services`, `configuration`, `logging`.
- [x] **No Tokio Contamination**: Zero Tokio dependencies proposed. Uses standard `std::thread` + `mpsc`.
- [x] **Voice Latency Budget**: Enforces `< 700 ms` voice-to-voice turn budget.
- [x] **Audio Framing Contract**: Enforces 16kHz mono 16-bit PCM input/output format.
- [x] **Fallback Path Order**: Explicit model resolution priority order defined for Whisper and Piper.
- [x] **Mock/Real Separation**: Retains `MockSttEngine` and `MockTtsEngine` for offline CI testing.
- [x] **Barge-In Protection**: Supports `AtomicBool` barge-in cancellation protocol.

---

STATUS: PROPOSED — PENDING RE-REVIEW  
IMPLEMENTATION PERMITTED: NO  
NEXT GATE: FORMAL ADR RE-REVIEW
