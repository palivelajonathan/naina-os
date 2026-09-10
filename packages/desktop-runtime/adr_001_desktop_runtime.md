# ADR-001: NAINA OS Desktop Runtime Architecture

- **Title:** ADR-001: NAINA OS Desktop Runtime Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-23
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/desktop-runtime`

---

## 1. Status
**APPROVED**

*Implementation Permitted: YES*

---

## 2. Context & Responsibility

NAINA OS uses a modular microkernel architecture. The `packages/desktop-runtime` package acts as the top-level desktop overlay and system automation runtime manager for the operating system:
```
apps → desktop-runtime → (runtime, services) → (kernel, capabilities, configuration, logging)
```

The primary responsibilities of `desktop-runtime` are:
1. Launching Windows applications (e.g. Notepad, VS Code) via native Win32 process execution.
2. Enumerating active desktop windows and querying window state metadata.
3. Inspecting semantic UI elements using Windows UI Automation COM interfaces.
4. Injecting synthetic keyboard and mouse input events via Win32 `SendInput`.
5. Rendering a lightweight desktop overlay window allocating `< 200 MB` RAM.
6. Enforcing a command execution latency budget of `< 500 ms`.
7. Isolating Win32 COM threads and unsafe platform FFI boundaries from the host microkernel supervisor.

---

## 3. Requirements Classification

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **First Alpha Specification (`FIRST_ALPHA_SPEC.md`)**:
  - Section 2 (MVN Core Pipeline): Desktop Control execution pipeline.
  - Section 3: Performance Target (Command Execution Latency < 500ms).
  - Section 4: Hardware Resource Budget (Desktop Overlay Host RAM < 200 MB).
  - Section 6: Milestone Tracker Week 6 (Windows Desktop Overlay Host & System Control API).
  - Section 7 Checklist (Item 5): Windows App Launching (Notepad, VS Code).
  - Section 7 Checklist (Item 10): Automatically restarts failed worker processes without crashing the NKRS supervisor.
- **Package Rules & Dependency Map (`engineering/DEPENDENCY_MAP.md`)**:
  - `desktop-runtime` depends directly on `runtime`, `services`, `configuration`, and `logging`.
  - `orchestrator`, `model-runtime`, `model-providers`, `memory`, `context-engine`, `tool-registry`, `voice-runtime`, `browser-runtime`, and `automation` MUST NOT be direct dependencies of `desktop-runtime`.
- **Project Charter (`PROJECT_CHARTER.md`)**:
  - Zero-Trust Security & Privacy: Capability tokens enforced via `ServiceRegistry` and `Runtime`; no plain-text credential logging.

### B. ARCHITECTURAL DECISIONS LOCKED BY THIS ADR:
1. **Separation of Concerns**:
   - **Win32 API**: Owns process launching (`CreateProcessW`), HWND window enumeration (`EnumWindows`), window metadata (`GetWindowTextW`), and synthetic input injection (`SendInput`).
   - **UI Automation COM**: Owns semantic UI element tree traversal (`IUIAutomation`).
2. **Locked Process Launching**: Win32 `CreateProcessW` is locked as the SINGLE Alpha application launch mechanism.
3. **Locked Overlay Architecture**: Native Win32/GDI layered transparent window is locked as the Alpha desktop overlay host (WebView and DirectX are explicitly excluded).
4. **Locked Input Injection**: Win32 `SendInput` API is locked as the keyboard/mouse event injection mechanism.
5. **Strict Input-Security Boundaries**:
   - Prohibited: Global keylogging drivers, credential capture, uncontrolled background input injection.
   - Required: Capability token validation on every desktop control action via `ServiceRegistry` and `Runtime`.
6. **Bounded UI Automation Traversal**:
   - Maximum traversal depth: `5 levels`.
   - Maximum element count: `100 elements`.
   - Maximum inspection timeout: `200 ms`.
7. **WindowHandle Serialized Representation**:
   - `WindowInfo.handle` represented as `u64` (serialized HWND).
   - Native HWND handle validation (`IsWindow`) strictly isolated inside platform FFI module (`src/platform.rs`).
8. **Isolated Unsafe FFI Boundary**:
   - Unsafe Win32 / COM FFI code strictly confined to `src/platform.rs`. Core runtime wrapper contains zero unsafe code.
9. **COM STA Thread Ownership & Fault Recovery**:
   - UI Automation COM calls run on a dedicated Single-Threaded Apartment (STA) background worker thread (`std::thread`) with `mpsc` channel transport and `AtomicBool` cancellation token.
   - Native COM or HWND errors caught via `catch_unwind` and returned as controlled `DesktopRuntimeError`.
10. **Latency Budget Strategy**:
    - Command execution latency budget: `< 500 ms`. `LatencyTargetExceeded` is a **diagnostic logging metric** (warning sink event); it does NOT fail the turn.
11. **Configuration Integration**: `DesktopRuntimeConfig` populates from `configuration::DesktopConfig` (`enabled: bool`).

### C. ENGINEERING TARGETS / ASSUMPTIONS:
- Desktop Overlay RAM peak `< 200 MB`.
- Command execution dispatch latency target `< 500 ms`.
- UI Automation tree traversal timeout `< 200 ms`.

### D. DEFERRED FEATURES (Post-1.0 / Beta):
- Advanced OCR screen fallback engines.
- Multi-monitor 3D spatial overlay rendering.
- Global keylogging hook drivers.
- Cross-platform Linux/macOS X11/Wayland desktop drivers.

---

## 4. Architectural Specifications (ADR-001)

### A. DesktopRuntime Structure
```rust
pub struct DesktopRuntime {
    config: DesktopRuntimeConfig,
    runtime: Arc<Runtime>,
    services: Arc<ServiceRegistry>,
    state: RwLock<DesktopState>,
    cancel_flag: Arc<AtomicBool>,
}
```

---

### B. Data Models
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopState {
    Idle,
    LaunchingApp,
    InspectingUi,
    InjectingInput,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowInfo {
    pub handle: u64, // Serialized HWND
    pub title: String,
    pub process_id: u32,
    pub bounds: Rect,
    pub is_visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiElement {
    pub element_id: String,
    pub name: String,
    pub control_type: String,
    pub bounds: Rect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppLaunchRequest {
    pub app_name: String,
    pub executable_path: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppLaunchResult {
    pub process_id: u32,
    pub window_handle: u64,
    pub status: String,
}
```

