# ADR-001: NAINA OS Client SDK Facade Architecture

- **Title:** ADR-001: NAINA OS Client SDK Facade Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-24
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/sdk`

---

## 1. Context
NAINA OS operates on a microkernel architecture. The `packages/sdk` package serves as the primary client-facing facade (`SDKFacade`) for applications (`apps/`) and developer tooling:
```
apps → sdk → (runtime, services) → (kernel, capabilities, configuration, logging)
```

The SDK provides high-level client bindings for microkernel initialization, session management, and operating system capability routing without duplicating underlying engine logic or violating microkernel encapsulation boundaries.

---

## 2. Source-Supported Requirements vs. ADR Decisions
- **Source-Supported Requirements**:
  - `FIRST_ALPHA_SPEC.md` Section 2: MVN Core Pipeline end-to-end cognitive loop binding.
  - `FIRST_ALPHA_SPEC.md` Section 3: Microkernel Cold Boot target `< 2.0 s`.
  - `FIRST_ALPHA_SPEC.md` Section 6: Week 9 Milestone (End-to-End MVN Integration Testing).
  - `DEPENDENCY_MAP.md`: `sdk` depends directly on `runtime` and `services`. All 14 higher-level and peer crates are explicitly forbidden as direct dependencies.
  - `PROJECT_CHARTER.md`: Zero-Trust CBAC token validation; zero plain-text logging of credentials/keys.
  - `PACKAGE_RULES.md`: No Tokio or external async runtimes in microkernel workspace.
- **ADR Decisions**:
  - `SDKFacade` struct layout and reference ownership model.
  - `SDKConfig` fields and defaults.
  - `SDKBuilder` initialization pipeline.
  - `SDKSession` handle tracking and context linkage.
  - `SDKResult<T>` and `SDKError` taxonomy.
  - Service-mediated action routing architecture.
  - Thread synchronization hierarchy (`sessions` → `logger`).

---

## 3. DAG / Dependency Boundary
- **Allowed Direct Dependencies**:
  - `runtime = { path = "../runtime" }`
  - `services = { path = "../services" }`
  - `configuration = { path = "../configuration" }`
  - `logging = { path = "../logging" }`

- **Explicit Forbidden Dependencies**:
  `kernel`, `capabilities`, `event-bus`, `orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `voice-runtime`, `desktop-runtime`, `browser-runtime`, `automation`, `ui`.

- `SDKFacade` MUST NOT directly import any forbidden high-level engine package.

---

## 4. SDKFacade Architecture & Ownership
```rust
pub struct SDKFacade {
    config: SDKConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    sessions: RwLock<HashMap<String, SDKSession>>,
    logger: Mutex<Logger>,
}
```
- Ownership: `SDKFacade` holds shared `Arc` references to `Runtime` and `ServiceRegistry`.
- `SDKFacade` is a client routing facade ONLY. It MUST NOT become a second orchestrator.

---

## 5. SDKConfig
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SDKConfig {
    pub system_name: String,        // default: "NAINA OS" (ADR DECISION)
    pub enable_logging: bool,       // default: true (ADR DECISION)
    pub auto_start_runtime: bool,  // default: true (ADR DECISION)
    pub session_timeout_ms: u64,    // default: 3,600,000 ms / 1 hr (ADR DECISION)
}
```

---

## 6. SDKBuilder
```rust
#[derive(Debug, Default)]
pub struct SDKBuilder {
    config: Option<SDKConfig>,
}

impl SDKBuilder {
    pub fn new() -> Self;
    pub fn with_config(mut self, config: SDKConfig) -> Self;
    pub fn build(self) -> Result<SDKFacade, SDKError>;
}
```
- Construction sequence:
  1. `Logger` initialization via `logging::LoggerConfig`.
  2. `Config` loading via `configuration::ConfigLoader`.
  3. `Runtime` instantiation via `runtime::RuntimeConfig`.
  4. `ServiceRegistry` instantiation via `services::ServicesConfig`.
  5. `SDKFacade` assembly.

---

## 7. SDKSession
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SDKSession {
    pub session_id: String,
    pub context_id: runtime::ExecutionContextId,
    pub created_at_ms: u64,
    pub is_active: bool,
}
```
- Session lifecycle managed via `SDKFacade::create_session()` and `SDKFacade::close_session()`.
- Thread-safe tracking in `HashMap` protected by `RwLock`.

---

## 8. SDKResult
```rust
pub type SDKResult<T> = std::result::Result<T, SDKError>;
```
- Standardized result type wrapping client operations.
- Encapsulates microkernel errors without leaking internal implementation details across the SDK boundary.

---

## 9. SDKError Taxonomy
```rust
#[derive(Debug)]
pub enum SDKError {
    InitializationFailed { message: String },
    SessionNotFound { session_id: String },
    SessionExpired { session_id: String },
    CapabilityDenied { message: String },
    Runtime(runtime::RuntimeError),
    Service(services::ServicesError),
    Configuration(configuration::ConfigError),
    LockError { message: String },
}
```
- Automatic `From` implementations for `RuntimeError`, `ServicesError`, `ConfigError`.

---

