# NAINA OS — Orchestrator (`orchestrator`)

Top-level agent execution orchestration, CARF task planning, multi-step workflow loop execution, context assembly, model inference dispatch, tool execution routing, and cognitive pipeline supervision for NAINA OS.

## Architecture

```
apps → orchestrator → (runtime, memory, context-engine, model-runtime, tool-registry) → (configuration, logging)
```

## Features
- **CARF Task Planning**: Task breakdown and multi-step plan generation.
- **Context Assembly**: Deterministic conversation context assembly via `ContextEngine`.
- **Model Inference Dispatch**: Inference request routing via `ModelRuntime`.
- **Tool Execution Routing**: Tool execution and capability validation via `ToolRegistry`.
- **Memory Integration**: Multi-turn history and semantic recall via `MemoryStore`.
