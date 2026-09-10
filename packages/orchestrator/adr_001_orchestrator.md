# ADR-001: NAINA OS Orchestrator Architecture

- **Title:** ADR-001: NAINA OS Orchestrator Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-22
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/orchestrator`

---

## 1. Status
**APPROVED**

*Implementation Permitted: YES*

---

## 2. Context & Responsibility

NAINA OS uses a modular microkernel architecture. The `packages/orchestrator` package acts as the top-level cognitive agent engine and task supervisor for the operating system:
```
apps → orchestrator → (runtime, memory, context-engine, model-runtime, tool-registry) → (configuration, logging)
```

The orchestrator's primary responsibilities are:
1. Receiving user task requests (`TaskRequest`).
2. Invoking the Cognitive Agent Architecture Framework (CARF) planner to decompose goals into multi-step execution plans (`ExecutionPlan`).
3. Assembling context via `ContextEngine`.
4. Dispatching prompt requests to LLM backends via `ModelRuntime`.
5. Routing tool invocations through `ToolRegistry` and `Runtime`.
6. Storing conversational history and retrieved memories in `MemoryStore`.
7. Supervising step-by-step execution loop state machine transitions while enforcing safety limits, VRAM budgets, step limits, and crash recovery.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **First Alpha Specification (`FIRST_ALPHA_SPEC.md`)**:
  - Section 2 (MVN Core Pipeline): Voice/Task Input -> CARF Planner -> Qwen GGUF (ARAL Model) -> Obsidian Memory Vault / Tools -> Output.
  - Section 7 Checklist (Item 8): Executes 1 complete multi-step automation workflow (e.g. summarize webpage -> save note to Obsidian).
  - Section 7 Checklist (Item 9): Context preservation across at least 5 user turns.
  - Section 7 Checklist (Item 10): Graceful crash recovery without crashing the NKRS process supervisor.
  - Section 3: Performance Targets (< 700ms voice, < 300ms memory, < 500ms command execution).
- **Package Rules & Dependency Map (`engineering/DEPENDENCY_MAP.md`)**:
  - `orchestrator` depends directly on `runtime`, `memory`, `context-engine`, `model-runtime`, `tool-registry`, `configuration`, and `logging`.
  - `model-providers` MUST NOT be a direct dependency (accessed strictly via `model-runtime`).

### B. ARCHITECTURAL DECISIONS PROPOSED BY THIS ADR:
- Exact struct definitions: `Orchestrator`, `OrchestratorConfig`, `TaskRequest`, `TaskResponse`, `ExecutionPlan`, `ExecutionStep`, `ExecutionState`.
- Execution loop state machine transition rules (`Planning` -> `ExecutingStep` -> `Evaluating` -> `Completed` / `Failed`).
- Bounds enforcement: `max_steps = 10`, `timeout_seconds = 60`, `retry_limit = 2`.
- Fault isolation: Step failure handling without orchestrator process breakdown.

---

## 4. Architectural Decisions (ADR-001)

### A. Orchestrator Structure
```rust
pub struct Orchestrator {
    config: OrchestratorConfig,
    runtime: Arc<Runtime>,
    memory: Arc<MemoryStore>,
    context_engine: Arc<ContextEngine>,
    model_runtime: Arc<ModelRuntime>,
    tool_registry: Arc<ToolRegistry>,
}
```
State synchronization is managed via thread-safe atomic primitives or standard `std::sync::RwLock` containers (`Send + Sync`).

---

### B. OrchestratorConfig
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrchestratorConfig {
    pub max_steps: usize,         // Default: 10 steps max per task
    pub timeout_seconds: u64,     // Default: 60 seconds max per task
    pub retry_limit: usize,       // Default: 2 retries per failed step
}
```

---

### C. TaskRequest & TaskResponse
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRequest {
    pub task_id: String,
    pub prompt: String,
    pub conversation_id: Option<ConversationId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: ExecutionState,
    pub output: String,
    pub steps_executed: usize,
}
```

---

### D. ExecutionPlan, ExecutionStep & ExecutionState
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionState {
    Planning,
    ExecutingStep,
    Evaluating,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionStep {
    pub step_id: String,
    pub description: String,
    pub action_type: String, // "tool_call", "model_reasoning", "memory_query"
    pub input: String,
    pub output: Option<String>,
    pub state: ExecutionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub steps: Vec<ExecutionStep>,
}
```