## 10. Service-Mediated Capability Boundary
- **CRITICAL BOUNDARY LOCK**:
  - `SDKFacade` MUST NOT directly depend on `orchestrator`, `voice-runtime`, `desktop-runtime`, `browser-runtime`, `automation`, `model-runtime`, or `model-providers`.
  - All high-level capability operations exposed to clients (e.g. voice turns, desktop launching, URL navigation, workflow automation) MUST cross the SDK boundary via **service-mediated routing**.
  - `SDKFacade` resolves registered service handles via `ServiceRegistry::lookup()` or dispatches capability-authorized contexts via `Runtime::execute_in_context()`.
  - `SDKFacade` is a client-facing routing layer; it is NOT an orchestrator, model runtime, voice runtime, desktop runtime, browser runtime, or automation engine.
  - The exact service-mediated payload serialization format across dynamic service boundaries is explicitly marked `UNSPECIFIED / DEFERRED`.

---

## 11. CBAC Delegation & Security
- `SDKFacade` delegates capability token verification to `Runtime` context calls and `ServiceRegistry` lookups.
- Zero capability bypass. Unauthorized operations surface as `SDKError::CapabilityDenied`.
- Logging restriction: Zero plain-text logging of passwords, capability tokens, credentials, encryption keys, or secret session payloads.

---

## 12. Concurrency Model
- `SDKFacade` implements `Send + Sync`. Standard usage is wrapped in `Arc<SDKFacade>`.
- Thread-safe synchronization via `std::sync::RwLock` (for sessions) and `Mutex` (for logger).
- Lock acquisition order: `sessions` → `logger` (prevents deadlocks).
- Core SDK wrapper contains zero unsafe code.
- **NO TOKIO OR UNAPPROVED ASYNC RUNTIMES**. Uses `std::thread` and `std::sync::mpsc`.

---

## 13. Lifecycle Model
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SDKState {
    Uninitialized,
    Initializing,
    Ready,
    Shutdown,
}
```
- Transitions: `Uninitialized` → `Initializing` → `Ready` → `Shutdown`.
- Repeated initialization returns `SDKError::InitializationFailed`.

---

## 14. Performance Targets
- **System Cold Boot**: `< 2.0 s` (`FIRST_ALPHA_SPEC.md` Section 3). Measures time from `SDKBuilder::build()` to microkernel `Ready` state.

---

## 15. Memory Budget Targets
- SDK handle allocation overhead target: `< 50 MB`.
- System idle total RAM target: `< 1.0 GB` (`FIRST_ALPHA_SPEC.md` Section 4).
- *Classification: ENGINEERING TARGET / ASSUMPTION.*

---

## 16. Logging & Privacy Rules
- Diagnostic operational logging permitted.
- Plaintext logging of credentials, keys, tokens, or secret session payloads strictly prohibited.

---

## 17. Alpha Scope vs. Deferred Features
- **INCLUDED IN ALPHA (v0.8.0)**:
  - `SDKFacade` initialization and `SDKBuilder`.
  - `SDKSession` creation and context linkage.
  - `Runtime` and `ServiceRegistry` binding.
  - Service-mediated action dispatch.
  - `SDKResult` and `SDKError` boundary wrapping.
  - CBAC capability delegation.
  - Lifecycle management.
- **DEFERRED**:
  - Multi-tenant cloud session synchronization.
  - Cross-language C/Python FFI export bindings.
  - Remote P2P RPC SDK transport protocol.
  - Exact service-mediated payload serialization format (marked `UNSPECIFIED / DEFERRED`).

---

## 18. Architecture Risks & Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **SDK → Engine Circular Dependencies** | Direct imports of high-level engine crates | Strict DAG rule limiting direct dependencies to `runtime` & `services` |
| **Monolithic Second Orchestrator** | Duplicating CARF task routing in SDK | Restrict SDK to facade routing via `ServiceRegistry` |
| **Tokio Contamination** | Unapproved async runtimes | Enforce standard library `std::thread` + `std::sync::mpsc` |
| **Capability Bypass** | Dynamic client requests bypassing tokens | Delegate CBAC token check to `Runtime`/`ServiceRegistry` |

---

## 19. Compliance Matrix

| Requirement | Source / Standard | Status |
| :--- | :--- | :---: |
| **DAG Compliance** | `engineering/DEPENDENCY_MAP.md` | **PASS** |
| **Allowed Dependencies ONLY** | `runtime`, `services`, `configuration`, `logging` | **PASS** |
| **Forbidden List Excluded** | 14 engine & peer crates | **PASS** |
| **SDKFacade Architecture** | Client facade wrapper | **PASS** |
| **SDKConfig & SDKBuilder** | Builder pattern initialization | **PASS** |
| **SDKSession & Context Linkage** | `ExecutionContextId` linkage | **PASS** |
| **SDKResult & SDKError** | Clean boundary wrapping | **PASS** |
| **Service-Mediated Architecture** | Dynamic routing via `ServiceRegistry` | **PASS** |
| **CBAC Security Delegation** | Delegated to `Runtime`/`Services` | **PASS** |
| **Concurrency & Thread Safety** | `Send + Sync`, zero unsafe code | **PASS** |
| **Microkernel Async Restrictions** | No Tokio; `std::thread` worker channels | **PASS** |
| **Cold Boot Target (<2.0s)** | `FIRST_ALPHA_SPEC.md` Section 3 | **PASS** |
| **Memory Target (<50MB)** | Engineering Target / Assumption | **PASS** |
| **Logging & Privacy** | Zero plaintext secret logging | **PASS** |
| **Alpha Scope Boundaries** | Excludes cloud sync / WASM / FFI | **PASS** |

---

## 20. ADR Status

SDK ADR-001 STATUS:
**PROPOSED — PENDING REVIEW**

IMPLEMENTATION PERMITTED:
**NO**

REQUIRED NEXT STEP:
**FORMAL ADR REVIEW**

SDK ADR-001 DRAFT COMPLETE
