# NAINA OS — REAL TENSOR INFERENCE IMPLEMENTATION PLAN

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Scope:** Implementation blueprint for transitioning `model-providers` and `voice-runtime` from software abstract/mock adapters into physical local tensor execution engines.
- **Authoritative Basis:** `engineering/REAL_INFERENCE_ADR_FINAL_REVIEW.md`, `packages/model-providers/adr_001_model_providers_amendment_01.md`, `packages/voice-runtime/adr_001_voice_runtime_amendment_01.md`.

---

## 1. Executive Summary

This document specifies the phased technical execution plan for implementing real local tensor inference across NAINA OS Alpha. Following the formal approval of ADR Amendment 01 for `model-providers` and `voice-runtime`, this plan defines dependency diffs, tensor execution pipelines, memory/VRAM allocation controls, API compatibility preservation, error mapping, and verification commands.

---

## 2. Current Baseline

| Component / Subsystem | Current Public Struct / API | Current Reality | Status |
| :--- | :--- | :---: | :---: |
| **Qwen LLM Adapter** | `pub struct QwenGgufAdapter` (`packages/model-providers`) | **ABSTRACT / MOCK** | Performs file existence check; returns formatted output string |
| **Whisper STT Adapter** | `pub struct CandleWhisperSttAdapter` (`packages/voice-runtime`) | **ABSTRACT / MOCK** | Validates 16kHz mono audio framing; returns mock text |
| **Piper TTS Adapter** | `pub struct PiperTtsAdapter` (`packages/voice-runtime`) | **ABSTRACT / MOCK** | Validates text payload; generates synthetic 10ms PCM audio frames |
| **Offline LLM Mock** | `pub struct MockModelProvider` (`packages/model-providers`) | **MOCK** | Zero-weight offline CI mock provider |
| **Offline STT Mock** | `pub struct MockSttEngine` (`packages/voice-runtime`) | **MOCK** | Zero-weight offline CI mock STT engine |
| **Offline TTS Mock** | `pub struct MockTtsEngine` (`packages/voice-runtime`) | **MOCK** | Zero-weight offline CI mock TTS engine |

---

## 3. Model-Providers Implementation Plan

- **Target Model**: Qwen 7B Instruct (`Q4_K_M` GGUF quantization).
- **Target File Path**: `./models/qwen-7b-instruct-q4_k_m.gguf` (`C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf`).
- **Physical Tensor Backend**: **Hugging Face Candle (`candle-core` + `candle-transformers`)**.
- **Package Ownership**: `packages/model-providers` owns tensor loading and generation.
- **Cargo Feature Compilation**:
  - `candle-core/cuda` enabled via `--features cuda` for GPU acceleration.
  - Standard CPU fallback compilation when `cuda` feature is omitted.
- **VRAM Allocation Guard**: Static allocation ~4.3 GB (`4_300_000_000` bytes). Max peak VRAM cap strictly enforced at **`< 4.8 GB`** (`4_800_000_000` bytes).
- **Concurrency & Stream Model**: `std::thread::spawn` worker + `std::sync::mpsc::channel`. `AtomicBool` token checking between generated tokens for responsive cancellation. Zero Tokio contamination.

---

## 4. Whisper Implementation Plan

- **Target Model ID**: `"whisper-base-en"`.
- **Physical Tensor Backend**: **Hugging Face Candle Whisper (`candle-transformers::models::whisper`)**.
- **Package Ownership**: `packages/voice-runtime` owns STT decoding.
- **Audio Format Contract**: 16,000 Hz, 1-channel (Mono), 16-bit signed PCM (`i16`).
- **Fallback Resolution Priority Order**:
  1. `./models/whisper-base-en.bin`
  2. `./models/whisper.bin`
  3. `./models/whisper-base-en/model.bin`
- **Tensor Pipeline**: PCM audio samples -> Mel-spectrogram filter-bank feature extraction -> Candle Whisper encoder-decoder forward pass -> greedy token decoding -> output transcript string.