---

### E. CARF Planner Integration
`orchestrator` exposes `pub trait Planner: Send + Sync + Debug` for intent routing and plan generation. The default planner parses high-level user prompt goals into a sequence of atomic `ExecutionStep` instances.

---

### F. ContextEngine Integration
Before dispatching inference steps to `ModelRuntime`, `orchestrator` calls `context_engine.assemble_context(conversation_id, prompt)` to assemble system instructions, retrieved memory context, and sliding-window conversation history.

---

### G. ModelRuntime Integration
Prompt inference is dispatched via `model_runtime.generate(provider_name, request)` or `generate_stream(...)`. Model providers are never accessed directly.

---

### H. ToolRegistry Integration
Tool calls requested during plan steps are executed via `tool_registry.execute_tool(tool_name, params, capability_token)`. Capability tokens are validated by `ToolRegistry` and `Runtime`.

---

### I. Memory Integration
Direct semantic memory search and history logging are delegated to `MemoryStore` and `ContextEngine`. Completed task steps update conversation history via `context_engine.add_turn(...)`.

---

### J. Runtime Integration
The `orchestrator` relies on `Runtime` to supervise worker process state and validate system resource bounds.

---

### K. Execution Loop State Machine
The execution loop follows a strict deterministic pipeline:
1. **`Planning`**: Generate `ExecutionPlan` from `TaskRequest`.
2. **`ExecutingStep`**: For each step in the plan:
   - Assemble context via `ContextEngine`.
   - Dispatch reasoning to `ModelRuntime` or tool to `ToolRegistry`.
   - Record output snippet in `ExecutionStep`.
3. **`Evaluating`**: Check if step succeeded or failed. If step failed and `retries < retry_limit`, retry step.
4. **`Completed`**: Return final `TaskResponse` when all steps succeed.
5. **`Failed`**: Halt task if `max_steps` is reached, timeout expires, or unrecoverable error occurs.

---

### L. Crash Recovery & Fault Tolerance
Tool or model execution failures return `OrchestratorError::ExecutionFailed` without crashing the `Orchestrator` process thread. The execution loop catches errors and transitions task state gracefully.

---

### M. Error Model
```rust
pub enum OrchestratorError {
    TaskFailed { task_id: String, reason: String },
    MaxStepsExceeded { task_id: String, max_steps: usize },
    Timeout { task_id: String, duration_seconds: u64 },
    PlanningFailed { message: String },
    ExecutionFailed { step_id: String, message: String },
    Runtime(runtime::RuntimeError),
    Memory(memory::MemoryError),
    ContextEngine(context_engine::ContextEngineError),
    ModelRuntime(model_runtime::ModelRuntimeError),
    ToolRegistry(tool_registry::ToolError),
    Configuration(configuration::ConfigError),
    LockError { message: String },
}
```

---

### N. Concurrency & Security
- Structs are `Send + Sync` (`Arc<Orchestrator>`).
- State synchronized via `std::sync::Mutex` / `RwLock`.
- Zero capability bypass; all tool calls pass through `ToolRegistry`.
- Zero plain-text prompt logging or credential disk leakage.

---

## 5. Architectural Risks & Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **Infinite Loops** | LLM repeatedly requesting tool steps | Enforce strict `max_steps = 10` hard ceiling |
| **Deadlocks** | Out-of-order lock acquisition | Lock acquisition hierarchy: Runtime -> Memory -> ContextEngine -> ModelRuntime -> ToolRegistry |
| **Unbounded Plans** | Planner generating 100+ steps | Truncate plan steps to `max_steps` during planning |
| **Process Crashes** | External tool panic | Isolate tool execution inside `catch_unwind` / `ToolRegistry` boundaries |

---

## IMPLEMENTATION CONTRACT

- **Public Struct**: `pub struct Orchestrator`
- **Allowed Direct Dependencies**: `runtime`, `memory`, `context-engine`, `model-runtime`, `tool-registry`, `configuration`, `logging`.
- **Forbidden Dependencies**: `model-providers`, `kernel`, `capabilities`, `event-bus`, `services`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`.

Status: APPROVED  
Implementation permitted: YES
