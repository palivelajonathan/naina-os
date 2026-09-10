# ADR-001: NAINA OS Browser Runtime Architecture

- **Title:** ADR-001: NAINA OS Browser Runtime Architecture
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-24
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/browser-runtime`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

*Implementation Permitted: NO*

---

## 2. Context
NAINA OS uses a modular microkernel architecture. The `packages/browser-runtime` package acts as the system browser automation runtime manager for the operating system:
```
apps → browser-runtime → (runtime, services) → (kernel, capabilities, configuration, logging)
```

The browser runtime provides Chrome DevTools Protocol (CDP) WebSocket communication, headless/headed browser instance control, page navigation, tab management, semantic DOM inspection, and text extraction for NAINA OS.

---

## 3. Decision
We lock the architectural design for `packages/browser-runtime` as specified in this document.

---

## 4. Dependency/DAG Boundaries
- **Allowed Direct Dependencies**:
  - `runtime = { path = "../runtime" }`
  - `services = { path = "../services" }`
  - `configuration = { path = "../configuration" }`
  - `logging = { path = "../logging" }`

- **Explicit Forbidden Dependencies**:
  `orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `voice-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`, `kernel`, `capabilities`, `event-bus`.

---

## 5. Browser Process Architecture
- Spawns Chromium/Edge browser process via standard `std::process::Command`.
- Process Supervision: PID tracking, deterministic child cleanup, `kill() + wait()` on supervisor shutdown/drop to prevent orphan Chromium zombie processes.
- Discovery Strategy:
  1. Configured explicit path (`BrowserRuntimeConfig.browser_executable_path`).
  2. Standard OS installation locations (Chrome / Edge / Chromium on Windows, Linux, macOS).
  3. `PATH` environment variable lookup.
  4. `MockBrowserEngine` fallback for headless CI execution.

---

## 6. CDP Transport Architecture
- **CRITICAL TRANSPORT DECISION**: **Tokio is FORBIDDEN**.
- The Alpha CDP transport is locked strictly to:
  - `std::net::TcpStream`
  - HTTP Upgrade handshake (`GET /json/version HTTP/1.1`)
  - Minimal WebSocket framing & parser
  - `std::thread` OS background worker execution
  - `std::sync::mpsc` channel transport
- All CDP transport logic is isolated behind `CdpTransport` in `src/platform.rs`.
- Do NOT introduce `tokio`, `async-std`, `smol`, or external async runtimes.

---

## 7. Browser Lifecycle
- Headless Mode: Default `headless = true` (uses `--headless=new` flag). Configurable to `headless = false` for active debugging.
- CDP Endpoint: Default remote debugging port `9222`. Queries `http://127.0.0.1:<port>/json/version` to obtain the browser WebSocket target URL (`ws://127.0.0.1:<port>/devtools/browser/...`).

---

## 8. Data Models
```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageInfo {
    pub target_id: String,
    pub url: String,
    pub title: String,
    pub is_loading: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DomNode {
    pub node_id: u32,
    pub tag_name: String,
    pub attributes: Vec<(String, String)>,
    pub text_content: String,
    pub children_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowserActionRequest {
    pub url: Option<String>,
    pub action_type: String,
    pub target_selector: Option<String>,
    pub text_payload: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowserActionResult {
    pub target_id: String,
    pub current_url: String,
    pub status: String,
    pub extracted_text: Option<String>,
}
```

---

## 9. Navigation & Tab Management
- Public operations:
  - `navigate_to(&self, url: &str) -> Result<BrowserActionResult>`
  - `open_tab(&self, url: &str) -> Result<PageInfo>`
  - `close_tab(&self, target_id: &str) -> Result<()>`
- Waits for CDP `Page.loadEventFired` event and enforces `navigation_timeout_ms` (10,000 ms).
- Auto-reconnects on socket disconnect by re-querying `/json/version`.

---

## 10. Semantic DOM Inspection
- `inspect_dom(&self, target_id: &str) -> Result<Vec<DomNode>>`
- Queries semantic DOM node hierarchy using CDP `DOM.getDocument` and `DOM.flattenedChildren`.

---

## 11. Page Text Extraction
- `extract_page_text(&self, target_id: &str) -> Result<String>`
- Strips `<script>`, `<style>`, HTML noise and extracts clean readable text payload.

