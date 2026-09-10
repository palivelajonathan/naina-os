# NAINA OS — MODEL ASSET ACQUISITION REPORT

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Scope:** Inspection of required model asset contracts and filesystem acquisition status for NAINA OS FIRST_ALPHA.

---

## 1. Required Asset Contracts

| Component | Exact Filename | Expected Directory | Model Format | Approx. Size | Required Backend | Required by Code? | Filesystem Status | Acquisition Status |
| :--- | :--- | :--- | :---: | :---: | :--- | :---: | :---: | :---: |
| **Qwen LLM** | `qwen-7b-instruct-q4_k_m.gguf` | `C:\naina-os\models\` | GGUF (`Q4_K_M`) | ~4.3 GB | Candle GGUF (`candle-transformers`) | **YES** | **MISSING (0 B)** | **UNACQUIRED** |
| **Whisper STT** | `whisper-base-en.bin` / `whisper.bin` | `C:\naina-os\models\` | Binary (`.bin`) | ~140 - 460 MB | Candle Whisper (`candle-transformers`) | **YES** | **MISSING (0 B)** | **UNACQUIRED** |
| **Piper TTS** | `piper-en-medium.onnx` / `piper.onnx` | `C:\naina-os\models\` | ONNX (`.onnx`) | ~50 - 150 MB | ONNX Runtime (`ort`) / `piper-rs` | **YES** | **MISSING (0 B)** | **UNACQUIRED** |

---

## 2. Detailed Asset Requirements

### A. Qwen 7B GGUF
- **Target File Path**: `C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf`
- **Quantization**: `Q4_K_M`
- **RAM/VRAM Allocation Budget**: `< 4.8 GB` peak VRAM ceiling (`4,300,000,000` bytes static estimate).
- **Source Code Contract**: `QwenGgufAdapter::new()` targets `./models/qwen-7b-instruct-q4_k_m.gguf`. `load_model()` fails with `ModelRuntimeError::LoadFailed` if file is absent on disk.

### B. Whisper STT
- **Target Fallback Priority**:
  1. `C:\naina-os\models\whisper-base-en.bin`
  2. `C:\naina-os\models\whisper.bin`
  3. `C:\naina-os\models\whisper-base-en\model.bin`
- **Audio Framing Requirement**: 16,000 Hz, 1-channel (mono), 16-bit PCM (`i16`).
- **Source Code Contract**: `CandleWhisperSttAdapter` checks resolution priority order and fails with `VoiceRuntimeError::SttTranscriptionFailed` if binary is absent on disk.

### C. Piper TTS
- **Target Fallback Priority**:
  1. `C:\naina-os\models\piper-en-medium.onnx`
  2. `C:\naina-os\models\piper.onnx`
  3. `C:\naina-os\models\piper-en-medium\model.onnx`
- **Audio Output Framing Requirement**: 16,000 Hz, 1-channel (mono), 16-bit PCM (`i16`), 640 bytes (10ms) per frame slice.
- **Source Code Contract**: `PiperTtsAdapter` checks resolution priority order and fails with `VoiceRuntimeError::TtsSynthesisFailed` if ONNX file is absent on disk.

---

## 3. Current Acquisition Gate Summary

- **Total Required Model Files**: 3
- **Present Files on Disk**: 0
- **Acquired Files**: 0
- **Acquisition Gate Status**: **BLOCKED — MODEL ASSETS MISSING**
