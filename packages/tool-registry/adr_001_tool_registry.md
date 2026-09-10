# ADR-001: NAINA OS Tool Registry Architecture

- **Title:** ADR-001: NAINA OS Tool Registry Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-21
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/tool-registry`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

*Implementation Permitted: NO*

---

## 2. Context
NAINA OS operates on a microkernel architecture (`PROJECT_CHARTER.md` Section 3). Lower-level packages (`configuration`, `logging`, `event-bus`, `capabilities`, `kernel`, `runtime`, `services`) are fully implemented and verified. `tool-registry` is the Layer 3 package providing capability-controlled tool registration, tool metadata management, tool lookup, and execution tracking for orchestrators and agent planners (`DEPENDENCY_MAP.md`).

Having completed `services`, `tool-registry` is ready in the dependency DAG.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **Dependency Matrix (`DEPENDENCY_MAP.md`)**: `tool-registry` depends on `capabilities` and `runtime`. Used by `orchestrator`. Public API: `ToolRegistry`.
- **Plugin Subsystem Budget (`FIRST_ALPHA_SPEC.md` Section 3)**: WASM Plugin / Tool load time budget `< 100ms`.
- **Microkernel Isolation (`PROJECT_CHARTER.md` Section 3)**: User-space processes manage tool execution environments.
- **Zero-Trust Security (`PROJECT_CHARTER.md` Section 2)**: All sensitive tool operations must authorize capability tokens via `CapabilityRegistry`.

### B. ADR DECISIONS (Introduced by this ADR):
- Struct definitions for `ToolRegistry`, `ToolRegistryConfig`, `ToolId(pub u64)`, `ToolName(pub String)`, `ToolDefinition`, `ToolRecord`.
- Enum variants for `ToolState`: `Registered`, `Active`, `Disabled`, `Failed`.
- Capability identifiers: `CAP_TOOL_REGISTER` (for registering tools) and `CAP_TOOL_EXECUTE` (for looking up and executing tools).
- Event Bus topic strings: `"tools.registered"`, `"tools.executed"`, `"tools.failed"`.
- Strongly typed `ToolError` enum and `Result<T>` alias.

---

## 4. Problem Statement
The primary engineering documents establish that `tool-registry` provides tool registration and execution lookup via `ToolRegistry`, but do not define the concrete struct fields, tool definition formats, lifecycle states, or capability security boundaries. An ADR is required to establish a minimal, safe, synchronous, in-memory Alpha contract.

---

## 5. Architectural Decisions (ADR-001)

1. **Layer 3 Tool Registry Framework**: `ToolRegistry` manages tool definitions, referencing `runtime::Runtime` for context execution while delegating authorization to `Capabilities`.
2. **In-Memory Thread-Safe Tool Store**: Tool records are stored in `std::sync::RwLock<BTreeMap<ToolId, ToolRecord>>` with name-to-ID indices in `RwLock<BTreeMap<String, ToolId>>`.
3. **Explicit Tool Lifecycle State Machine**: Tools transition through `Registered` -> `Active` -> `Disabled` / `Failed`.
4. **Synchronous Tool Execution**: Tools are executed synchronously in-memory, meeting the `< 100ms` load and execution budget.
5. **Capability Security Boundaries**:
   - `register_tool` requires `CAP_TOOL_REGISTER` authorization via `CapabilityRegistry`.
   - `lookup_tool`, `execute_tool`, and `list_tools` require `CAP_TOOL_EXECUTE` authorization via `CapabilityRegistry`.
6. **EventBus Notifications**: Tool registration and execution emit events (`"tools.registered"`, `"tools.executed"`, `"tools.failed"`).

---

## 6. Tool Lifecycle & Metadata Model

```
  [Registered]
       │
    activate()
       ▼
    [Active] ──── (execution failure) ────> [Failed]
       │
    disable()
       ▼
   [Disabled]
```

---

## 7. Tool Data Structures

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolName(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolState {
    Registered,
    Active,
    Disabled,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: ToolName,
    pub description: String,
    pub parameters_schema: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRecord {
    pub id: ToolId,
    pub definition: ToolDefinition,
    pub state: ToolState,
    pub execution_context_id: Option<runtime::ExecutionContextId>,
}
```

---

## 8. Tool Registry API

```rust
pub struct ToolRegistry {
    config: ToolRegistryConfig,
    runtime: std::sync::Arc<runtime::Runtime>,
    tools: std::sync::RwLock<std::collections::BTreeMap<ToolId, ToolRecord>>,
    by_name: std::sync::RwLock<std::collections::BTreeMap<String, ToolId>>,
    next_tool_id: std::sync::atomic::AtomicU64,
}
```