---

### C. Error Model
```rust
pub enum DesktopRuntimeError {
    AppLaunchFailed { message: String },
    WindowNotFound { handle: u64 },
    UiElementNotFound { element_id: String },
    InputInjectionFailed { message: String },
    ComInitializationFailed { message: String },
    Runtime(runtime::RuntimeError),
    Service(services::ServicesError),
    Configuration(configuration::ConfigError),
    LockError { message: String },
}
```

---

## 5. Architectural Risks & Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **COM STA Deadlocks** | Blocking Win32 COM calls in runtime loops | Isolate COM execution inside dedicated STA background worker thread |
| **Memory Leaks** | Unbounded UI Automation tree traversal | Enforce hard bounds: max depth = 5, max elements = 100, max timeout = 200ms |
| **Supervisor Crashes** | Invalid HWND dereference or Win32 panic | Validate handle with `IsWindow` and wrap FFI calls inside `catch_unwind` |
| **High Latency** | Slow application window initialization | Asynchronous process launch with non-blocking window handle polling |

---

## IMPLEMENTATION CONTRACT

- **Public Struct**: `pub struct DesktopRuntime`
- **Allowed Direct Dependencies**: `runtime`, `services`, `configuration`, `logging`.
- **Forbidden Dependencies**: `orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `voice-runtime`, `browser-runtime`, `automation`, `sdk`, `ui`, `kernel`, `capabilities`, `event-bus`.

Status: APPROVED  
Implementation permitted: YES
