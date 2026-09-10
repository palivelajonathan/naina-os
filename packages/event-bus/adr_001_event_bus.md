# ADR-001: NAINA OS EventBus Architecture

- **Title:** ADR-001: NAINA OS EventBus Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-21
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/event-bus`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

---

## 2. Context
NAINA OS uses a microkernel architecture where foundational packages (`configuration`, `logging`, `event-bus`, `capabilities`) supply core primitives to upper-level runtimes (`kernel`, `runtime`, `services`, `orchestrator`).

Following `configuration` and `logging`, the next package in `engineering/DEPENDENCY_MAP.md` is `event-bus`.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. Requirements Explicitly Supported by Existing Documents:
- **Dependency Hierarchy (`DEPENDENCY_MAP.md`)**: `event-bus` depends exclusively on `logging` and exposes `EventBus` as its public API. It is consumed by `runtime` and `orchestrator` (and `kernel`).
- **Package Layout (`PACKAGE_RULES.md`)**: Follows standard layout (`src/lib.rs`, `src/error.rs`, `src/types.rs`, `src/traits.rs`, `src/config.rs`) and maintains a strict Directed Acyclic Graph (DAG).
- **Performance Budget (`FIRST_ALPHA_SPEC.md`)**: Subsystem boot latency `< 2.0s`, idle RAM `< 1.0 GB`, background CPU `< 5.0%`. Avoid unvetted third-party crates.

### B. Requirements NOT Specified by Existing Documents (Ambiguities resolved by this ADR):
- Event struct schema and payload model
- Publish / Subscribe method signatures
- Handler execution semantics and error propagation behavior
- Concurrency and lock synchronization primitives

---

## 4. Problem Statement
The primary engineering documents mandate an `event-bus` package exposing `EventBus`, but omit exact API method signatures and event schemas. An ADR is required to establish explicit implementation rules without introducing premature async runtimes (Tokio), external message queues (NATS/ZMQ), circular dependencies, or unvetted external dependencies.

---

## 5. Architectural Decisions (ADR-001)

1. **Synchronous & In-Memory Execution**: `EventBus` is strictly in-memory and synchronous for Alpha.
2. **Deterministic Handler Order**: Event handlers execute strictly in subscription registration order.
3. **Short-Circuit Error Propagation**: If a handler returns an error during `publish()`:
   - Dispatch stops immediately.
   - Later handlers are NOT invoked.
   - The first handler error is returned directly from `publish()`.
4. **Decoupled Logging Error Path**: `EventBus` retains `logging` as an allowed workspace dependency, but `EventBusError` operates independently without requiring `LogError` on normal execution/error paths.
5. **Minimal Configuration**: `EventBusConfig` is a simple default configuration struct without un-specified fields like `max_subscribers_per_topic`.
6. **Thread Safety & Ownership**: `EventBus` implements `Send + Sync` via internal synchronization (`std::sync::RwLock`). Callers may wrap `EventBus` in `std::sync::Arc<EventBus>` for multi-threaded usage, but `Arc` is not an internal field of `EventBus`.

---

## 6. Event Model
Concrete structured `Event` struct (matching string payload patterns in `LogRecord`):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    id: String,
    event_type: String,
    source: String,
    timestamp: std::time::SystemTime,
    payload: std::collections::BTreeMap<String, String>,
}
```

---

## 7. EventBus API & Subscription Model

```rust
pub struct EventBus {
    config: EventBusConfig,
    subscribers: std::sync::RwLock<std::collections::BTreeMap<String, Vec<Subscription>>>,
    next_id: std::sync::atomic::AtomicU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(pub u64);
```

Public methods:
- `pub fn new(config: EventBusConfig) -> Self`
- `pub fn publish(&self, event: &Event) -> Result<()>`
- `pub fn subscribe(&self, event_type: impl Into<String>, handler: Box<dyn EventHandler>) -> Result<SubscriptionId>`
- `pub fn unsubscribe(&self, id: SubscriptionId) -> Result<bool>`

---

## 8. Handler Model
Thread-safe `EventHandler` trait:

```rust
pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &Event) -> Result<()>;
}
```

---

## 9. Error Model
Standalone `EventBusError`:

```rust
#[derive(Debug)]
pub enum EventBusError {
    Handler { subscription_id: SubscriptionId, message: String },
    LockError { message: String },
}

pub type Result<T> = std::result::Result<T, EventBusError>;
```

---

## 10. Configuration Model
Minimal default configuration struct:

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventBusConfig;
```

---

## 11. Deferred Capabilities (Post-Alpha / Beta)
- ❌ **Tokio & Async Traits**: Deferred until async scheduling is required by `runtime`.
- ❌ **Channels & Network Transport (NATS/ZMQ)**: Deferred to Beta (`NOS-API-001`).
- ❌ **Event Persistence & Replay**: Deferred to Beta.
- ❌ **Dynamic `Any` Payloads**: Deferred; structured key-value strings used for type safety.

---

## 12. Consequences
- Microkernel DAG strictly preserved.
- Zero external crate dependencies added.
- Deterministic, short-circuiting error handling guaranteed.
