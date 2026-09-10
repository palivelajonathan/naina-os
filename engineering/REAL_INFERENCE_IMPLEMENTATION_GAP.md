# NAINA OS — REAL INFERENCE IMPLEMENTATION GAP

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Scope:** Technical gap analysis for transitioning Qwen LLM, Whisper STT, and Piper TTS from software abstract/mock adapters into physical local tensor inference.

---

## 1. Executive Summary

The NAINA OS First Alpha release (v0.8.0) includes 20 fully compiled, verified, and integrated workspace crates. While microkernel supervision, service registration, process lifecycle management, Win32 app control, browser CDP transport, BM25/vector memory search, and turn orchestration are **100% REAL and operational**, the underlying model provider adapters (`QwenGgufAdapter`, `CandleWhisperSttAdapter`, `PiperTtsAdapter`) currently perform file existence checks and PCM audio framing while returning structured software abstract/mock responses.

To achieve physical local model inference on physical hardware without breaking microkernel constraints or introducing unapproved async dependencies (Tokio), formal architectural backend selections and ADR amendments are required.

---

## 2. Current Reality

| Component / Subsystem | Current Implementation | Reality Classification | Status |
| :--- | :--- | :---: | :---: |
| **NKRS Microkernel & Supervisor** | `packages/kernel`, `packages/runtime` | **REAL** | 100% Operational |
| **Service Registry & CBAC Security** | `packages/services`, `packages/capabilities` | **REAL** | 100% Operational |
| **Obsidian Memory Engine (BM25+Vector)** | `packages/memory` | **REAL** | 100% Operational |
| **Win32 Desktop Control FFI** | `packages/desktop-runtime` | **REAL** | 100% Operational |
| **Browser CDP Control Adapter** | `packages/browser-runtime` | **REAL** | 100% Operational |
| **CARF Cognitive Orchestrator** | `packages/orchestrator` | **REAL** | 100% Operational |
| **Desktop Host Composition Root** | `apps/desktop` (`naina-desktop.exe`) | **REAL** | 100% Operational |
| **Qwen 7B GGUF Model Adapter** | `packages/model-providers/src/qwen_gguf.rs` | **ABSTRACT / MOCK** | File Check Only |
| **Whisper STT Model Adapter** | `packages/voice-runtime/src/adapters.rs` | **ABSTRACT / MOCK** | PCM Frame Check Only |
| **Piper TTS Model Adapter** | `packages/voice-runtime/src/adapters.rs` | **ABSTRACT / MOCK** | Synthetic PCM Generator |

---

## 3. Qwen GGUF Gap

- **Current Implementation**: `QwenGgufAdapter` in `packages/model-providers/src/qwen_gguf.rs`. Checks `model_path.exists()` and returns formatted output text strings.
- **Required Real Inference Capability**: Loading GGUF tensor weights into system RAM/VRAM and executing token generation.
- **Missing Dependencies**: Hugging Face Candle GGUF tensor decoder crates (`candle-core`, `candle-transformers`) or native `llama.cpp` FFI bindings (`llama-cpp-rs`).
- **GPU/VRAM Requirements**: Conservative VRAM allocation budget of **`< 4.3 GB`** for Qwen 7B `Q4_K_M` quantization. CPU fallback required when VRAM is unavailable.
- **DAG Impact**: None. `model-providers` remains at the same position in the DAG (`model-providers → model-runtime → runtime`).
- **ADR Impact**: Requires formal ADR amendment to `model-providers/adr_001_model_providers.md`.

---

## 4. Whisper Gap

- **Current Implementation**: `CandleWhisperSttAdapter` in `packages/voice-runtime/src/adapters.rs`. Checks 16kHz mono PCM framing and returns structured mock text.
- **Required Real Inference Capability**: Mel-spectrogram extraction and Whisper transformer decoder execution.
- **Missing Dependencies**: `candle-transformers` (Whisper model definition) + audio Mel-spectrogram feature extractor.
- **GPU/VRAM Requirements**: ~500 MB RAM/VRAM for Whisper Base (`whisper-base-en`).
- **DAG Impact**: None. `voice-runtime` remains dependent on `runtime` and `services`.
- **ADR Impact**: Requires formal ADR amendment to `voice-runtime/adr_001_voice_runtime.md`.

---

## 5. Piper Gap

- **Current Implementation**: `PiperTtsAdapter` in `packages/voice-runtime/src/adapters.rs`. Synthesizes synthetic 16kHz PCM audio buffers.
- **Required Real Inference Capability**: ONNX neural acoustic model execution and VITS phoneme synthesis.
- **Missing Dependencies**: `ort` (ONNX Runtime Rust bindings) or C FFI bindings to `piper-rs` + `espeak-ng` phonemizer binary.
- **GPU/VRAM Requirements**: ~150 MB RAM (CPU execution preferred).
- **DAG Impact**: None.
- **ADR Impact**: Requires formal ADR amendment to `voice-runtime/adr_001_voice_runtime.md`.

