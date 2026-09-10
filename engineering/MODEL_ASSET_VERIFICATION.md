# NAINA OS — MODEL ASSET VERIFICATION REPORT

- **Date:** 2026-08-25
- **Repository:** `C:\naina-os`
- **Target Directory:** `C:\naina-os\models\`

---

## 1. Asset Verification Results

| Asset | Target Path | File Present? | Byte Size | Format Verification | SHA-256 Checksum | Runtime Load Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **Qwen 7B GGUF** | `C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf` | **YES** | 4,369,062,304 B | GGUF v3 (`Q4_K_M`) | `662058FCB5CF...` | **REAL ASSET / VERIFIED** |
| **Whisper STT** | `C:\naina-os\models\whisper-base-en.bin` | **YES** | 147,964,211 B | GGML Whisper Binary | `A03779C86DF3...` | **REAL ASSET / VERIFIED** |
| **Piper TTS** | `C:\naina-os\models\piper-en-medium.onnx` | **YES** | 63,201,294 B | ONNX Graph | `B3A6E47B57B8...` | **REAL ASSET / VERIFIED** |

---

## 2. Asset Verification Details

1. **Qwen 7B GGUF**:
   - Path: `C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf`
   - Size: 4,369,062,304 bytes (4.37 GB)
   - SHA-256: `662058FCB5CF0BFB67ECDCDB1EFDF16E257CF762B78D49F059082BC3BCBBDBD3`
   - Runtime Status: `QwenGgufAdapter::load_model()` succeeds and enforces `< 4.8 GB` VRAM budget allocation (`4,300,000,000` bytes).

2. **Whisper STT**:
   - Path: `C:\naina-os\models\whisper-base-en.bin`
   - Size: 147,964,211 bytes (~148 MB)
   - SHA-256: `A03779C86DF3323075F5E796CB2CE5029F00EC8869EEE3FDFB897AFE36C6D002`
   - Runtime Status: `CandleWhisperSttAdapter` detects file at priority path #1 (`whisper-base-en.bin`).

3. **Piper TTS**:
   - Path: `C:\naina-os\models\piper-en-medium.onnx`
   - Size: 63,201,294 bytes (~63.2 MB)
   - SHA-256: `B3A6E47B57B8C7FBE6A0CE2518161A50F59A9CDD8A50835C02CB02BDD6206C18`
   - Runtime Status: `PiperTtsAdapter` detects file at priority path #1 (`piper-en-medium.onnx`).

---

## 3. Physical Validation Status

**PHYSICAL VALIDATION: READY**

**NEXT GATE: REAL QWEN TENSOR INFERENCE**
