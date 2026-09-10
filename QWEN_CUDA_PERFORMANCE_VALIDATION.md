# NAINA OS — QWEN CUDA PERFORMANCE VALIDATION REPORT

**Document ID**: `QWEN_CUDA_PERFORMANCE_VALIDATION.md`  
**Date**: August 28, 2026  
**Status**: `STATUS: FUNCTIONALLY VERIFIED — PERFORMANCE TARGET NOT MET`  

---

## Executive Summary

The NAINA OS Qwen CUDA performance benchmark gate has been executed on physical GPU hardware. Autoregressive tensor inference using the Qwen 7B GGUF (`Q4_K_M`) model was verified end-to-end using Candle's CUDA backend (`candle-core` 0.8.4, `candle-nn` 0.8.4, `candle-transformers` 0.8.4, `cudarc` 0.13.9) compiled with CUDA Toolkit 12.8.93 on an NVIDIA GeForce RTX 4050 Laptop GPU (6 GB VRAM).

In release compilation mode (`opt-level = 3`) with persistent CUDA `Device` handle reuse, single-request warm inference latency was reduced from the `19.86 s` debug baseline down to **`7.639 s` min / `7.833 s` mean** for a 16-token generation request (**2.04 tokens/sec**, **489.56 ms/token**). Compared to the original CPU baseline (`938.96 s`), this represents a **~119.86x overall performance speedup**.

However, the warm-inference latency of `7.639 s` exceeds the gate requirement of **`< 700 ms`**. Therefore, the performance target is **NOT MET**, requiring the system to advance to the next gate: **`DEEP QWEN CUDA KERNEL PERFORMANCE OPTIMIZATION`**.

---

## 1. System & Hardware Environment Verification

| Parameter | Confirmed Value |
| :--- | :--- |
| **GPU Model** | NVIDIA GeForce RTX 4050 Laptop GPU |
| **VRAM Capacity** | 6,141 MiB (6 GB VRAM) |
| **NVIDIA Driver Version** | 596.36 |
| **CUDA Toolkit Version** | 12.8.93 (`nvcc` path: `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.8\bin\nvcc.exe`) |
| **Candle Framework** | v0.8.4 (`candle-core`, `candle-nn`, `candle-transformers`) |
| **Cudarc Binding** | v0.13.9 |
| **Model Path** | `C:\naina-os\models\qwen-7b-instruct-q4_k_m.gguf` |
| **Model File Size** | 4,767,104,224 bytes (4.77 GB) |
| **Model SHA256 Hash** | `D7F132B1EFF9CE35ACF8E83AB96D2BC87EAEDB68244E467BBC99E9F46A122A4C` |
| **CUDA Device** | `Device::Cuda(0)` initialized |

---

## 2. Benchmark Execution Results

### 2.1 Latency Breakdown

| Phase / Run | Latency | Output / Details |
| :--- | :--- | :--- |
| **CUDA Init Time** | `315.59 ms` | Drivers, CUDA context & handle creation |
| **Model Load Time** | `10.190 s` | Reading 4.77 GB GGUF file from disk & uploading 387 quantized tensors to VRAM |
| **Cold Start Total** | `10.506 s` | Total elapsed time before first prompt generation |
| **Warm Run 1** | `8.503 s` | 16 tokens generated (`"\nNaina Online.\n\n\n\n产融"`) |
| **Warm Run 2** | `7.729 s` | 16 tokens generated |
| **Warm Run 3** | `7.648 s` | 16 tokens generated |
| **Warm Run 4** | `7.639 s` | 16 tokens generated |
| **Warm Run 5** | `7.645 s` | 16 tokens generated |

### 2.2 Summary Metrics (5 Warm Runs)

* **Minimum Warm Latency**: `7.639 s`
* **Maximum Warm Latency**: `8.503 s`
* **Mean Warm Latency**: `7.833 s`
* **Median Warm Latency**: `7.648 s`
* **Tokens Generated per Run**: 16.0 tokens
* **Tokens per Second (Throughput)**: **`2.04 tok/s`**
* **Per-Token Latency**: **`489.56 ms/token`**
* **Initial Estimated VRAM Allocation**: `322 MiB` (conservative baseline check) / Active model memory `~4.3 GB`
* **Peak GPU Utilization**: `22%`
* **Original CPU Baseline Latency**: `938.96 s`
* **GPU Speedup vs CPU Baseline**: **~119.86x speedup**
* **Performance Gate Target**: `< 700 ms`
* **Gate Target Status**: **`TARGET NOT MET`** (`7.639 s` > `0.700 s`)

---

## 3. Bottleneck Identification & Analysis

Source-level inspection and profiling of Candle's Qwen2 GGUF execution loop (`packages/model-providers/src/qwen_gguf.rs`) identified two primary root causes for the `489.56 ms/token` per-token latency:

1. **Quantized Matrix-Vector (Q4_K) Memory & Unpacking Overheads**:
   - Each token generation step calls `ModelWeights::forward(&input_tensor, pos)`.
   - Across Qwen 7B's 28 transformer layers, generating a single token requires iterating through all 387 GGUF quantized tensors (Q4_K, Q6_K).
   - On the RTX 4050 (194 GB/s theoretical memory bandwidth), sweeping 4.3 GB of model weights per token requires `~22 ms` minimum hardware memory transfer time.
2. **Kernel Launch Frequency & Framework Overhead**:
   - Each token generation launches ~196 distinct CUDA kernels (RMSNorm, RoPE, Q4_K MatVec, Softmax, MLP projections, LM head).
   - In Candle 0.8.4, host-side synchronization, intermediate tensor allocations, and un-batched kernel launch latency accumulate ~460 ms of host CPU and driver overhead per token.

---

## 4. Applied Safe Optimizations

| Optimization | Change Location | Measurement / Effect |
| :--- | :--- | :--- |
| **Release Build Profile** | `--release` (`opt-level = 3`) | Latency dropped from `19.86 s` to `8.093 s` (**~2.45x speedup**) |
| **Device Handle Reuse** | `packages/model-providers/src/qwen_gguf.rs` | Stored `device: Device` once in `QwenGgufAdapter` struct and reused across `load_model()` and `generate()`, eliminating redundant CUDA driver context initialization. Reduced warm mean latency from `8.093 s` to `7.833 s` (min `7.639 s`). |
| **Strict Error Propagation** | `packages/model-providers/src/qwen_gguf.rs` | Added explicit error returns in `load_model()` for GGUF parsing and tokenizer loading failure paths. |

---

## 5. Regression Safety Suite Results

All workspace regression checks were executed and confirmed 100% passing:

```
1. cargo fmt --all -- --check                              [PASS]
2. cargo check -p model-providers --features cuda           [PASS]
3. cargo test -p model-providers --features cuda -- --test-threads=1 [PASS] (16/16 tests passed)
4. cargo clippy -p model-providers --all-targets --features cuda -- -D warnings [PASS]
5. cargo check --workspace                                 [PASS]
6. cargo clippy --workspace --all-targets -- -D warnings   [PASS]
```

---

## 6. Mandatory Next Gate Recommendation

Because warm inference latency (`7.639 s`) remains above the `< 700 ms` performance threshold:

**`STATUS: FUNCTIONALLY VERIFIED — PERFORMANCE TARGET NOT MET`**  
**`NEXT GATE = DEEP QWEN CUDA KERNEL PERFORMANCE OPTIMIZATION`**
