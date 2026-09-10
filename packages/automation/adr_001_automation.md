# ADR-001: NAINA OS Automation Engine Architecture

- **Title:** ADR-001: NAINA OS Automation Engine Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-24
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/automation`

---

## 1. Context
NAINA OS uses a modular microkernel architecture. The `packages/automation` package acts as the multi-step workflow execution engine for the operating system:
```
apps → automation → (runtime, services) → (kernel, capabilities, configuration, logging)
```

The automation engine supervises multi-step workflow execution, step sequencing, tool routing, action triggering, conditional branching, and workflow execution state tracking for NAINA OS.

---

## 2. Source-Supported Requirements
- **First Alpha Specification (`FIRST_ALPHA_SPEC.md`)**:
  - Section 2 (MVN Core Pipeline): Workflow Automation Engine execution pipeline.
  - Section 3: Plugin Subsystem WASM load target `< 100 ms`.
  - Section 6: Milestone Tracker Week 8 (Workflow Automation Engine & CARF Planner Integration).
  - Section 7 Checklist (Item 8): Automation Workflow (Executes 1 complete multi-step workflow e.g. summarize webpage → save note to Obsidian).
  - Section 7 Checklist (Item 10): Automatically restarts failed worker processes without crashing the host microkernel supervisor.
- **Dependency Map (`engineering/DEPENDENCY_MAP.md`)**:
  - `automation` depends directly on `runtime`, `services`, `configuration`, and `logging`.
  - All other packages (`orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `voice-runtime`, `desktop-runtime`, `browser-runtime`, `sdk`, `ui`, `kernel`, `capabilities`, `event-bus`) MUST NOT be direct dependencies of `automation`.
- **Project Charter (`PROJECT_CHARTER.md`)**:
  - Zero-Trust Security & Privacy: Capability tokens (`CBAC`) enforced via `ServiceRegistry` and `Runtime`; zero plain-text credential logging.

---

## 3. Dependency DAG Boundaries
- **Allowed Direct Dependencies**:
  - `runtime = { path = "../runtime" }`
  - `services = { path = "../services" }`
  - `configuration = { path = "../configuration" }`
  - `logging = { path = "../logging" }`

- **Explicit Forbidden Dependencies**:
  `orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `voice-runtime`, `desktop-runtime`, `browser-runtime`, `sdk`, `ui`, `kernel`, `capabilities`, `event-bus`.

---

## 4. AutomationEngine Architecture
```rust
pub struct AutomationEngine {
    config: AutomationConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    state: RwLock<AutomationState>,
    runner: RwLock<Arc<dyn WorkflowStepRunner>>,
    cancel_flag: Arc<AtomicBool>,
    logger: Mutex<Logger>,
}
```
`AutomationEngine` implements `Send + Sync` using internal synchronization primitives (`RwLock`, `Mutex`, `AtomicBool`).

---

## 5. AutomationConfig
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomationConfig {
    pub max_steps_per_workflow: usize,   // default: 20
    pub step_timeout_ms: u64,             // default: 5,000 ms
    pub workflow_timeout_ms: u64,         // default: 30,000 ms
    pub max_retry_attempts: u32,          // default: 3
    pub wasm_plugin_timeout_ms: u64,      // default: 100 ms (target)
    pub enabled: bool,                    // default: true
}
```
*Note: The 100ms WASM value is an architectural performance target, not a hard execution condition.*

---

## 6. Workflow Data Model
```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkflowSpec {
    pub workflow_id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkflowStep {
    pub step_id: String,
    pub action_type: String,
    pub payload: String,
    pub condition: Option<String>,
    pub on_failure_retry: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StepResult {
    pub step_id: String,
    pub status: String,
    pub output: Option<String>,
    pub elapsed_ms: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkflowExecutionResult {
    pub workflow_id: String,
    pub status: String,
    pub step_results: Vec<StepResult>,
    pub total_elapsed_ms: u64,
}
```

---

## 7. Workflow State Machine
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutomationState {
    Idle,
    Validating,
    Executing,
    Evaluating,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}
