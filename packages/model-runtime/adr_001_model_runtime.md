# ADR-001: NAINA OS Model Layer Architecture & Dependency DAG Resolution

- **Title:** ADR-001: NAINA OS Model Layer Architecture & Dependency DAG Resolution
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-22
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/model-runtime` & `packages/model-providers`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

*Implementation Permitted: NO*

---

## 2. Context & Problem Statement
NAINA OS operates under strict directed-acyclic-graph (DAG) package rules (`DEPENDENCY_MAP.md` Rule 33 & `PACKAGE_RULES.md` Section 2). 

An architecture investigation revealed a circular dependency in `engineering/DEPENDENCY_MAP.md`:
- Row 20 (`model-runtime`): Lists `Depends On: configuration, logging, model-providers`.
- Row 21 (`model-providers`): Lists `Depends On: model-runtime, configuration, logging`.

This bidirectional dependency (`model-runtime ↔ model-providers`) violates the core DAG requirement. An ADR is required to resolve ownership, invert the dependency direction, establish an acyclic package hierarchy, and define the model runtime abstraction contract before implementation.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **First Alpha Specification (`FIRST_ALPHA_SPEC.md`)**:
  - Section 6 & Section 7 (Checklist Item 2): ARAL Model Runtime & Qwen 7B GGUF Adapter (`IModelAdapter`). Loads Qwen 7B GGUF model via `IModelAdapter` and streams response tokens.
  - Section 4 (Workstation Budget): Active pipeline GPU VRAM Peak (Qwen 7B GGUF) `< 4.8 GB`.
- **Root Configuration (`packages/configuration/src/models.rs`)**:
  - `configuration::ModelConfig { pub provider: String, pub model_name: String, pub context_window: usize }`.
- **Zero-Trust Security & Microkernel Rules (`PROJECT_CHARTER.md` & `PACKAGE_RULES.md`)**:
  - Dependencies must form a strict directed acyclic graph.
  - No direct Cargo dependencies on higher-level packages (`orchestrator`, `runtime`, `services`, `tool-registry`, `memory`, `context-engine`).
  - Pure Rust microkernel core with zero unapproved external async runtimes (no Tokio).

### B. ARCHITECTURAL DECISIONS PROPOSED BY THIS ADR:
- Adopt **Option A**: `model-providers` depends on `model-runtime`, and `model-runtime` depends ONLY on `configuration` and `logging`.
- `model-runtime` owns all foundational traits, manager structs, data models, and error types.
- `model-providers` owns concrete provider adapters (`QwenGgufAdapter`, `MockModelProvider`).
- Pure Rust token streaming iterator (`TokenStream`) operating without third-party async runtimes.
- Formal proposal to update `DEPENDENCY_MAP.md` Row 20 to remove `model-providers` from `model-runtime`'s `Depends On`.

---

## 4. Architectural Decisions (ADR-001)

### 1. Resolution of the Model Layer Dependency Cycle (Option A)
To eliminate the circular dependency and maintain an acyclic DAG, package ownership is partitioned as follows:
- **`packages/model-runtime`** is the **Model Execution Abstraction Layer**. It depends ONLY on `configuration` and `logging`. It does NOT depend on `model-providers`.
- **`packages/model-providers`** is the **Concrete Provider Implementation Layer**. It depends on `model-runtime`, `configuration`, and `logging`.

```
                    DAG ARCHITECTURE
                  ┌──────────────────┐
                  │   orchestrator   │
                  └────────┬─────────┘
                           │
                           ▼
                  ┌──────────────────┐
                  │ model-providers  │
                  └────────┬─────────┘
                           │
                           ▼
                  ┌──────────────────┐
                  │  model-runtime   │
                  └────────┬─────────┘
                           │
            ┌──────────────┴──────────────┐
            ▼                             ▼
  ┌──────────────────┐          ┌──────────────────┐
  │  configuration   │          │     logging      │
  └──────────────────┘          └──────────────────┘
```

### 2. Confirmation: No New Abstraction Package Required
`model-runtime` itself serves as the abstraction package. Creating a third package (e.g. `model-traits`) is unneeded and avoided.

### 3. Exact Required Change to `DEPENDENCY_MAP.md`
Proposed update to `engineering/DEPENDENCY_MAP.md` (Row 20 & 21):
```diff
- | model-runtime | configuration, logging, model-providers | orchestrator | ModelRuntime |
+ | model-runtime | configuration, logging | orchestrator, model-providers | ModelRuntime |
  | model-providers | model-runtime, configuration, logging | orchestrator | ModelProvider |
```

### 4. Ownership Partitioning

| Domain / Concept | Owning Package | Module Location |
| :--- | :--- | :--- |
| `ModelRuntime` Manager Struct | `model-runtime` | `packages/model-runtime/src/model_runtime.rs` |
| `ModelProvider` Trait (`IModelAdapter`) | `model-runtime` | `packages/model-runtime/src/traits.rs` |
| `ModelRequest`, `ModelResponse`, `TokenStream`, `InferenceParams` | `model-runtime` | `packages/model-runtime/src/types.rs` |
| `ModelRuntimeError`, `Result<T>` | `model-runtime` | `packages/model-runtime/src/error.rs` |
| `ModelRuntimeConfig` | `model-runtime` | `packages/model-runtime/src/config.rs` |
| `QwenGgufAdapter` (Qwen 7B GGUF) | `model-providers` | `packages/model-providers/src/qwen_gguf.rs` |
| `MockModelProvider` (Offline Test Provider) | `model-providers` | `packages/model-providers/src/mock.rs` |

### 5. `model-runtime` Data Models & Traits

```rust
// packages/model-runtime/src/types.rs

