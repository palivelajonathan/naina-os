# NAINA OS — REAL INFERENCE ADR AMENDMENT REVIEW

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Reviewed Documents:**
  - `packages/model-providers/adr_001_model_providers_amendment_01.md`
  - `packages/voice-runtime/adr_001_voice_runtime_amendment_01.md`
- **Authoritative Basis:** `engineering/DEPENDENCY_MAP.md`, `engineering/PACKAGE_RULES.md`, `engineering/PROJECT_CHARTER.md`, `FIRST_ALPHA_SPEC.md`, `REAL_INFERENCE_IMPLEMENTATION_GAP.md`, `MODEL_ASSET_COMPATIBILITY_REPORT.md`, `MODEL_ACQUISITION_READINESS.md`.

---

## 1. Model-Providers Amendment Review

### Technical Evaluation & Claims Classification:
1. **Qwen GGUF Backend Decision**: Evaluates Candidate A (`candle-transformers`) vs Candidate B (`llama-cpp-rs`). Recommends Candle for pure Rust safety, Windows MSVC build simplicity, and CPU fallback.
2. **GGUF / Q4_K_M Quantization**: Native support in Candle `candle_transformers::models::quantized_qwen2`. [SOURCE-SUPPORTED]
3. **Windows MSVC Compatibility**: Candle compiles natively with `cargo build` without CMake or native C++ compilers. [SOURCE-SUPPORTED]
4. **NVIDIA GPU Acceleration**: Supported via `candle-core` `cuda` feature flag. [ENGINEERING ANALYSIS]
5. **RTX 4050 VRAM Budget (`< 4.8 GB`)**: Static weights ~4.3 GB (`4_300_000_000` bytes) fit strictly under the `< 4.8 GB` peak VRAM budget ceiling. [ENGINEERING ANALYSIS]
6. **CPU Execution Fallback**: Supported when CUDA GPU is unavailable. [SOURCE-SUPPORTED]
7. **Tokio Prohibition**: Uses `std::thread` worker channels and `std::sync::mpsc`. Zero Tokio contamination. [SOURCE-SUPPORTED]
8. **Unsafe FFI Boundaries**: Pure Rust execution; zero unsafe application code. [SOURCE-SUPPORTED]
9. **Cancellation**: Step-by-step token generation checks `AtomicBool` cancellation flags. [SOURCE-SUPPORTED]
10. **Model Loading Path Contract**: `./models/qwen-7b-instruct-q4_k_m.gguf` (`C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf`). [SOURCE-SUPPORTED]
11. **Mock-vs-Real Separation**: Retains `MockModelProvider` for offline unit testing. [SOURCE-SUPPORTED]
12. **Security & Privacy**: Zero plaintext secret prompt logging enforced. [SOURCE-SUPPORTED]

---

## 2. Voice-Runtime Amendment Review

### Whisper STT Pipeline:
1. **Physical Decoder Strategy**: Hugging Face Candle Whisper decoder (`candle-transformers::models::whisper`).
2. **Audio Input Contract**: 16,000 Hz, 1-channel (Mono), 16-bit i16 PCM. [SOURCE-SUPPORTED]
3. **Model Identity**: `"whisper-base-en"`. Filename and acquisition URL remain explicitly **UNSPECIFIED** in Rust source code.
4. **Error Handling & Cancellation**: Maps to `VoiceRuntimeError::SttTranscriptionFailed`. Supports barge-in cancellation via `AtomicBool`.
5. **Mock-vs-Real Separation**: Retains `MockSttEngine` for offline unit testing.

### Piper TTS Pipeline:
1. **Physical Execution Strategy**: ONNX Runtime (`ort`) or native C FFI `piper-rs` engine wrapper.
2. **Voice Identity**: `"piper-en-medium"`. Filename and additional `.json` config files remain explicitly **UNSPECIFIED** in Rust source code.
3. **Audio Output Contract**: 16,000 Hz, 1-channel (Mono), 16-bit i16 PCM (640 bytes / 10ms frame). [SOURCE-SUPPORTED]
4. **Error Handling & Cancellation**: Maps to `VoiceRuntimeError::TtsSynthesisFailed`. Supports output channel flushing.
5. **Mock-vs-Real Separation**: Retains `MockTtsEngine` for offline unit testing.