---

## 5. Piper Implementation Plan

- **Target Voice ID**: `"piper-en-medium"`.
- **Physical Execution Backend**: **ONNX Runtime (`ort`) or C FFI `piper-rs` engine wrapper**.
- **Package Ownership**: `packages/voice-runtime` owns TTS synthesis.
- **Audio Output Contract**: 16,000 Hz, 1-channel (Mono), 16-bit signed PCM (`i16`), framed in 640-byte (10ms) slices.
- **Fallback Resolution Priority Order**:
  1. `./models/piper-en-medium.onnx`
  2. `./models/piper.onnx`
  3. `./models/piper-en-medium/model.onnx`
- **Synthesis Pipeline**: Input text string -> phonemization -> ONNX acoustic graph inference -> 16kHz PCM audio frame generation.

---

## 6. Dependency Diff

| Package | Crate Name | Version | Purpose | Approved by ADR? | Tokio? | DAG Violation? |
| :--- | :--- | :---: | :--- | :---: | :---: | :---: |
| `packages/model-providers` | `candle-core` | `^0.8` | Safe Rust tensor memory & CUDA dispatch | YES | NO | NO |
| `packages/model-providers` | `candle-transformers` | `^0.8` | Quantized Qwen2 GGUF model decoder | YES | NO | NO |
| `packages/voice-runtime` | `candle-core` | `^0.8` | Tensor memory for Mel features | YES | NO | NO |
| `packages/voice-runtime` | `candle-transformers` | `^0.8` | Whisper model decoder | YES | NO | NO |
| `packages/voice-runtime` | `ort` / `piper-rs` | `^2.0` | Piper ONNX TTS synthesis | YES | NO | NO |

---

## 7. Model Asset Manifest

| Asset | Required? | Target Filename | Format | Expected Location | Checksum / Source |
| :--- | :---: | :--- | :---: | :--- | :--- |
| **Qwen 7B GGUF** | YES | `qwen-7b-instruct-q4_k_m.gguf` | GGUF | `./models/qwen-7b-instruct-q4_k_m.gguf` | UNSPECIFIED |
| **Whisper STT** | YES | `whisper-base-en.bin` | Binary | `./models/whisper-base-en.bin` | UNSPECIFIED |
| **Piper TTS** | YES | `piper-en-medium.onnx` | ONNX | `./models/piper-en-medium.onnx` | UNSPECIFIED |

---

## 8. Hardware Requirements

- **GPU Target**: NVIDIA RTX 4050-class / GTX 1660+ GPU with CUDA driver support.
- **VRAM Budget**: `< 4.8 GB` peak VRAM allocation ceiling.
- **CPU Fallback**: Full CPU multi-threading execution fallback when CUDA GPU is unavailable.
- **RAM Target**: ≥ 16 GB system memory (1.0 GB idle budget).

---

## 9. API Compatibility

| Existing Public Struct | Real Implementation Change | Breaking API Change? |
| :--- | :--- | :---: |
| `QwenGgufAdapter` | Internal `generate()` executes Candle GGUF tensor math | **NO** |
| `CandleWhisperSttAdapter` | Internal `transcribe()` executes Candle Whisper decoder | **NO** |
| `PiperTtsAdapter` | Internal `synthesize()` executes ONNX acoustic synthesis | **NO** |
| `MockModelProvider` | Unchanged offline mock provider | **NO** |
| `MockSttEngine` | Unchanged offline mock STT engine | **NO** |
| `MockTtsEngine` | Unchanged offline mock TTS engine | **NO** |

---

## 10. Error Model

