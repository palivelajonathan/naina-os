# ADR-001: NAINA OS Runtime Execution Architecture

- **Title:** ADR-001: NAINA OS Runtime Execution Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-21
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/runtime`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

*Implementation Permitted: NO*

---

## 2. Context
NAINA OS operates on a microkernel architecture (`PROJECT_CHARTER.md` Section 3). Foundational packages (`configuration`, `logging`, `event-bus`, `capabilities`, `kernel`) are fully implemented. `runtime` is the Layer 3 package providing the unified runtime execution framework that bridges `kernel` process supervision to higher-level user-space services and domain runtimes (`DEPENDENCY_MAP.md`).

Having completed `kernel`, `runtime` is the next package in the dependency DAG.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **Dependency Matrix (`DEPENDENCY_MAP.md`)**: `runtime` directly depends on `kernel`. It is consumed by `apps`, `services`, and `orchestrator`.
- **Microkernel Isolation (`PROJECT_CHARTER.md` Section 3)**: Keep kernel logic lightweight, pushing subsystem execution into isolated user-space process contexts.
- **Cold Boot Budget (`FIRST_ALPHA_SPEC.md` Section 3)**: System cold boot overhead `< 2.0s`.
- **Runtime Performance Overhead (`PROJECT_CHARTER.md` Section 4)**: Local runtime execution overhead `< 200ms`.
- **Zero-Trust Security (`PROJECT_CHARTER.md` Section 2)**: All sensitive runtime operations must verify capability permissions.

### B. ADR DECISIONS (Introduced by this ADR):
- Struct definitions for `Runtime`, `RuntimeConfig`, `ExecutionContext`, `ExecutionContextId`.
- Enum variants for `RuntimeState` (`Uninitialized`, `Initializing`, `Ready`, `ShuttingDown`, `Stopped`, `Failed`).
- Enum variants for `ContextState` (`Created`, `Running`, `Completed`, `Failed`, `Closed`).
- `CAP_RUNTIME_EXECUTE` capability identifier for execution context creation.
- Event Bus topic strings (`"runtime.initializing"`, `"runtime.ready"`, `"runtime.shutdown"`).

---

## 4. Problem Statement
The primary engineering documents establish that `runtime` provides unified execution management, but omit concrete struct definitions, execution context models, state machine transitions, and error enums. An ADR is required to specify the minimal safe Alpha runtime contract without introducing heavy async runtimes (Tokio), C++ dependencies, or circular references to higher-level packages.

---

## 5. Architectural Decisions (ADR-001)

1. **Thin Layer 3 Runtime Above Kernel**: `Runtime` wraps an `Arc<kernel::Kernel>` instance, delegating process supervision and state tracking to `Kernel` while managing user-space `ExecutionContext`s.
2. **Explicit Runtime State Machine**: Runtime lifecycle strictly follows: `Uninitialized` -> `Initializing` -> `Ready` -> `ShuttingDown` -> `Stopped` / `Failed`.
3. **In-Memory Execution Context Model**: Subsystem worker tasks execute within lightweight `ExecutionContext`s identified by `ExecutionContextId(pub u64)`.
4. **Synchronous In-Memory Execution**: Context creation, execution tracking, and state transitions execute synchronously using standard library primitives (`std::sync::RwLock`, `std::sync::atomic::AtomicU64`), satisfying the `< 200ms` runtime overhead target.
5. **Zero-Trust Capability Enforcement**: Context creation authorizes `CAP_RUNTIME_EXECUTE` via the underlying `CapabilityRegistry` accessible through `Kernel`.
6. **Lifecycle Event Propagation**: `Runtime` emits `"runtime.initializing"`, `"runtime.ready"`, and `"runtime.shutdown"` notifications via `EventBus`.

---

## 6. Runtime Lifecycle

```
  [Uninitialized]
         │
    initialize()
         ▼
   [Initializing] ──────(init failure)──────> [Failed]
         │
      start()
         ▼
      [Ready]
         │
    shutdown()
         ▼
  [ShuttingDown]
         │
         ▼
     [Stopped]
```

---

## 7. Execution Context Model

Each runtime execution environment is tracked via an `ExecutionContext`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionContextId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextState {
    Created,
    Running,
    Completed,
    Failed { error: String },
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionContext {
    pub id: ExecutionContextId,
    pub name: String,
    pub state: ContextState,
    pub kernel_process_id: kernel::ProcessId,
}
```

---

## 8. Runtime API

```rust
pub struct Runtime {
    config: RuntimeConfig,
    kernel: std::sync::Arc<kernel::Kernel>,
    state: std::sync::RwLock<RuntimeState>,
    contexts: std::sync::RwLock<std::collections::BTreeMap<ExecutionContextId, ExecutionContext>>,
    next_context_id: std::sync::atomic::AtomicU64,
}
```

---

