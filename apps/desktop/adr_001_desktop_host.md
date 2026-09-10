# ADR-001 — NAINA OS Desktop Host Architecture

- **Title:** ADR-001: NAINA OS Desktop Host Composition Root Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-24
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `apps/desktop`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

---

## 2. Context
NAINA OS operates on a microkernel architecture. The `apps/desktop` package represents the top-level composition root application binary (`naina-desktop`). It is responsible for initializing, wiring, and lifecycle-managing all 18 core microkernel packages and engine runtimes to fulfill the Minimum Viable NAINA (MVN) end-to-end cognitive loop (`FIRST_ALPHA_SPEC.md` Section 2):
```
Microphone → Whisper STT → CARF Planner / Orchestrator → Qwen 7B GGUF Model Runtime → Obsidian Memory Vault → Desktop Control → Piper TTS → Speaker
```

The composition root MUST NOT duplicate business logic, task planning, or engine runtime execution belonging to lower-level packages.

---

## 3. Decision

### 3.1 Package Identity
- **Package Name**: `desktop-host` (`apps/desktop`)
- **Binary Name**: `naina-desktop`
- **Composition Root API**: `DesktopHostApp`
- **Responsibility**: Application-level composition root only. It initializes microkernel components, wires dependency handles into `ServiceRegistry`, coordinates top-level turn execution across engine APIs, and manages Windows application lifecycle and graceful shutdown.

### 3.2 Composition Root Layout
```rust
pub struct DesktopHostApp {
    config: DesktopHostConfig,
    sdk: Arc<SDKFacade>,
    ui: Arc<UIFramework>,
    orchestrator: Arc<Orchestrator>,
    voice: Arc<VoiceRuntime>,
    desktop: Arc<DesktopRuntime>,
    browser: Arc<BrowserRuntime>,
    automation: Arc<AutomationEngine>,
    state: RwLock<DesktopHostState>,
}
```
- `DesktopHostConfig`: Model paths (Qwen GGUF, Whisper STT, Piper TTS), Obsidian Vault path, hotkey settings.
- `DesktopHostState`: `Uninitialized` → `Booting` → `Ready` ⇄ `ProcessingTurn` → `ShuttingDown` → `Stopped`.
- `DesktopHostError`: Taxonomy encapsulating `BootFailed`, `VoiceError`, `OrchestratorError`, `ModelError`, `MemoryError`, `ShutdownError`, `LockError`.
- `DesktopHostResult<T>`: Result type alias.

### 3.3 Dependency DAG
- **Allowed Application-Level Direct Dependencies**:
  `sdk`, `ui`, `orchestrator`, `voice-runtime`, `desktop-runtime`, `browser-runtime`, `automation`, `model-runtime`, `model-providers`, `memory`, `context-engine`, `tool-registry`, `services`, `runtime`, `configuration`, `logging`.
- **Forbidden Direct Dependencies**:
  `kernel` (direct application import forbidden - accessed exclusively via `runtime`), `capabilities` (direct application import forbidden - accessed via `runtime`/`services` CBAC token handles).
- **Async Restrictions**: `tokio`, `async-std`, `smol`, or external async runtimes are **STRICTLY FORBIDDEN**.

### 3.4 MVN Cognitive Pipeline
```
               MINIMUM VIABLE NAINA (MVN) COGNITIVE LOOP
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Voice Input  │ ──> │ CARF Planner │ ──> │ Qwen GGUF    │ ──> │ Obsidian     │
│ (Whisper)    │     │ (Intent)     │     │ (ARAL Model) │     │ Memory Vault │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
                                                                      │
┌──────────────┐     ┌──────────────┐                                 ▼
│ Voice Output │ <── │ Desktop      │ <───────────────────────────────┘
│ (Piper TTS)  │     │ Control      │
└──────────────┘     └──────────────┘
```
1. `VoiceRuntime::stt_transcribe()` receives input buffer.
2. `ContextEngine` retrieves multi-turn conversation history.
3. `Orchestrator::submit_task()` creates CARF execution plan.
4. `ModelRuntime` dispatches prompt to `QwenGgufAdapter`.
5. `MemoryStore::hybrid_search()` queries local Obsidian Vault notes.
6. `DesktopRuntime` / `BrowserRuntime` / `AutomationEngine` executes task actions.
7. `VoiceRuntime::tts_synthesize()` converts output text to speech.

### 3.5 Service Registration
`DesktopHostApp` registers the following service handles into `ServiceRegistry`:
- `service.voice` → `VoiceRuntime`
- `service.model` → `ModelRuntime`
- `service.memory` → `MemoryStore`
- `service.tools` → `ToolRegistry`
- `service.desktop` → `DesktopRuntime`
- `service.browser` → `BrowserRuntime`
- `service.automation` → `AutomationEngine`
- `service.orchestration` → `Orchestrator`
- `service.ui` → `UIFramework`

