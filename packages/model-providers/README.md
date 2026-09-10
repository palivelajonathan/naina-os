# NAINA OS — Model Providers (`model-providers`)

Concrete model provider adapters implementing `model_runtime::ModelProvider` for NAINA OS.

## Architecture

```
orchestrator → model-providers → model-runtime → (configuration, logging)
```

## Features
- **`QwenGgufAdapter`**: Local Qwen 7B GGUF model execution adapter using Hugging Face Candle.
- **`MockModelProvider`**: Deterministic offline test provider for CI test environments without GPU hardware or model weight files.
