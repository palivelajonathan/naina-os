# ADR-001: NAINA OS Service Registry & Discovery Architecture

- **Title:** ADR-001: NAINA OS Service Registry & Discovery Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-21
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/services`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

*Implementation Permitted: NO*

---

## 2. Context
NAINA OS operates on a microkernel architecture (`PROJECT_CHARTER.md` Section 3). Lower-level packages (`configuration`, `logging`, `event-bus`, `capabilities`, `kernel`, `runtime`) are fully implemented and verified. `services` is the Layer 3 package providing the user-space service registration, discovery, health tracking, and capability-controlled service access framework (`DEPENDENCY_MAP.md`).

Having completed `runtime`, `services` is the next package in the dependency DAG.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **Dependency Matrix (`DEPENDENCY_MAP.md`)**: `services` depends on `runtime` and `capabilities`. Used by `orchestrator`. Public API: `ServiceRegistry`.
- **Microkernel Isolation (`PROJECT_CHARTER.md` Section 3)**: Keep kernel lightweight; run user-space services in isolated processes managed by `kernel` and `runtime`.
- **Cold Boot Budget (`FIRST_ALPHA_SPEC.md` Section 3 & Criterion #7)**: System cold boot overhead `< 2.0s`; Microkernel NKRS and Service Framework must register and discover services during cold boot in `< 2.0s`.
- **Zero-Trust Security (`PROJECT_CHARTER.md` Section 2)**: All sensitive operations must authorize capability tokens via `CapabilityRegistry`.

### B. ADR DECISIONS (Introduced by this ADR):
- Struct definitions for `ServiceRegistry`, `ServicesConfig`, `ServiceId(pub u64)`, `ServiceName(pub String)`, `ServiceRecord`.
- Enum variants for `ServiceState` (`Registered`, `Starting`, `Active`, `Degraded`, `Stopped`, `Failed`).
- Enum variants for `ServiceHealth` (`Healthy`, `Degraded { reason: String }`, `Unhealthy { reason: String }`, `Unknown`).
- Capability identifiers `CAP_SERVICE_REGISTER` (for registration/unregistration) and `CAP_SERVICE_LOOKUP` (for discovery/lookup).
- Event Bus topic strings (`"services.registered"`, `"services.active"`, `"services.unregistered"`, `"services.health_changed"`).
- Strongly typed `ServicesError` enum and `Result<T>` alias.

---

## 4. Problem Statement
The primary engineering documents state that `services` provides service registration and discovery via `ServiceRegistry`, but do not define the concrete struct fields, discovery lookups, health tracking semantics, or capability authorization boundaries. An ADR is required to establish a minimal, safe, synchronous, in-memory Alpha contract.

---

## 5. Architectural Decisions (ADR-001)

1. **Layer 3 Service Framework**: `ServiceRegistry` manages user-space services, referencing `runtime::Runtime` for context execution while delegating process supervision to `Kernel` and authorization to `Capabilities`.
2. **In-Memory Thread-Safe Service Registry**: Service metadata records are tracked in `std::sync::RwLock<BTreeMap<ServiceId, ServiceRecord>>` with name-to-ID indices in `RwLock<BTreeMap<String, ServiceId>>`.
3. **Explicit Service Lifecycle State Machine**: Services transition through `Registered` -> `Starting` -> `Active` -> `Degraded` / `Stopped` / `Failed`.
4. **Synchronous Health Tracking**: Health status is represented via `ServiceHealth` (`Healthy`, `Degraded`, `Unhealthy`, `Unknown`) updated synchronously via `update_health`. Background polling threads are deferred to Beta.
5. **Capability Security Boundaries**:
   - `register` and `unregister` require `CAP_SERVICE_REGISTER` authorization via `CapabilityRegistry`.
   - `lookup` requires `CAP_SERVICE_LOOKUP` authorization via `CapabilityRegistry`.
6. **EventBus Notifications**: Service registration, state transitions, and health changes emit events (`"services.registered"`, `"services.active"`, `"services.unregistered"`, `"services.health_changed"`).

---

## 6. Service Lifecycle & Health Model

```
  [Registered]
       │
   start()
       ▼
   [Starting]
       │
       ▼
    [Active] <──── (health update) ────> [Degraded]
       │
  stop()/unregister()
       ▼
   [Stopped] / [Failed]
```

Health Status:
- `Healthy`
- `Degraded { reason: String }`
- `Unhealthy { reason: String }`
- `Unknown`

---

## 7. Service Record Model

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceName(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceState {
    Registered,
    Starting,
    Active,
    Degraded,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceHealth {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceRecord {
    pub id: ServiceId,
    pub name: ServiceName,
    pub state: ServiceState,
    pub health: ServiceHealth,
    pub execution_context_id: Option<runtime::ExecutionContextId>,
}
```

---

## 8. Service Registry API

```rust
pub struct ServiceRegistry {
    config: ServicesConfig,
    runtime: std::sync::Arc<runtime::Runtime>,
    services: std::sync::RwLock<std::collections::BTreeMap<ServiceId, ServiceRecord>>,
    by_name: std::sync::RwLock<std::collections::BTreeMap<String, ServiceId>>,
    next_service_id: std::sync::atomic::AtomicU64,
}
```

