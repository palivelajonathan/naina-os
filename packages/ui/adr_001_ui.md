# ADR-001: NAINA OS User Interface Framework Architecture

- **Title:** ADR-001: NAINA OS User Interface Framework Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-24
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/ui`

---

## 1. Context & Purpose
NAINA OS operates on a microkernel architecture. The `packages/ui` package provides the client-side user interface framework (`UIFramework`), desktop overlay host abstractions, and event handling bindings for desktop applications (`apps/desktop`):
```
apps → ui → runtime → (kernel, capabilities, configuration, logging)
```

The UI framework abstracts window lifecycle management and event dispatch while integrating with microkernel execution contexts without duplicating engine logic or introducing unapproved async dependencies.

---

## 2. Source-Supported Requirements vs. ADR Decisions
- **Source-Supported Requirements**:
  - `FIRST_ALPHA_SPEC.md` Section 4: Desktop Overlay Host RAM budget `< 200 MB`.
  - `FIRST_ALPHA_SPEC.md` Section 4: System Idle CPU consumption `< 5.0 %`.
  - `FIRST_ALPHA_SPEC.md` Section 6: Week 6 Milestone (Windows Desktop Overlay Host & System Control API).
  - `DEPENDENCY_MAP.md`: `ui` depends directly on `runtime`. All 14 higher-level and peer crates are explicitly forbidden.
  - `PROJECT_CHARTER.md`: Zero-Trust CBAC token validation; zero plain-text logging of credentials/keys/input text.
  - `PACKAGE_RULES.md`: No Tokio or external async runtimes in microkernel workspace.
- **ADR Decisions**:
  - `UIFramework` struct layout and reference ownership model.
  - `UIConfig` fields and default values.
  - `UIBuilder` initialization sequence.
  - `UIState` lifecycle transitions.
  - `UIEvent` taxonomy and `std::sync::mpsc` channel transport.
  - `UIError` taxonomy and `UIResult<T>` alias.
  - Synchronization hierarchy (`state` → `windows` → `logger`).
- **Unspecified / Deferred**:
  - Concrete physical GUI rendering backend (egui, Slint, Webview, Winit, Win32).

---

## 3. DAG / Dependency Boundary
- **Allowed Direct Dependencies**:
  - `runtime = { path = "../runtime" }`
  - `configuration = { path = "../configuration" }`
  - `logging = { path = "../logging" }`

- **Explicit Forbidden Dependencies**:
  `kernel`, `capabilities`, `event-bus`, `orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `voice-runtime`, `desktop-runtime`, `browser-runtime`, `automation`, `sdk`.

- `UIFramework` MUST NOT directly import any forbidden engine or peer crate.

---

## 4. UIFramework Struct Layout & Ownership
```rust
pub struct UIFramework {
    config: UIConfig,
    runtime: Arc<Runtime>,
    logger: Mutex<Logger>,
    state: RwLock<UIState>,
    windows: RwLock<HashMap<String, WindowRecord>>,
}
```
- Ownership: `UIFramework` holds a shared `Arc` reference to microkernel `Runtime` and owns its configuration, state machine, and window handles.
- `UIFramework` acts as an overlay state abstraction layer ONLY.

---

## 5. UIConfig & UIBuilder
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UIConfig {
    pub title: String,           // default: "NAINA OS Desktop Host" (ADR DECISION)
    pub enable_overlay: bool,    // default: true (SOURCE-SUPPORTED)
    pub width: u32,              // default: 1280 (ADR DECISION)
    pub height: u32,             // default: 800 (ADR DECISION)
    pub refresh_rate_hz: u32,    // default: 60 (ADR DECISION)
    pub auto_show: bool,         // default: true (ADR DECISION)
}
```

```rust
#[derive(Debug, Default)]
pub struct UIBuilder {
    config: Option<UIConfig>,
    runtime: Option<Arc<Runtime>>,
}

impl UIBuilder {
    pub fn new() -> Self;
    pub fn with_config(mut self, config: UIConfig) -> Self;
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self;
    pub fn build(self) -> Result<UIFramework, UIError>;
}
```

---

## 6. UIState & Lifecycle State Machine
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UIState {
    Uninitialized,
    Initializing,
    Ready,
    OverlayVisible,
    OverlayHidden,
    Shutdown,
}
```
- Transitions: `Uninitialized` → `Initializing` → `Ready` ⇄ `OverlayVisible` / `OverlayHidden` → `Shutdown`.

---