#[derive(Clone, Debug, PartialEq)]
pub struct InferenceParams {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: usize,
    pub stop_sequences: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelRequest {
    pub model_name: String,
    pub prompt: String,
    pub params: InferenceParams,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelResponse {
    pub text: String,
    pub tokens_generated: usize,
    pub finish_reason: String,
}

/// Token streaming handle wrapping a thread-safe token iterator.
pub struct TokenStream {
    receiver: std::sync::mpsc::Receiver<String>,
}
```

```rust
// packages/model-runtime/src/traits.rs

pub trait ModelProvider: Send + Sync + std::fmt::Debug {
    fn provider_name(&self) -> &str;
    fn is_model_supported(&self, model_name: &str) -> bool;
    fn load_model(&self, model_name: &str) -> Result<(), crate::error::ModelRuntimeError>;
    fn generate(&self, request: &crate::types::ModelRequest) -> Result<crate::types::ModelResponse, crate::error::ModelRuntimeError>;
    fn generate_stream(&self, request: &crate::types::ModelRequest) -> Result<crate::types::TokenStream, crate::error::ModelRuntimeError>;
    fn current_vram_usage_bytes(&self) -> usize;
}
```

### 6. Model Lifecycle & VRAM Resource Management
`ModelRuntime` tracks model state and enforces the `< 4.8 GB` peak VRAM budget:
- **Lifecycle States**: `Unloaded`, `Loading`, `Ready`, `Inferring`, `Error`.
- **VRAM Enforcement**: `ModelRuntime` queries `provider.current_vram_usage_bytes()`. If a requested model load would exceed `4.8 GB` (4,800,000,000 bytes), returns `Err(ModelRuntimeError::VramExceeded)`.

### 7. Token Streaming Without Tokio
- Uses standard library `std::sync::mpsc::channel()` channels inside `TokenStream`.
- Guarantees token streaming without third-party async runtimes or Tokio dependencies.

### 8. Windows Native C++ Binding Risk Mitigation
- `QwenGgufAdapter` provides a pure Rust interface wrapper. Native C++ bindings (e.g. `llama.cpp` bindings) are dynamically loaded or wrapped behind feature flags.
- `MockModelProvider` is always compiled by default for deterministic CI unit testing without native GPU/C++ compiler requirements.

---

## 5. Error Model (`ModelRuntimeError`)

```rust
#[derive(Debug)]
pub enum ModelRuntimeError {
    ModelNotFound { model_name: String },
    ProviderNotFound { provider_name: String },
    LoadFailed { message: String },
    InferenceFailed { message: String },
    VramExceeded { limit_bytes: usize, requested_bytes: usize },
    ConfigError { message: String },
    LockError { message: String },
    Configuration(configuration::ConfigError),
}
```

---

## 6. Security & Dependency DAG Compliance

### Direct Dependencies:
- **`model-runtime`**: `configuration`, `logging` ONLY.
- **`model-providers`**: `model-runtime`, `configuration`, `logging` ONLY.

### Forbidden Dependencies:
`runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, `tool-registry`, `memory`, `context-engine`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`.

### Privacy Rules:
Model prompt text must be sanitized before execution; prompt text must not be emitted to disk logs in plain text.

---

## 7. Alpha Scope & Deferred Features

### Alpha Scope:
- `ModelRuntime` manager and provider registration.
- `ModelProvider` trait (`IModelAdapter`) definition.
- `QwenGgufAdapter` for Qwen 7B GGUF model execution.
- `MockModelProvider` for offline testing.
- `< 4.8 GB` VRAM budget tracking.
- Standard library `mpsc` token streaming (`TokenStream`).

### Explicitly Deferred to Beta:
- ❌ Multi-GPU distributed inference.
- ❌ Dynamic quantization / ONNX runtime engines.
- ❌ Remote cloud model API proxying (NSP).

---

## 8. Architecture Risks & Mitigation

1. **Native C++ Build Failures on Windows**: *Mitigation*: `MockModelProvider` provides fallback execution for CI/CD.
2. **VRAM Memory Leak**: *Mitigation*: `ModelRuntime` enforces explicit model unloading hooks.
3. **DAG Regression**: *Mitigation*: `model-runtime` manifest strictly forbids depending on `model-providers`.

---

## IMPLEMENTATION CONTRACT

### `model-runtime` Public API:
- Structs: `ModelRuntime`, `ModelRuntimeConfig`, `ModelRequest`, `ModelResponse`, `TokenStream`, `InferenceParams`
- Traits: `ModelProvider`
- Enums: `ModelRuntimeError`
- Type aliases: `Result<T> = std::result::Result<T, ModelRuntimeError>`

### `model-providers` Public API:
- Structs: `QwenGgufAdapter`, `MockModelProvider`

Status: PROPOSED — PENDING REVIEW  
Implementation permitted: NO