---

## 6. Cross-Cutting Runtime Constraints

1. **No Tokio / Unapproved Async Runtimes**: All inference workers MUST run on standard library threads (`std::thread`) and communicate via `std::sync::mpsc` channels.
2. **DAG Integrity**: High-level engine packages MUST NOT import `kernel` or `capabilities` directly.
3. **Threading & Lock Hierarchy**: Heavy model inference must execute asynchronously on dedicated worker channels without holding `RwLock` write locks across inference steps.
4. **Memory & VRAM Constraints**: Total active pipeline VRAM peak MUST remain strictly **`< 4.8 GB`**.
5. **Fault Isolation**: Inference panics or out-of-memory errors MUST be caught cleanly and surfaced as controlled `ModelRuntimeError` or `VoiceRuntimeError` without crashing the `Kernel` supervisor process.
6. **Cancellation**: Long-running generation loops MUST check `AtomicBool` cancellation flags between output tokens.
7. **Security**: Plaintext secret logging remains strictly prohibited across all logger sinks.

---

## 7. Dependency Analysis

| Proposed Dependency | Purpose | Existing in Workspace? | Allowed by DAG? | Architectural Decision Required? |
| :--- | :--- | :---: | :---: | :---: |
| **`candle-core` / `candle-transformers`** | Safe Rust GGUF & Whisper inference | NO | YES | **YES** |
| **`llama-cpp-rs`** | C++ FFI `llama.cpp` GGUF inference | NO | YES | **YES** |
| **`ort` (ONNX Runtime)** | Piper TTS ONNX model execution | NO | YES | **YES** |
| **`piper-rs`** | Native C++ Piper TTS bindings | NO | YES | **YES** |

---

## 8. Implementation Options

### Option A: Pure Rust Stack via Hugging Face Candle (`candle-core` + `candle-transformers`)
- **Advantages**: 100% safe Rust, zero C++ FFI build complexity, native compatibility with standard library threading without Tokio.
- **Disadvantages**: Slightly lower token generation speed compared to highly optimized C++ `llama.cpp` AVX-512/CUDA kernels.
- **DAG Impact**: None (internal crate dependency inside `model-providers` and `voice-runtime`).
- **Complexity**: Low / Medium.
- **Risk**: Low.

### Option B: C++ FFI Stack via `llama.cpp` & `piper-rs`
- **Advantages**: Maximum physical GGUF inference speed and VRAM optimization.
- **Disadvantages**: Requires C++ toolchains (MSVC / CMake), external C FFI memory management, complex cross-compilation.
- **DAG Impact**: None.
- **Complexity**: High.
- **Risk**: Medium.

---

## 9. Required Architectural Decisions

1. **Physical LLM Backend Selection**: Formally approve `candle-transformers` vs `llama-cpp-rs` for GGUF model execution in `model-providers`.
2. **Physical Voice Backend Selection**: Formally approve `candle-transformers` (Whisper) and `ort`/`piper-rs` (Piper) in `voice-runtime`.
3. **C++ Toolchain Policy**: Determine whether external C++ build toolchains (CMake / MSVC) are permitted in NAINA OS release builds.

---

## 10. Recommended Next Gate

**B. ADR AMENDMENT REQUIRED**

- Draft formal ADR amendments for `packages/model-providers/adr_001_model_providers.md` and `packages/voice-runtime/adr_001_voice_runtime.md` locking physical tensor execution backend dependencies.
- **Implementation Permitted**: **NO** (Pending formal ADR amendment review and approval).

---

## 11. Physical Validation Impact

- The microkernel architecture, composition root, SDK, UI, Win32 desktop control, CDP browser control, Obsidian memory engine, and automated test suite are **100% READY**.
- Live physical tensor benchmarking on physical GPU hardware remains **BLOCKED** until physical tensor backends are formally approved via ADR amendments and model binary weights are placed on disk.

---

REAL INFERENCE GAP ANALYSIS:
COMPLETE

QWEN:
`QwenGgufAdapter` is ABSTRACT/MOCK (verifies `model_path.exists()`). Real inference requires locking physical tensor backend (`candle-transformers` vs `llama-cpp-rs`).

WHISPER:
`CandleWhisperSttAdapter` is ABSTRACT/MOCK (validates 16kHz mono audio). Real inference requires locking Candle Whisper decoder tensor pipeline.

PIPER:
`PiperTtsAdapter` is ABSTRACT/MOCK (generates synthetic PCM). Real inference requires locking ONNX/Piper TTS runtime pipeline.

REAL LOCAL INFERENCE:
NOT READY

ARCHITECTURE CHANGES:
NO

ADR REQUIRED:
YES (ADR amendments required for `model-providers` and `voice-runtime`)

IMPLEMENTATION PERMITTED:
NO

NEXT GATE:
Draft formal ADR amendments for `model-providers` and `voice-runtime` locking physical tensor execution backends.
