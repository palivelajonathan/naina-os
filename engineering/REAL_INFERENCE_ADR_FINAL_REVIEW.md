# REAL-INFERENCE ADR FINAL REVIEW

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Reviewed Documents:**
  - `packages/model-providers/adr_001_model_providers_amendment_01.md`
  - `packages/voice-runtime/adr_001_voice_runtime_amendment_01.md`
  - `engineering/REAL_INFERENCE_ADR_REVISION_REPORT.md`
- **Authoritative Basis:** `engineering/DEPENDENCY_MAP.md`, `engineering/PACKAGE_RULES.md`, `engineering/PROJECT_CHARTER.md`, `FIRST_ALPHA_SPEC.md`.

---

## Overall Status
APPROVED

## Model-Providers Amendment
PASS

## Voice-Runtime Amendment
PASS

## Required Revisions
NONE

## Critical Issues
NONE

## Dependency/DAG Review
PASS

## Security Review
PASS

## Async Runtime Review
PASS

## Real vs Mock Boundary
PASS

## Model Asset Claims
PASS

## Architecture Compatibility
PASS

## Implementation Readiness
YES

## Final Decision
APPROVED — IMPLEMENTATION PERMITTED: YES

---

### Detailed Review Findings

1. **Model-Providers Amendment (`packages/model-providers/adr_001_model_providers_amendment_01.md`)**:
   - **Backend Decision**: Locked to safe Rust Hugging Face Candle (`candle-transformers::models::quantized_qwen2`). [ADR DECISION]
   - **Feature Flags**: Explicitly specifies `candle-core/cuda` for NVIDIA GPU acceleration (`cargo build --features cuda`) and default CPU fallback (`cargo build`). [ADR DECISION]
   - **Model Path Contract**: Target file `./models/qwen-7b-instruct-q4_k_m.gguf` with `Q4_K_M` quantization. [SOURCE-SUPPORTED]
   - **VRAM Protection**: Strictly enforces `< 4.8 GB` peak VRAM budget limit (`4_800_000_000` bytes). [SOURCE-SUPPORTED]
   - **Async Runtime Prohibition**: 100% compliant (`std::thread` + `std::sync::mpsc`). Zero Tokio contamination. [SOURCE-SUPPORTED]
   - **Mock/Real Boundary**: Retains `MockModelProvider` for offline zero-weight CI unit tests. [SOURCE-SUPPORTED]

2. **Voice-Runtime Amendment (`packages/voice-runtime/adr_001_voice_runtime_amendment_01.md`)**:
   - **Whisper STT Pipeline**: Candle Whisper decoder (`candle-transformers::models::whisper`). [ADR DECISION]
   - **Piper TTS Pipeline**: ONNX Runtime (`ort`) or C FFI `piper-rs` engine wrapper. [ADR DECISION]
   - **Audio Format Contract**: 16,000 Hz 1ch (Mono) 16-bit i16 PCM. [SOURCE-SUPPORTED]
   - **Fallback Resolution Priority Order**:
     - Whisper STT: `./models/{stt_model_id}.bin` -> `./models/whisper.bin` -> `./models/{stt_model_id}/model.bin`. [ADR DECISION]
     - Piper TTS: `./models/{tts_voice_id}.onnx` -> `./models/piper.onnx` -> `./models/{tts_voice_id}/model.onnx`. [ADR DECISION]
   - **Turn Latency Target**: Enforces `< 700 ms` voice-to-voice turn processing budget. [SOURCE-SUPPORTED]
   - **Mock/Real Boundary**: Retains `MockSttEngine` and `MockTtsEngine` for offline zero-weight CI unit tests. [SOURCE-SUPPORTED]

3. **Dependency / DAG Compliance**:
   - `model-providers` depends ONLY on `model-runtime`, `configuration`, `logging`.
   - `voice-runtime` depends ONLY on `runtime`, `services`, `configuration`, `logging`.
   - Zero illegal DAG cross-imports or transitive package rule violations.

---

REVIEW STATUS:
APPROVED

MODEL-PROVIDERS AMENDMENT:
APPROVED

VOICE-RUNTIME AMENDMENT:
APPROVED

CRITICAL ISSUES:
0

REQUIRED REVISIONS:
0

IMPLEMENTATION PERMITTED:
YES

REAL INFERENCE IMPLEMENTATION:
NOW PERMITTED

NEXT GATE:
REAL TENSOR INFERENCE IMPLEMENTATION PLAN