---

## 9. Capability Security
- `register` & `unregister`: Authorize `CAP_SERVICE_REGISTER` against `CapabilityRegistry` when token provided.
- `lookup`: Authorize `CAP_SERVICE_LOOKUP` against `CapabilityRegistry` when token provided.
- Preserves deny-by-default security.

---

## 10. Concurrency Model
- Internal state synchronized via `std::sync::RwLock` and `AtomicU64`.
- `ServiceRegistry` is `Send + Sync` and shareable across threads via `std::sync::Arc<ServiceRegistry>`.

---

## 11. Performance Requirements
- Cold boot contribution: `< 50ms` (within total system cold boot budget `< 2.0s`).
- Service lookup latency: `< 1ms` in-memory lookup.

---

## 12. Alpha Scope
1. In-memory `ServiceRegistry` wrapping `Arc<Runtime>`.
2. `register()`, `lookup()`, `get_service()`, `unregister()`, `health_check()`, `update_health()`, `update_state()` methods.
3. Integration with `CAP_SERVICE_REGISTER` and `CAP_SERVICE_LOOKUP` capability checks.
4. Unit tests covering service registration, lookup, state changes, health updates, capability authorization, and thread safety.

---

## 13. Deferred Capabilities (Beta / Production)
- ❌ Tokio async background health polling workers.
- ❌ Remote gRPC/REST network service discovery.
- ❌ NATS / ZMQ service messaging backplanes.
- ❌ Dynamic WASM service sandboxes.
- ❌ Dynamic OS process cgroups / PID namespace isolation.

---

## 14. Alternatives Considered
1. **Background Polling Thread for Health Checks**: Rejected for Alpha to keep cold boot overhead strictly `< 2.0s` and avoid unvetted async runtimes.
2. **Duplicating Process Supervision in ServiceRegistry**: Rejected to preserve microkernel single-responsibility (`kernel` supervises processes; `runtime` handles execution contexts; `services` handles service registry and discovery).

---

## 15. Risks & Mitigation
- *Orphaned Services*: `ServiceRecord` left `Active` when underlying context closes. *Mitigation*: Sync with `runtime.get_context_state()` during health check.

---

## 16. Consequences
- Clean microkernel DAG maintained (`runtime` + `capabilities` -> `services`).
- Zero external crate dependencies added.

---

## IMPLEMENTATION CONTRACT

Public structs:
- `pub struct ServiceRegistry`
- `pub struct ServicesConfig`
- `pub struct ServiceId(pub u64)`
- `pub struct ServiceName(pub String)`
- `pub struct ServiceRecord`

Public enums:
- `pub enum ServiceState`
- `pub enum ServiceHealth`
- `pub enum ServicesError`

Public type aliases:
- `pub type Result<T> = std::result::Result<T, ServicesError>`

Exact method signatures:
- `impl ServiceRegistry`:
  - `pub fn new(config: ServicesConfig, runtime: std::sync::Arc<runtime::Runtime>) -> Self`
  - `pub fn register(&self, name: impl Into<String>, execution_context_id: Option<runtime::ExecutionContextId>, capability_registry: Option<(&kernel::capabilities::CapabilityRegistry, &kernel::capabilities::CapabilityToken)>) -> Result<ServiceId>`
  - `pub fn lookup(&self, name: &str, capability_registry: Option<(&kernel::capabilities::CapabilityRegistry, &kernel::capabilities::CapabilityToken)>) -> Result<ServiceRecord>`
  - `pub fn get_service(&self, id: ServiceId) -> Result<ServiceRecord>`
  - `pub fn unregister(&self, id: ServiceId, capability_registry: Option<(&kernel::capabilities::CapabilityRegistry, &kernel::capabilities::CapabilityToken)>) -> Result<()>`
  - `pub fn health_check(&self, id: ServiceId) -> Result<ServiceHealth>`
  - `pub fn update_health(&self, id: ServiceId, health: ServiceHealth) -> Result<()>`
  - `pub fn update_state(&self, id: ServiceId, state: ServiceState) -> Result<()>`

Lifecycle transitions:
- `Registered` -> `Starting` -> `Active` -> `Degraded` / `Stopped` / `Failed`

Capability requirements:
- Authorizes `CAP_SERVICE_REGISTER` for registration/unregistration.
- Authorizes `CAP_SERVICE_LOOKUP` for service lookup.

Event types:
- `"services.registered"`, `"services.active"`, `"services.unregistered"`, `"services.health_changed"`

Dependencies:
- `runtime = { path = "../runtime" }`
- `capabilities = { path = "../capabilities" }` (or via `kernel::capabilities`)

Forbidden dependencies:
- `orchestrator`, `tool-registry`, `memory`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`

Deferred features:
- Tokio async background health polling, remote network service discovery, NATS/ZMQ messaging, WASM service sandboxing.

Implementation permitted:
NO
