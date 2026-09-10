# ADR-001: NAINA OS Kernel / NKRS Process Supervisor & Lifecycle Architecture

- **Title:** ADR-001: NAINA OS Kernel / NKRS Process Supervisor & Lifecycle Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-21
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/kernel`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

---

## 2. Context
NAINA OS is built on a microkernel architecture (`PROJECT_CHARTER.md` Section 3). Lower-level packages (`configuration`, `logging`, `event-bus`, `capabilities`) supply core primitives. `kernel` is the central supervisor overseeing process registration, cold-boot startup (`< 2.0s`), process health monitoring, and system lifecycle transitions.

Having completed `configuration`, `logging`, `event-bus`, and `capabilities`, `kernel` is the final prerequisite package before `runtime` can be implemented (`DEPENDENCY_MAP.md`).

---

## 3. Existing Documented Requirements (Source of Truth)

### A. Explicit Documented Requirements:
- **Dependency Matrix (`DEPENDENCY_MAP.md`)**: `kernel` depends on `configuration`, `logging`, `event-bus`, and `capabilities`. It is consumed by `runtime`.
- **Boot Budget (`FIRST_ALPHA_SPEC.md` Section 3)**: Microkernel NKRS Startup overhead `< 2.0s` (`test_cold_boot.py`).
- **Crash Recovery (`FIRST_ALPHA_SPEC.md` Checklist #10)**: *"Graceful Crash Recovery: Automatically restarts failed worker processes without crashing the NKRS supervisor."*
- **Microkernel Isolation (`PROJECT_CHARTER.md` Section 3)**: Keep kernel logic lightweight, pushing subsystem logic into isolated user-space processes (NKRS).

### B. Requirements NOT Specified (Ambiguities resolved by this ADR):
- Exact Rust struct definitions for `Kernel` and process supervision types.
- Process state machine variants and lifecycle method signatures.
- Alpha vs. Beta process isolation boundaries (logical in-memory supervision vs C++ PREEMPT_RT process isolation).

---

## 4. Problem Statement
The primary engineering documents mandate a `kernel` package exposing `Kernel`, but omit method signatures, lifecycle state definitions, and supervisor restart policies. An ADR is required to establish the minimal safe Alpha contract without adding heavy async runtimes (Tokio), C++ CMake integration, or circular dependencies.

---

## 5. Architectural Decisions (ADR-001)

1. **In-Memory Logical Supervisor for Alpha**: `Kernel` provides thread-safe logical process supervision and lifecycle state management within the Rust monorepo process space.
2. **Explicit System Lifecycle State Machine**: Lifecycle transitions strictly follow: `Uninitialized` -> `Booting` -> `Running` -> `ShuttingDown` -> `Stopped` / `Failed`.
3. **Subsystem Process Supervision**: Monitored subsystem processes are represented by `ProcessRecord`s with explicit `ProcessState` and configurable `RestartPolicy`.
4. **Synchronous In-Memory Execution**: Boot and supervision logic execute synchronously using standard library synchronization (`std::sync::RwLock`, `std::sync::atomic::AtomicU64`), targeting the `< 2.0s` cold boot budget.
5. **Decoupled System Event Notifications**: `Kernel` publishes lifecycle events (`"system.booting"`, `"system.started"`, `"system.shutdown"`, `"process.failed"`) via `EventBus`.
6. **Zero-Trust Capability Enforcement**: Sensitive kernel operations require a valid `CapabilityToken` (e.g. `CAP_KERNEL_SUPERVISE`) checked against `CapabilityRegistry`.

---

## 6. Kernel Lifecycle State Machine

```
  [Uninitialized]
         │
      boot()
         ▼
     [Booting] ──────(boot failure)──────> [Failed]
         │
      start()
         ▼
     [Running]
         │
    shutdown()
         ▼
  [ShuttingDown]
         │
         ▼
     [Stopped]
```

---

## 7. Process Model & Process State Machine

Each monitored process has a unique `ProcessId(pub u64)` and `ProcessState`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessState {
    Registered,
    Starting,
    Running,
    Stopped,
    Failed { error: String },
}
```

---

## 8. Process Supervision & Restart Policy

To satisfy Alpha Acceptance Criterion #10 (Automatic Crash Recovery), processes configure a `RestartPolicy`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    OnFailure { max_retries: u32 },
}
```

If a process transitions to `ProcessState::Failed` during health checks or process operations:
- `RestartPolicy::OnFailure` attempts restart up to `max_retries`.
- `RestartPolicy::Never` marks the process as `Failed` and emits a `"process.failed"` event.

---

## 9. EventBus Integration
During lifecycle transitions and process failure events, `Kernel` publishes events to `EventBus`:
- `"system.booting"`
- `"system.started"`
- `"system.shutdown"`
- `"process.failed"`

---

## 10. CapabilityRegistry Integration
Kernel operations (`register_process`, `stop_process`) verify presented `CapabilityToken`s via `CapabilityRegistry::authorize()`. Required capability: `CAP_KERNEL_SUPERVISE`.

---

## 11. Error Model

```rust
#[derive(Debug)]
pub enum KernelError {
    BootFailed { message: String },
    ProcessNotFound { id: ProcessId },
    ProcessFailed { id: ProcessId, message: String },
    Unauthorized { capability_id: String },
    InvalidState { current: String, expected: String },
    LockError { message: String },
    Capability(capabilities::CapabilityError),
    EventBus(event_bus::EventBusError),
}