## 9. Kernel Integration
When a new `ExecutionContext` is created via `Runtime::create_context`, `Runtime` invokes `Kernel::register_process` to obtain a `kernel::ProcessId`. `Kernel` remains the authoritative supervisor for process state, crash recovery, and restart retries.

---

## 10. Capability Security
Sensitive context creation operations verify `CAP_RUNTIME_EXECUTE` against `CapabilityRegistry` when a token is supplied.

---

## 11. EventBus Integration
During lifecycle state changes, `Runtime` emits events:
- `"runtime.initializing"`
- `"runtime.ready"`
- `"runtime.shutdown"`

---

## 12. Logging
Runtime state changes and execution failures are logged via `logging::Logger`. Tokens, keys, and secrets are strictly excluded from logs.

---

## 13. Concurrency Model
- Internal state synchronized via `std::sync::RwLock` and `AtomicU64`.
- `Runtime` is `Send + Sync` and shareable across threads via `std::sync::Arc<Runtime>`.

---

## 14. Performance Requirements
- Cold boot contribution: `< 50ms` (within the `< 2.0s` total cold boot budget).
- Execution context creation & dispatch overhead: `< 200ms` (matching `PROJECT_CHARTER.md` Section 4).

---

## 15. Alpha Scope
1. In-memory `Runtime` struct wrapping `Arc<Kernel>`.
2. `initialize()`, `start()`, `shutdown()`, `create_context()`, `execute_context()`, `close_context()` methods.
3. Integration with `Kernel` process registration.
4. Unit tests covering lifecycle transitions, context creation, execution, and graceful shutdown.

---

## 16. Deferred Capabilities (Beta / Production)
- ❌ Tokio or third-party async runtimes.
- ❌ OS process containers and PID namespace sandboxing.
- ❌ C++ PREEMPT_RT kernel real-time scheduling.
- ❌ WASM plugin execution sandboxes.
- ❌ Remote IPC execution bridges.

---

## 17. Alternatives Considered
1. **Direct Tokio Async Executor in Alpha**: Rejected to maintain zero unvetted external dependencies and keep cold boot overhead strictly `< 2.0s`.
2. **Duplicating Kernel Supervisor in Runtime**: Rejected to preserve single-responsibility microkernel isolation (`kernel` handles supervision; `runtime` handles execution context).

---

## 18. Risks
- *Process Mismatch*: `ExecutionContext` and `Kernel` `ProcessId` desynchronization. *Mitigation*: Atomically create `ExecutionContext` and `ProcessId` during context creation.

---

## 19. Consequences
- Clean microkernel DAG maintained (`kernel` -> `runtime`).
- Zero external crate dependencies added.

---

## IMPLEMENTATION CONTRACT

Public structs:
- `pub struct Runtime`
- `pub struct RuntimeConfig`
- `pub struct ExecutionContextId(pub u64)`
- `pub struct ExecutionContext`

Public enums:
- `pub enum RuntimeState`
- `pub enum ContextState`
- `pub enum RuntimeError`

Public traits:
- None required for Alpha.

Public type aliases:
- `pub type Result<T> = std::result::Result<T, RuntimeError>`

Exact method signatures:
- `impl Runtime`:
  - `pub fn new(config: RuntimeConfig, kernel: std::sync::Arc<kernel::Kernel>) -> Self`
  - `pub fn initialize(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()>`
  - `pub fn start(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()>`
  - `pub fn shutdown(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()>`
  - `pub fn create_context(&self, name: impl Into<String>, capability_registry: Option<(&capabilities::CapabilityRegistry, &capabilities::CapabilityToken)>) -> Result<ExecutionContextId>`
  - `pub fn execute_context(&self, id: ExecutionContextId) -> Result<()>`
  - `pub fn close_context(&self, id: ExecutionContextId) -> Result<()>`
  - `pub fn state(&self) -> RuntimeState`
  - `pub fn get_context_state(&self, id: ExecutionContextId) -> Result<ContextState>`

Lifecycle transitions:
- `Uninitialized` -> `Initializing` -> `Ready` -> `ShuttingDown` -> `Stopped` / `Failed`

Context transitions:
- `Created` -> `Running` -> `Completed` / `Failed` -> `Closed`

Kernel integration:
- Wraps `Arc<kernel::Kernel>`. `create_context` registers a `kernel::ProcessId` with `Kernel`.

Capability requirements:
- Authorizes `CAP_RUNTIME_EXECUTE` via `CapabilityRegistry` during `create_context`.

Event types:
- `"runtime.initializing"`, `"runtime.ready"`, `"runtime.shutdown"`

Dependencies:
- `kernel = { path = "../kernel" }`

Forbidden dependencies:
- `services`, `orchestrator`, `tool-registry`, `memory`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`.

Deferred features:
- Tokio async executor, OS PID namespaces, WASM container sandboxing, C++ PREEMPT_RT real-time scheduling.

Implementation permitted:
NO