---

## 3. Dependency / DAG Review

| Dependency | Target Package | Purpose | Existing? | Proposed? | DAG Allowed? | Tokio Contamination? | Unsafe / FFI? | Review Status |
| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| `candle-core` | `model-providers` | Safe Rust tensor memory | NO | YES | **YES** | NO | NO | **PROPOSED** |
| `candle-transformers` | `model-providers` | Qwen GGUF decoder | NO | YES | **YES** | NO | NO | **PROPOSED** |
| `candle-transformers` | `voice-runtime` | Whisper STT decoder | NO | YES | **YES** | NO | NO | **PROPOSED** |
| `ort` / `piper-rs` | `voice-runtime` | Piper ONNX TTS synthesis | NO | YES | **YES** | NO | YES | **PROPOSED** |

- **DAG Compliance**: All proposed dependencies are contained entirely inside `model-providers` and `voice-runtime`. Neither package introduces illegal cross-imports or violates DAG position.
- **Async Runtime Prohibition**: Zero Tokio or async runtime dependencies proposed.

---

## 4. Real vs. Mock vs. Abstract Review
- **REAL**: Physical tensor execution via Candle (Qwen/Whisper) and ONNX Runtime (Piper) when model files exist.
- **MOCK**: Deterministic offline unit testing via `MockModelProvider`, `MockSttEngine`, and `MockTtsEngine`.
- **ABSTRACT**: Current implementation state where file existence checks and PCM audio framing are active without physical tensor evaluation.
- **DEFERRED**: Cloud API proxies, multi-GPU parallelism, and hardware ROS robotics.

---

## 5. Performance Review
- Preserves all First Alpha performance targets:
  - System Cold Boot: **`< 2.0 s`**
  - Active Pipeline Peak VRAM: **`< 4.8 GB`**
  - Voice-to-Voice Turn Latency: **`< 700 ms`**
- Explicitly separates architectural targets, software/mock benchmarks, and future physical hardware benchmarks.

---

## 6. Security Review
- Zero plaintext secret prompt or raw audio logging to disk.
- Zero credential exposure.
- Fault isolation: Native model loading errors or tensor decoding failures return controlled `Result` error types without aborting the host process.

---

## 7. Architecture Consistency Review
- Both amendments preserve microkernel rules, zero-trust CBAC security, acyclic DAG boundaries, and standard library worker thread models (`std::thread` + `mpsc`).

---

## 8. Critical Issues
**NONE**

---

## 9. Required Revisions

1. **Model-Providers Amendment (`adr_001_model_providers_amendment_01.md`)**:
   - *Revision 1*: Explicitly document Cargo feature flags (`cuda` vs `cpu`) for compiling Candle with NVIDIA CUDA acceleration or CPU fallback.
2. **Voice-Runtime Amendment (`adr_001_voice_runtime_amendment_01.md`)**:
   - *Revision 2*: Explicitly define the fallback path resolution order when resolving model files for `"whisper-base-en"` (e.g., `./models/whisper-base-en.bin`, `./models/whisper.bin`) and `"piper-en-medium"` (e.g., `./models/piper-en-medium.onnx`, `./models/piper.onnx`).

---

## 10. Final Decisions

- **MODEL-PROVIDERS AMENDMENT**: **APPROVED WITH REQUIRED REVISIONS**
- **VOICE-RUNTIME AMENDMENT**: **APPROVED WITH REQUIRED REVISIONS**

---

## 11. Implementation Gate

- **IMPLEMENTATION PERMITTED**: **NO**
- **NEXT GATE**: REVISE ADR AMENDMENTS (Apply Required Revisions 1 & 2 to amendment documents prior to final approval).

---

MODEL-PROVIDERS AMENDMENT:
APPROVED WITH REQUIRED REVISIONS

VOICE-RUNTIME AMENDMENT:
APPROVED WITH REQUIRED REVISIONS

CRITICAL ISSUES:
0

REQUIRED REVISIONS:
2

IMPLEMENTATION PERMITTED:
NO

NEXT GATE:
REVISE ADR AMENDMENTS