---

## 12. State Machine
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrowserState {
    Idle,
    StartingBrowser,
    Navigating,
    InspectingDom,
    ExtractingText,
    Error,
}
```
Valid successful operations return to `Idle`. Operational failures transition to `Error`.

---

## 13. Error Model
```rust
pub enum BrowserRuntimeError {
    BrowserLaunchFailed { message: String },
    ConnectionFailed { message: String },
    NavigationFailed { url: String, message: String },
    DomInspectionFailed { message: String },
    TabNotFound { target_id: String },
    Runtime(runtime::RuntimeError),
    Service(services::ServicesError),
    Configuration(configuration::ConfigError),
    LockError { message: String },
}
```

---

## 14. Concurrency & Cancellation
- `BrowserRuntime` MUST be `Send + Sync` (`Arc<BrowserRuntime>`).
- Internal synchronization via `std::sync::RwLock` and `Mutex`.
- Cancellation via `Arc<AtomicBool>` checked before and during CDP command execution.

---

## 15. Security & Privacy
- Every operation must be authorized via `ServiceRegistry` and `Runtime` capability tokens (`CBAC`).
- Zero logging of credentials, authentication tokens, cookies, session secrets, or sensitive page text.
- Privacy: Uses ephemeral incognito browser profile (`--user-data-dir` tempdir); wipes profile and session data when browser process exits.

---

## 16. Runtime & Service Integration
- `runtime`: `Arc<Runtime>` supervises execution contexts.
- `services`: `Arc<ServiceRegistry>` registers `BrowserService`.
- `configuration`: `BrowserRuntimeConfig::from_browser_config(&configuration::Config)`.

---

## 17. Performance & Resource Constraints
- Latency target: `< 1,000 ms` for navigation and text extraction. `LatencyTargetExceeded` is a diagnostic log warning event only.
- Resource target: Single Chromium instance managed within workstation memory limits.

---

## 18. Alpha Scope
- Headless browser process launch (`--headless=new`).
- Page navigation & URL loading.
- Tab opening & closing.
- Page text extraction.
- Semantic DOM node inspection.

---

## 19. Deferred Features
- Full extension framework.
- PDF printing pipeline.
- Canvas/WebGL video recording.
- Multi-user profile synchronization.

---

## 20. Architecture Risks & Mitigations

| Risk | Cause | Mitigation Strategy |
| :--- | :--- | :--- |
| **Chromium Zombie Processes** | Unexpected supervisor process termination | Implement `ChildProcessGuard` with `kill() + wait()` cleanup on drop |
| **CDP Socket Hangs** | Hanging socket read on CDP event stream | Set socket read timeouts on `std::net::TcpStream` |
| **Tokio Contamination** | Importing external async runtimes into microkernel | Enforce `std`-only TCP/WebSocket transport in `src/platform.rs` |
| **Data Leakage** | Plaintext credential logging from input fields | Enforce Zero-Trust masking; zero logging of credentials/cookies |

---

## 21. Acceptance Criteria
- Clean construction of `BrowserRuntime` with `BrowserRuntimeConfig`.
- Non-blocking browser tab navigation and URL loading.
- Page text extraction and semantic DOM node hierarchy querying.
- Clean recovery from navigation timeouts and socket disconnects.

---

## 22. Consequences
- Guarantees lightweight microkernel encapsulation without Tokio async runtime overhead.
- Provides robust, fault-isolated browser automation for NAINA OS.

---

## 23. Verification Checklist
- [ ] `cargo check -p browser-runtime`
- [ ] `cargo test -p browser-runtime`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy -p browser-runtime --all-targets --all-features -- -D warnings`
- [ ] Workspace regression tests pass cleanly

---

## IMPLEMENTATION CONTRACT

- **Public Struct**: `pub struct BrowserRuntime`
- **Allowed Direct Dependencies**: `runtime`, `services`, `configuration`, `logging`.
- **Forbidden Dependencies**: `orchestrator`, `model-providers`, `model-runtime`, `context-engine`, `memory`, `tool-registry`, `voice-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`, `kernel`, `capabilities`, `event-bus`.

Status: PROPOSED — PENDING REVIEW  
Implementation permitted: NO