### 3.6 Startup Sequence
1. Load configuration via `configuration::ConfigLoader`.
2. Initialize `logging::Logger` with component `"desktop-host"`.
3. Construct `SDKBuilder` producing `SDKFacade`, `Runtime`, `ServiceRegistry`.
4. Instantiate `MemoryStore`, `ContextEngine`, `ToolRegistry`, `ModelRuntime`, `Orchestrator`.
5. Instantiate `VoiceRuntime`, `DesktopRuntime`, `BrowserRuntime`, `AutomationEngine`, `UIFramework`.
6. Register subsystem handles into `ServiceRegistry`.
7. Transition `DesktopHostState` to `Ready`.

### 3.7 State Machine
```
Uninitialized → Booting → Ready ⇄ ProcessingTurn → ShuttingDown → Stopped
```
- Invalid state transitions reject with controlled `DesktopHostError`.

### 3.8 Windows Host Lifecycle
- Executable binary entry point: `apps/desktop/src/main.rs`.
- Signal handling: Intercepts Ctrl+C (`SIGINT`), Ctrl+Break (`SIGBREAK`), and Windows close events (`WM_CLOSE`), triggering `DesktopHostApp::shutdown()`.
- Process exit: Clean termination with code `0`.

### 3.9 Concurrency
- `DesktopHostApp` implements `Send + Sync` (`Arc<DesktopHostApp>`).
- Synchronization via `std::sync::RwLock` and `Mutex`.
- Heavy operations (model inference, STT/TTS) executed on background `std::thread` worker channels. Zero Tokio.

### 3.10 Security / CBAC
- Token verification delegated to `Runtime` context executions.
- Zero plain-text secret logging: Passwords, tokens, credentials, raw microphone audio, or raw user text MUST NEVER be logged.

### 3.11 Hardware Boundary
- **REAL IMPLEMENTATION**: Process supervision, BM25/vector memory search, CDP browser control, Win32 app launching, PCM audio framing, pipeline orchestration.
- **MOCK / ABSTRACT BOUNDARY**: Physical microphone capture (CPAL/ALSA) and speaker playback (audio PCM buffers passed via API).

### 3.12 UI Boundary
- PHYSICAL GUI RENDERING BACKEND: **UNSPECIFIED / DEFERRED**.
- `DesktopHostApp` composes `UIFramework` state/event management only.

### 3.13 Model Boundary
- Supports `QwenGgufAdapter` with strict **`< 4.8 GB`** VRAM limit enforcement.

### 3.14 Memory Boundary
- `MemoryStore` parses Markdown files in local Obsidian Vault path using BM25 keyword + vector similarity hybrid search.

### 3.15 Voice Boundary
- `VoiceRuntime` manages STT (`Whisper`) and TTS (`Piper`) pipelines and PCM audio frame formatting.

### 3.16 Failure Isolation
- Subsystem errors (voice turn fail, model fail, desktop action fail) surface as controlled `DesktopHostError` values without crashing the microkernel supervisor.

### 3.17 Performance Targets

| Parameter | Target Threshold | Classification |
| :--- | :---: | :--- |
| **System Cold Boot** | **`< 2.0 s`** | **MEASURED BENCHMARK** |
| **System Idle RAM** | **`< 1.0 GB`** | **ENGINEERING TARGET** |
| **System Idle CPU** | **`< 5.0 %`** | **ENGINEERING TARGET** |
| **Desktop Overlay RAM** | **`< 200 MB`** | **ENGINEERING TARGET** |
| **Voice Processing Latency** | **`< 700 ms`** | **MEASURED BENCHMARK** |
| **Memory Retrieval Latency** | **`< 300 ms`** | **MEASURED BENCHMARK** |
| **Desktop Execution Latency** | **`< 500 ms`** | **MEASURED BENCHMARK** |

### 3.18 Alpha Scope
- `DesktopHostApp` composition root struct, startup sequence, service registration, turn coordination, signal handling, and shutdown.

### 3.19 Deferred Features
- Physical GUI rendering engine backend (**UNSPECIFIED / DEFERRED**).
- Physical microphone/speaker hardware device drivers.
- Cloud synchronization (NSP).
- WASM plugin execution engine.

---

## 4. Consequences
- **Positive**: Complete end-to-end integration of NAINA OS Alpha without breaking microkernel encapsulation or introducing async runtime bloat.
- **Negative**: Physical audio I/O requires PCM API buffer wrapping until hardware HAL drivers are implemented in Beta.

---

## 5. Risks and Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **Composition Root Scope Creep** | Reimplementing business logic in `apps/desktop` | Delegate all domain actions to lower-level package APIs |
| **Tokio Contamination** | Importing external async crates in app root | Enforce standard library `std::thread` and `std::sync::mpsc` |
| **Shutdown Deadlocks** | Blocking worker threads during shutdown | Implement timeout-bounded thread joins and context cancellation |

---

## 6. Verification / Acceptance Criteria
- `cargo check -p desktop-host` compiles cleanly with 0 errors.
- `cargo test -p desktop-host` passes 100% of integration tests.
- Workspace regression check passes 100% cleanly across all 19 microkernel crates.

---

## 7. Implementation Permission

IMPLEMENTATION PERMITTED:
**NO**
