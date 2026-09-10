# NAINA OS — MODEL ACQUISITION READINESS REPORT

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Scope:** Source code audit of model acquisition parameters, file contracts, and tensor inference readiness.

---

## 1. Qwen
- **Exact Filename**: `qwen-7b-instruct-q4_k_m.gguf` (`packages/model-providers/src/qwen_gguf.rs` line 31)
- **Path**: `C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf`
- **Quantization**: `Q4_K_M`
- **Source Acquisition Information**: **UNSPECIFIED** (No HuggingFace repository link or download URL in codebase).
- **Checksum**: **UNSPECIFIED**
- **Real Inference Status**: **ABSTRACT / MOCK**. `QwenGgufAdapter` checks `model_path.exists()`, but returns formatted text responses rather than executing HuggingFace Candle GGUF tensor math.

---

## 2. Whisper
- **Model ID**: `"whisper-base-en"` (`VoiceRuntimeConfig::default()`)
- **Filename**: **UNSPECIFIED** in Rust source code.
- **Acquisition Information**: **UNSPECIFIED** (No download URL or checksum in codebase).
- **Real Inference Status**: **ABSTRACT / MOCK**. `CandleWhisperSttAdapter` validates 16kHz mono PCM framing, but returns structured mock text responses rather than invoking C++/Candle Whisper tensor weights.

---

## 3. Piper
- **Voice ID**: `"piper-en-medium"` (`VoiceRuntimeConfig::default()`)
- **Filename**: **UNSPECIFIED** in Rust source code.
- **Acquisition Information**: **UNSPECIFIED** (No download URL or model path in codebase).
- **Additional Files**: **UNSPECIFIED** (No `.json` voice config file required by Rust source code).
- **Real Inference Status**: **ABSTRACT / MOCK**. `PiperTtsAdapter` generates synthetic PCM audio frames without loading ONNX weights.

---

## 4. Critical Finding

### Real Inference Implementation Analysis
1. **Model Weight Loading**:
   Placing model files into `C:\naina-os\models\` will satisfy `QwenGgufAdapter::load_model()`, which executes `if !self.model_path.exists() { return Err(...); }`.
2. **Tensor Inference Execution**:
   Placing model files into `models/` will **NOT** automatically transform `QwenGgufAdapter`, `CandleWhisperSttAdapter`, and `PiperTtsAdapter` into real C++/Rust tensor execution engines. The current engine implementations use structured mock/synthetic data generators wrapped in safe microkernel interfaces.
3. **Classification**:
   The current model-provider/model-runtime/voice-runtime implementations are **ABSTRACT / MOCK**.

---

MODEL ACQUISITION READINESS:
COMPLETE

QWEN:
Filename `qwen-7b-instruct-q4_k_m.gguf` specified at `./models/`. Quantization `Q4_K_M`. Acquisition URL/checksum UNSPECIFIED in codebase. Adapter is ABSTRACT/MOCK.

WHISPER:
Model ID `"whisper-base-en"`. Filename and acquisition URL UNSPECIFIED in Rust code. Adapter is ABSTRACT/MOCK.

PIPER:
Voice ID `"piper-en-medium"`. Filename and acquisition URL UNSPECIFIED in Rust code. Adapter is ABSTRACT/MOCK.

REAL INFERENCE READY:
NO

PHYSICAL VALIDATION BLOCKED:
YES

NEXT ACTION:
Acknowledge that model weight binary placement satisfies file existence preconditions (`model_path.exists()`), while physical tensor execution remains software-abstracted in Alpha.