```
- **Transitions**:
  - `Idle` → `Validating` → `Executing` → `Evaluating` → `Executing` (next step) → `Completed` → `Idle`
  - `Executing` / `Evaluating` → `Failed` → `Idle`
  - `Executing` / `Evaluating` → `Cancelled` → `Idle`
  - `Executing` / `Evaluating` → `TimedOut` → `Idle`
- Invalid transitions are explicitly rejected.

---

## 8. Step Execution Boundary
- `Runtime`: Owns execution contexts (`Arc<Runtime>`).
- `ServiceRegistry`: Owns service registration and lookup (`Arc<ServiceRegistry>`).
- CBAC Capability Authorization: Enforced strictly at `Runtime` and `ServiceRegistry` boundaries.
- `AutomationEngine` MUST NOT bypass capability token checks and MUST NOT call higher-level engine runtimes directly.

---

## 9. CARF Integration
- **Boundary Lock**: `CARF INTEGRATION API: UNSPECIFIED` in direct package dependencies.
- `automation` MUST NOT depend on `orchestrator`.
- Higher-level layers (`apps` or `orchestrator`) translate CARF execution plans (`ExecutionPlan`) into `WorkflowSpec` before passing them to `AutomationEngine`.
- `AutomationEngine` consumes `WorkflowSpec` only.

---

## 10. Conditional Branching
- `WorkflowStep.condition` controls whether a step executes.
- If `Some(condition)`, the condition is evaluated: `true` executes the step, `false` skips the step.
- Maximum nested branching depth: `5 levels`.
- Branch termination is deterministic. Recursive uncontrolled branching is strictly prohibited.

---

## 11. Retry Policy
- Maximum retry limit: `3 attempts per step`.
- Retryable failures: temporary timeouts or transient service unavailability.
- Non-retryable failures: capability authorization denial (`CapabilityDenied`) or invalid payload.
- Exponential backoff: `100ms * 2^attempt`.
- Controlled `AutomationError` returned on step failure; zero supervisor crash.

---

## 12. Timeout & Cancellation
- Per-step timeout: `5,000 ms`. Total workflow timeout: `30,000 ms`.
- Cancellation token: `Arc<AtomicBool>` checked before and during step execution.
- Resources cleaned up on cancellation.
- **NO TOKIO OR UNAPPROVED ASYNC RUNTIMES**. Uses standard library `std::thread` OS background worker channels with `std::sync::mpsc`.

---

## 13. WASM Plugin Boundary
- **Option B Selected**: WASM runtime compilation is explicitly deferred to post-Alpha (`FIRST_ALPHA_SPEC.md` Section 5 Non-Goals), while retaining the `< 100 ms` load time target as an architectural performance target.
- Rationale: Prevents unvetted external WASM runtime dependencies from entering the Alpha microkernel DAG while preserving architectural performance metrics.

---

## 14. Error Model
```rust
#[derive(Debug)]
pub enum AutomationError {
    WorkflowValidationFailed { message: String },
    StepExecutionFailed { step_id: String, message: String },
    WorkflowTimeout { workflow_id: String },
    WorkflowCancelled { workflow_id: String },
    CapabilityDenied { step_id: String, message: String },
    Runtime(runtime::RuntimeError),
    Service(services::ServicesError),
    Configuration(configuration::ConfigError),
    LockError { message: String },
}
```

---

## 15. Security
- CBAC authorization enforced on every workflow step action via `ServiceRegistry` and `Runtime`.
- Zero plain-text credentials, authentication keys, cookies, or secret tokens logged to disk.
- Workflow input payloads must be validated before step execution.

---

## 16. Concurrency
- `AutomationEngine` implements `Send + Sync`.
- Synchronization via `std::sync::RwLock` and `Mutex`. Core wrapper contains zero unsafe code.
- Lock acquisition order: `state` → `runner` → `logger` (prevents deadlocks).

---

## 17. Resource & Loop Protection
- Maximum allowed steps per workflow: `max_steps_per_workflow = 20`.
- Maximum nested branch depth: `5 levels`.
- Per-step timeout: `5,000 ms`.
- Workflow timeout: `30,000 ms`.
- Maximum retry limit: `3 attempts`.

---

## 18. Performance
- WASM/plugin load target: `< 100 ms` (diagnostic/engineering target).
- Workflow execution managed within workstation memory limits.

---

## 19. Alpha Scope
- **INCLUDED IN ALPHA**:
  - Multi-step workflow execution
  - Sequential steps
  - Conditional branching
  - Step status telemetry
  - Cancellation & timeout enforcement
  - Fault isolation
  - Deterministic `MockWorkflowStepRunner` for CI testing
- **DEFERRED**:
  - Cloud workflow synchronization
  - External marketplace plugin installation
  - Multi-user workflow permissions
  - WASM runtime engine execution

---

## 20. Architecture Risks & Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **Infinite Step Loops** | Cyclic step conditions | Enforce hard limit `max_steps_per_workflow = 20` |
| **Recursive Deadlocks** | Nested branch calls | Enforce max branch depth = 5 and strict lock ordering |
| **Tokio Contamination** | Unapproved async runtimes in microkernel | Use `std::thread` + `std::sync::mpsc` for background workers |
| **Capability Bypass** | Dynamic step execution bypassing token checks | Enforce CBAC check at `Runtime`/`ServiceRegistry` boundaries |

---

## 21. Public API
```rust
pub struct AutomationEngine;
pub struct AutomationConfig;
pub struct WorkflowSpec;
pub struct WorkflowStep;
pub struct StepResult;
pub struct WorkflowExecutionResult;
pub enum AutomationState;
pub enum AutomationError;
pub trait WorkflowStepRunner;
```

---

## 22. Alternatives Considered
- **Direct dependency on `orchestrator`**: REJECTED (violates Microkernel DAG rules; higher-level orchestrator must translate plans to `WorkflowSpec`).
- **Direct dependency on `tool-registry` / `model-runtime`**: REJECTED (violates microkernel encapsulation).
- **Tokio async runtime**: REJECTED (violates microkernel lightweight runtime directive).
- **Unbounded workflow step execution**: REJECTED (causes resource exhaustion and supervisor deadlocks).

---

## 23. Compliance Matrix
- **`DEPENDENCY_MAP.md`**: PASS (`apps → automation → runtime, services`).
- **`FIRST_ALPHA_SPEC.md`**: PASS (Week 8 milestone, Item 8 automation workflow).
- **`PROJECT_CHARTER.md`**: PASS (Zero-Trust CBAC token validation).
- **`PACKAGE_RULES.md`**: PASS (Standard module layout, safe Rust).

---

## 24. Decision

ADR STATUS:
**PROPOSED — PENDING REVIEW**

IMPLEMENTATION PERMITTED:
**NO**

REQUIRED NEXT STEP:
**FORMAL ADR REVIEW**