pub type Result<T> = std::result::Result<T, KernelError>;
```

---

## 12. Configuration Model

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelConfig {
    pub boot_timeout: std::time::Duration,
    pub max_process_restarts: u32,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            boot_timeout: std::time::Duration::from_secs(2),
            max_process_restarts: 3,
        }
    }
}
```

---

## 13. Logging & Security Audit Model
- All state transitions (`boot`, `start`, `shutdown`, `restart`) are logged via `logging::Logger`.
- Secret keys, credentials, and token payload details are NEVER logged.

---

## 14. Concurrency Model
- Internal state synchronized using `std::sync::RwLock` and `AtomicU64`.
- `Kernel` is `Send + Sync` and can be wrapped in `std::sync::Arc<Kernel>`.

---

## 15. Dependency Constraints
- **Allowed**: `configuration`, `logging`, `event-bus`, `capabilities`.
- **Forbidden**: `runtime`, `services`, `orchestrator`, `tool-registry`, `memory`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`.

---

## 16. Alpha Scope
1. In-memory `Kernel` struct and process registry.
2. `boot()`, `start()`, `shutdown()`, `register_process()`, `stop_process()`, `check_health()` methods.
3. Integration with `EventBus` and `CapabilityRegistry`.
4. Unit tests covering cold boot, process registration, crash recovery restart policies, and shutdown.

---

## 17. Deferred Capabilities (Beta / Production)
- ❌ C++ PREEMPT_RT real-time kernel scheduling.
- ❌ OS-level process isolation and PID namespace isolation.
- ❌ WASM plugin container sandboxing.

---

## 18. Consequences
- Microkernel DAG strictly preserved.
- Zero external crate dependencies added.
- Guaranteed cold boot overhead `< 2.0s`.

---

## IMPLEMENTATION CONTRACT

Exact public structs:
- `pub struct Kernel`
- `pub struct KernelConfig`
- `pub struct ProcessId(pub u64)`
- `pub struct ProcessRecord`

Exact public enums:
- `pub enum KernelState`
- `pub enum ProcessState`
- `pub enum RestartPolicy`
- `pub enum KernelError`

Exact public traits:
- None required for Alpha.

Exact methods:
- `impl Kernel`:
  - `pub fn new(config: KernelConfig) -> Self`
  - `pub fn boot(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()>`
  - `pub fn start(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()>`
  - `pub fn shutdown(&self, event_bus: Option<&event_bus::EventBus>) -> Result<()>`
  - `pub fn register_process(&self, name: impl Into<String>, policy: RestartPolicy, capability_registry: Option<(&capabilities::CapabilityRegistry, &capabilities::CapabilityToken)>) -> Result<ProcessId>`
  - `pub fn stop_process(&self, id: ProcessId) -> Result<bool>`
  - `pub fn state(&self) -> KernelState`
  - `pub fn get_process_state(&self, id: ProcessId) -> Result<ProcessState>`

Lifecycle state machine:
- `Uninitialized` -> `Booting` -> `Running` -> `ShuttingDown` -> `Stopped` / `Failed`

Process state machine:
- `Registered` -> `Starting` -> `Running` -> `Stopped` / `Failed`

Boot sequence:
1. Check state is `Uninitialized`.
2. Transition state to `Booting`.
3. Emit `"system.booting"` via `EventBus`.
4. Validate config (`boot_timeout <= 2.0s`).
5. Transition state to `Booting` complete.

Shutdown sequence:
1. Transition state to `ShuttingDown`.
2. Emit `"system.shutdown"` via `EventBus`.
3. Set all process states to `Stopped`.
4. Transition state to `Stopped`.

Failure semantics:
1. Boot failure sets `KernelState::Failed`.
2. Process failure updates `ProcessState::Failed { error }` and triggers `RestartPolicy`.

Restart semantics:
1. `RestartPolicy::OnFailure`: retries up to `max_retries`, logs restart attempt, and resets state to `Running`.
2. If retries exceeded, sets state to `Failed` and emits `"process.failed"` event.

EventBus integration:
- Emits `"system.booting"`, `"system.started"`, `"system.shutdown"`, and `"process.failed"` events.

CapabilityRegistry integration:
- Authorizes `CAP_KERNEL_SUPERVISE` when `CapabilityRegistry` and token are supplied during process registration.

Logging requirements:
- Logs all state transitions and process failures via `logging::Logger`.

Concurrency model:
- `std::sync::RwLock` for internal process registry and state access. `Send + Sync`.

Configuration:
- `KernelConfig { boot_timeout, max_process_restarts }`.

Dependencies:
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`
- `event-bus = { path = "../event-bus" }`
- `capabilities = { path = "../capabilities" }`

Files to implement:
- `packages/kernel/src/types.rs`
- `packages/kernel/src/error.rs`
- `packages/kernel/src/config.rs`
- `packages/kernel/src/kernel.rs`
- `packages/kernel/src/lib.rs`
- `packages/kernel/tests/supervisor.rs`

Alpha scope:
- In-memory process supervisor, lifecycle state transitions, crash recovery restart policies, EventBus and CapabilityRegistry integration.

Deferred features:
- C++ PREEMPT_RT real-time scheduling, OS-level container isolation, WASM sandboxing.

Open architectural questions:
1. Dynamic worker thread pooling for asynchronous process monitoring in Beta.

Implementation permitted:
NO
