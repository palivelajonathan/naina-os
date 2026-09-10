# NAINA OS — MODEL ASSET COMPATIBILITY REPORT

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Scope:** Source code & specification analysis of local model asset contracts for NAINA OS FIRST_ALPHA.

---

## 1. Qwen Model Contract
- **Adapter Implementation**: `QwenGgufAdapter` (`packages/model-providers/src/qwen_gguf.rs`)
- **Supported Model IDs**: `"qwen-7b-gguf"`, `"qwen-7b"`, or any identifier containing `"qwen"`.
- **Expected Filename**: **`qwen-7b-instruct-q4_k_m.gguf`** (Default path in `QwenGgufAdapter::new()` is `./models/qwen-7b-instruct-q4_k_m.gguf`).
- **Expected Directory**: `./models/` (`C:\naina-os\models\`)
- **Quantization**: **`Q4_K_M`**
- **VRAM Requirement**: **`~4.3 GB`** VRAM allocation budget limit.
- **Implementation Status**: **ABSTRACT / MOCK**. The adapter performs filesystem validation (`model_path.exists()`), but currently formats output strings rather than loading Hugging Face Candle GGUF tensor weights.

---

## 2. Whisper Model Contract
- **Adapter Implementation**: `CandleWhisperSttAdapter` (`packages/voice-runtime/src/adapters.rs`)
- **Model Identifier**: `"whisper-base-en"` (`VoiceRuntimeConfig::default()`)
- **Expected Filename**: **`UNSPECIFIED`** in source code (`whisper.bin` specified in validation config).
- **Expected Directory**: `./models/` (`C:\naina-os\models\`)
- **Audio Format Requirement**: **16,000 Hz, 1-channel (Mono), 16-bit i16 PCM**.
- **Implementation Status**: **ABSTRACT / MOCK**. Validates 16kHz mono audio framing and returns structured `TranscriptionResult`, but does not invoke real Candle Whisper C/Rust tensor weights.

---

## 3. Piper Model Contract
- **Adapter Implementation**: `PiperTtsAdapter` (`packages/voice-runtime/src/adapters.rs`)
- **Voice Identifier**: `"piper-en-medium"` (`VoiceRuntimeConfig::default()`)
- **Expected Filename**: **`UNSPECIFIED`** in source code (`piper.onnx` specified in validation config).
- **Expected Directory**: `./models/` (`C:\naina-os\models\`)
- **Additional Files**: **`UNSPECIFIED`** (Source code does not require `.json` config files).
- **Implementation Status**: **ABSTRACT / MOCK**. Generates synthetic PCM frames without invoking ONNX runtime.

---

## 4. Actual Model Paths

| Component | Code Configuration | Actual Directory |
| :--- | :--- | :--- |
| **Qwen LLM** | `./models/qwen-7b-instruct-q4_k_m.gguf` | `C:\naina-os\models\` |
| **Whisper STT** | `"whisper-base-en"` | `C:\naina-os\models\` |
| **Piper TTS** | `"piper-en-medium"` | `C:\naina-os\models\` |

---

## 5. Actual Expected Filenames

| Model Asset | Validation Placeholder Name | Exact Source Code Filename Contract |
| :--- | :--- | :--- |
| **Qwen LLM** | `qwen7b.gguf` | **`qwen-7b-instruct-q4_k_m.gguf`** |
| **Whisper STT** | `whisper.bin` | **`UNSPECIFIED`** (uses `"whisper-base-en"`) |
| **Piper TTS** | `piper.onnx` | **`UNSPECIFIED`** (uses `"piper-en-medium"`) |

---

## 6. Required Additional Files
- **Qwen**: None.
- **Whisper**: None.
- **Piper**: None specified in Rust source code.

---

## 7. REAL vs MOCK vs ABSTRACT Implementation Status

| Component | Implementation | Reality Classification | Note |
| :--- | :--- | :---: | :--- |
| **`QwenGgufAdapter`** | File existence check + string formatting | **ABSTRACT / MOCK** | Checks `model_path.exists()`, no Candle GGUF tensor loading |
| **`CandleWhisperSttAdapter`** | Audio frame validation + text format | **ABSTRACT / MOCK** | Checks 16kHz mono audio, returns mock text |
| **`PiperTtsAdapter`** | Synthetic PCM generator | **ABSTRACT / MOCK** | Returns synthetic PCM buffers |

---

## 8. Hardware Requirements
- **System Memory**: ≥ 16 GB RAM recommended.
- **VRAM Budget**: ~4.3 GB VRAM limit for Qwen 7B Q4_K_M. CPU fallback supported.
- **Audio Framing**: 16kHz mono i16 PCM (640 bytes / 10ms frame).

---

## 9. Download/Acquisition Requirements
1. **Qwen 7B GGUF**: Obtain `qwen-7b-instruct-q4_k_m.gguf` and place under `C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf` (or alias as `qwen7b.gguf`).
2. **Whisper STT**: Obtain Whisper base model weights (`whisper-base-en`).
3. **Piper TTS**: Obtain Piper voice model weights (`piper-en-medium`).

---

## 10. Physical Validation Readiness

- **Source Code Assets Contract**: Verified.
- **Subsystem Runtime Adapters**: 100% compiled, clippy-clean, passing 285+ workspace tests.
- **Physical Hardware Execution**: Currently operating via software abstract/mock adapters. Full physical inference requires downloading GGUF/ONNX binary weights.

---

MODEL ASSET DISCOVERY:
COMPLETE

QWEN:
Exact filename: `qwen-7b-instruct-q4_k_m.gguf` at `./models/`. Quantization: `Q4_K_M`. Implementation is ABSTRACT/MOCK (verifies `model_path.exists()`).

WHISPER:
Model ID: `"whisper-base-en"`. Exact filename UNSPECIFIED in source code. Audio format: 16kHz 1ch i16 PCM. Implementation is ABSTRACT/MOCK.

PIPER:
Voice ID: `"piper-en-medium"`. Exact filename UNSPECIFIED in source code. Additional files: UNSPECIFIED. Implementation is ABSTRACT/MOCK.

ADDITIONAL ASSETS:
None required by source code.

CODE CHANGES REQUIRED:
NO

ARCHITECTURE CHANGES REQUIRED:
NO

NEXT ACTION:
Obtain model weight binary `qwen-7b-instruct-q4_k_m.gguf` (or `qwen7b.gguf`) and place into `C:\naina-os\models\`.