---

## 9. Capability Security
- `register_tool`: Authorizes `CAP_TOOL_REGISTER` against `CapabilityRegistry` when token provided.
- `lookup_tool`, `execute_tool`, `list_tools`: Authorize `CAP_TOOL_EXECUTE` against `CapabilityRegistry` when token provided.
- Preserves deny-by-default security.

---

## 10. Concurrency Model
- Internal state synchronized via `std::sync::RwLock` and `AtomicU64`.
- `ToolRegistry` is `Send + Sync` and shareable across threads via `std::sync::Arc<ToolRegistry>`.

---

## 11. Performance Requirements
- Tool registration & load time: `< 100ms` (`FIRST_ALPHA_SPEC.md` Section 3).
- In-memory tool lookup latency: `< 1ms`.

---

## 12. Alpha Scope
1. In-memory `ToolRegistry` wrapping `Arc<Runtime>`.
2. `register_tool()`, `lookup_tool()`, `get_tool()`, `execute_tool()`, `list_tools()` methods.
3. Integration with `CAP_TOOL_REGISTER` and `CAP_TOOL_EXECUTE` capability checks.
4. Unit tests covering registration, lookup, execution, capability authorization, and thread safety.

---

## 13. Deferred Capabilities (Beta / Production)
- ❌ Tokio async background tool workers.
- ❌ Dynamic WASM plugin sandboxing engines.
- ❌ Remote gRPC/REST RPC tool gateways.
- ❌ Dynamic JSON schema validation engines.

---

## 14. Alternatives Considered
1. **Dynamic WASM Plugin Runtime in Alpha**: Deferred to Beta to keep cold boot overhead strictly `< 2.0s` and tool load time `< 100ms`.
2. **Duplicating Runtime Context Execution**: Rejected to preserve single-responsibility microkernel isolation (`runtime` manages execution contexts; `tool-registry` manages tool definitions and lookups).

---

## 15. Risks & Mitigation
- *Unchecked Execution*: Invalid input payloads passed to tools. *Mitigation*: Validate JSON schema string presence prior to execution.

---

## 16. Consequences
- Clean microkernel DAG maintained (`capabilities` + `runtime` -> `tool-registry`).
- Zero external crate dependencies added.

---

## IMPLEMENTATION CONTRACT

Public structs:
- `pub struct ToolRegistry`
- `pub struct ToolRegistryConfig`
- `pub struct ToolId(pub u64)`
- `pub struct ToolName(pub String)`
- `pub struct ToolDefinition`
- `pub struct ToolRecord`

Public enums:
- `pub enum ToolState`
- `pub enum ToolError`

Public type aliases:
- `pub type Result<T> = std::result::Result<T, ToolError>`

Exact method signatures:
- `impl ToolRegistry`:
  - `pub fn new(config: ToolRegistryConfig, runtime: std::sync::Arc<runtime::Runtime>) -> Self`
  - `pub fn register_tool(&self, definition: ToolDefinition, execution_context_id: Option<runtime::ExecutionContextId>, capability_registry: Option<(&capabilities::CapabilityRegistry, &capabilities::CapabilityToken)>) -> Result<ToolId>`
  - `pub fn lookup_tool(&self, name: &str, capability_registry: Option<(&capabilities::CapabilityRegistry, &capabilities::CapabilityToken)>) -> Result<ToolRecord>`
  - `pub fn get_tool(&self, id: ToolId) -> Result<ToolRecord>`
  - `pub fn execute_tool(&self, id: ToolId, input_json: &str, capability_registry: Option<(&capabilities::CapabilityRegistry, &capabilities::CapabilityToken)>) -> Result<String>`
  - `pub fn list_tools(&self, capability_registry: Option<(&capabilities::CapabilityRegistry, &capabilities::CapabilityToken)>) -> Result<Vec<ToolRecord>>`

Lifecycle transitions:
- `Registered` -> `Active` -> `Disabled` / `Failed`

Capability requirements:
- Authorizes `CAP_TOOL_REGISTER` for tool registration.
- Authorizes `CAP_TOOL_EXECUTE` for tool lookup, execution, and listing.

Event types:
- `"tools.registered"`, `"tools.executed"`, `"tools.failed"`

Dependencies:
- `capabilities = { path = "../capabilities" }`
- `runtime = { path = "../runtime" }`

Forbidden dependencies:
- `orchestrator`, `memory`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`

Deferred features:
- Tokio async tool workers, WASM plugin sandboxing, remote gRPC RPC tool gateways.

Implementation permitted:
NO