## 7. UIEvent & Event Model
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UIEvent {
    OverlayShown,
    OverlayHidden,
    InputSubmitted { input_length: usize },
    CommandTriggered { command_name: String },
    WindowClosed { window_id: String },
}
```
- Event transport: Thread-safe channel messaging via `std::sync::mpsc`.

---

## 8. Runtime Context Integration
- `UIFramework` creates a dedicated microkernel execution context via `runtime.create_context("ui_overlay_host", None)`.
- The execution context MUST be closed gracefully upon `UIFramework::shutdown()`.

---

## 9. Concurrency & Lock Hierarchy
- `UIFramework` implements `Send + Sync`. Standard usage is wrapped in `Arc<UIFramework>`.
- Thread-safe synchronization via `std::sync::RwLock` and `std::sync::Mutex`.
- Strict lock acquisition hierarchy: `state` → `windows` → `logger` (prevents deadlocks).
- Core UI wrapper contains zero unsafe code.

---

## 10. Async Restrictions (No Tokio)
- **TOKIO AND EXTERNAL ASYNC RUNTIMES ARE STRICTLY FORBIDDEN**.
- Uses standard library `std::thread` and `std::sync::mpsc` channels exclusively.

---

## 11. Security & Privacy Rules
- CBAC capability validation is delegated to `Runtime` context executions.
- Zero plain-text secret logging: Passwords, capability tokens, secret payload contents, or raw text input MUST NEVER be logged.
- Input telemetry MUST record `input_length` only.

---

## 12. Error Taxonomy & Fault Recovery
```rust
#[derive(Debug)]
pub enum UIError {
    InitializationFailed { message: String },
    RenderError { message: String },
    WindowNotFound { window_id: String },
    Runtime(runtime::RuntimeError),
    Configuration(configuration::ConfigError),
    Logging(logging::error::LogError),
    LockError { message: String },
}
```
- **Fault Recovery**: Rendering failures MUST degrade gracefully to `OverlayHidden` state rather than crashing the microkernel supervisor process.

---

## 13. Performance & Resource Budgets
- **Desktop Overlay Host RAM**: **`< 200 MB`** (`FIRST_ALPHA_SPEC.md` Section 4).
- **System Idle CPU Usage**: **`< 5.0 %`** (`FIRST_ALPHA_SPEC.md` Section 4).
- *Classification: SOURCE-SUPPORTED TARGET / ENGINEERING REQUIREMENT.*

---

## 14. Rendering Architecture Boundary
- **CRITICAL BOUNDARY LOCK**:
  - The concrete physical GUI rendering backend is **UNSPECIFIED / DEFERRED**.
  - The Alpha `UIFramework` defines window/overlay state abstractions without locking a physical rendering toolkit dependency (such as egui, Slint, Webview, Winit, DirectX, or GDI).
  - Concrete pixel rendering is uncoupled from core state management to preserve microkernel simplicity and avoid async runtime contamination.

---

## 15. Alpha Scope vs. Deferred Features
- **INCLUDED IN ALPHA (v0.8.0)**:
  - `UIFramework` struct layout, `UIConfig`, and `UIBuilder`.
  - `UIState` lifecycle management.
  - `UIEvent` channel dispatch via `std::sync::mpsc`.
  - `Runtime` execution context linkage ("ui_overlay_host").
  - `UIError` taxonomy and `UIResult<T>` alias.
  - Overlay host state abstraction.
- **DEFERRED**:
  - Physical 2D/3D GUI rendering backend bindings (**UNSPECIFIED / DEFERRED**).
  - Multi-monitor spatial layout algorithms.
  - Custom CSS/theme styling parser.
  - Touch gesture recognition.

---

## 16. Architecture Risks & Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **Heavy GUI Dependency Bloat** | Unapproved GUI crates introducing Tokio runtimes | Restrict Alpha UI layer to abstract state management with `std::thread` |
| **RAM Footprint Inflation** | Texture buffer caching exceeding budget | Enforce strict `< 200 MB` overlay host RAM target |
| **CPU Spinning / Render Loops** | Unthrottled render loops | Implement event-driven updates (render on event/damage only) |
| **Direct Engine Contamination** | Importing `desktop-runtime` or `voice-runtime` | Restrict direct dependencies to `runtime`, `configuration`, and `logging` |

---

## 17. Alternatives Considered
- **Directly Bundling Winit/egui in `packages/ui`**: Rejected for Alpha to prevent introducing heavy external async runtime dependencies into microkernel crates.
- **IPC-Only UI Host**: Deferred to post-Alpha when multi-process desktop apps are wired via `apps/desktop`.

---

## 18. Verification Criteria
- `cargo check -p ui` compiles cleanly with zero errors.
- `cargo test -p ui` passes 100% of unit and lifecycle integration tests.
- `cargo fmt` and `cargo clippy -p ui -- -D warnings` produce zero warnings.
- Workspace regression check passes cleanly across all microkernel crates.

---

## 19. Status & Next Step

UI ADR-001 DRAFT COMPLETE

ADR STATUS:
**PROPOSED — PENDING REVIEW**

IMPLEMENTATION PERMITTED:
**NO**

REQUIRED NEXT STEP:
**FORMAL ADR REVIEW**
