# NAINA OS — REAL INFERENCE ADR REVISION REPORT

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Revised Documents:**
  - `packages/model-providers/adr_001_model_providers_amendment_01.md`
  - `packages/voice-runtime/adr_001_voice_runtime_amendment_01.md`
- **Authoritative Basis:** `engineering/REAL_INFERENCE_ADR_REVIEW.md`

---

## 1. Model-Providers Revision
- **Original Review Finding**: Revision 1 — Explicitly document Cargo feature flags (`cuda` vs `cpu`) for compiling Candle with NVIDIA CUDA acceleration or CPU fallback.
- **ADR Section Changed**: Section 4 (`Formal Backend Recommendation & Locked Decision`) & Section 10 (`Architectural Review Checklist`).
- **Revision Applied**: Added `Locked Cargo Feature Compilation Policy` explicitly mandating `candle-core/cuda` for GPU acceleration (`cargo build --features cuda`) and fallback to standard CPU execution when the `cuda` feature is omitted (`cargo build`).
- **Why it Resolves Finding**: Eliminates ambiguity regarding CUDA vs CPU compilation strategy, ensuring zero-dependency buildability on CI environments lacking NVIDIA CUDA SDK toolchains.

---

## 2. Voice-Runtime Revision
- **Original Review Finding**: Revision 2 — Explicitly define the fallback path resolution order when resolving model files for `"whisper-base-en"` and `"piper-en-medium"`.
- **ADR Section Changed**: Section 3 (`Physical Whisper STT Pipeline Selection`) & Section 4 (`Physical Piper TTS Pipeline Selection`).
- **Revision Applied**: Defined explicit fallback resolution priority orders:
  - Whisper STT: `./models/{stt_model_id}.bin` -> `./models/whisper.bin` -> `./models/{stt_model_id}/model.bin`.
  - Piper TTS: `./models/{tts_voice_id}.onnx` -> `./models/piper.onnx` -> `./models/{tts_voice_id}/model.onnx`.
- **Why it Resolves Finding**: Provides a deterministic, robust path resolution strategy for physical model assets without inventing unverified file names or URLs.

---

## 3. Unchanged Decisions Preserved
- `QwenGgufAdapter` targets `./models/qwen-7b-instruct-q4_k_m.gguf` with `Q4_K_M` quantization.
- Active pipeline peak VRAM budget limit remains strictly **`< 4.8 GB`**.
- End-to-end voice-to-voice turn latency budget target remains strictly **`< 700 ms`**.
- Offline mock providers (`MockModelProvider`, `MockSttEngine`, `MockTtsEngine`) preserved for zero-weight CI unit testing.
- Async runtime prohibition: Zero Tokio dependencies proposed. Uses standard library `std::thread` + `mpsc` channels.

---

## 4. Dependency Review
- Zero production code files in `src/` modified.
- Zero `Cargo.toml` files modified.
- Zero model weights downloaded.

---

## 5. Implementation Gate
- Both ADR amendment documents have been updated to `STATUS: PROPOSED — PENDING RE-REVIEW`.
- `Implementation Permitted` remains strictly **NO**.

---

ADR AMENDMENTS REVISED:
YES

MODEL-PROVIDERS:
REVISED

VOICE-RUNTIME:
REVISED

PRODUCTION CODE CHANGED:
NO

CARGO CHANGED:
NO

MODELS DOWNLOADED:
NO

IMPLEMENTATION PERMITTED:
NO

NEXT GATE:
FORMAL ADR RE-REVIEW