1. **Missing Model File**: Returns `ModelRuntimeError::LoadFailed` / `VoiceRuntimeError::SttTranscriptionFailed`.
2. **VRAM Exceeded**: Returns `ModelRuntimeError::VramExceeded` (`vram > 4.8 GB`). Unloads model immediately.
3. **Audio Format Mismatch**: Returns `VoiceRuntimeError::InvalidAudioFormat` if sample rate != 16000 Hz or channels != 1.
4. **Cancellation**: Worker checks `AtomicBool` token; aborts cleanly without worker thread deadlock.
5. **Panic Isolation**: Unhandled tensor panics isolated via `std::panic::catch_unwind` at thread boundaries.

---

## 11. Performance Validation

- **Cold Boot Target**: `< 2.0 s`
- **Voice-to-Voice Turn Target**: `< 700 ms`
- **Peak VRAM Cap**: `< 4.8 GB`
- **Memory Retrieval Target**: `< 300 ms`
- **Desktop Execution Target**: `< 500 ms`

---

## 12. Test Strategy

1. **Unit Tests**: Mock unit tests (`MockModelProvider`, `MockSttEngine`, `MockTtsEngine`) execute on every CI run without model weights.
2. **Integration Tests**: Feature-gated physical tensor tests (`#[cfg(feature = "physical-tests")]`) run when local model weights exist in `./models/`.

---

## 13. Phased Implementation Plan

- **Phase 0: Architecture & Safety Preflight** (Verify ADR status, clean working tree).
- **Phase 1: Dependency Integration** (Add approved `candle` / `ort` crates to `Cargo.toml`).
- **Phase 2: Real Qwen GGUF Backend** (Implement `Candle` tensor generation in `QwenGgufAdapter`).
- **Phase 3: Real Whisper STT Backend** (Implement Mel feature extraction & decoding in `CandleWhisperSttAdapter`).
- **Phase 4: Real Piper TTS Backend** (Implement ONNX speech synthesis in `PiperTtsAdapter`).
- **Phase 5: Integration & Verification** (Run full workspace compiler checks and regression suites).
- **Phase 6: Hardware Benchmark** (Run live GPU/VRAM hardware validation with loaded weights).

---

## 14. Cargo Verification

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

## 15. Real vs. Mock Matrix

| Subsystem | Baseline State | Planned Target State | Hardware Validation Method |
| :--- | :---: | :---: | :--- |
| **Qwen LLM** | ABSTRACT / MOCK | **REAL (Candle GGUF)** | Live GPU prompt execution in `./models/` |
| **Whisper STT** | ABSTRACT / MOCK | **REAL (Candle Whisper)** | PCM buffer transcription |
| **Piper TTS** | ABSTRACT / MOCK | **REAL (ONNX Piper)** | Audio PCM synthesis |
| **Win32 App Launch** | REAL | **REAL** | `CreateProcessW` FFI Notepad launch |
| **Browser CDP** | REAL | **REAL** | Chromium CDP DOM inspection |
| **Obsidian Memory** | REAL | **REAL** | BM25 + Vector hybrid search in `vault/` |

---

## 16. Architectural Safety Gate

- [x] Zero Tokio or prohibited async runtime dependencies.
- [x] Zero illegal cross-imports or DAG position violations.
- [x] Strictly enforces `< 4.8 GB` peak VRAM budget limit.
- [x] Zero plaintext prompt or audio payload logging in log sinks.
- [x] Offline mock providers preserved for CI testing.

---

## 17. Open Questions / Unspecified Items

- **Model Download URLs & Checksums**: Model binary download URLs and SHA256 checksums are **UNSPECIFIED** in codebase; user must manually place weight binaries in `./models/`.

---

## 18. Final Implementation Readiness

The Real Tensor Inference Implementation Plan is **COMPLETE**. Implementation will commence upon user approval of this plan.

---

REAL TENSOR INFERENCE IMPLEMENTATION PLAN:
COMPLETE

IMPLEMENTATION:
NOT STARTED

MODELS:
NOT DOWNLOADED

CARGO CHANGES:
NOT MADE

NEXT GATE:
IMPLEMENTATION PLAN REVIEW
