# NAINA OS — Browser Runtime Package (`browser-runtime`)

The `browser-runtime` package is responsible for supervising browser automation, Chrome DevTools Protocol (CDP) WebSocket communication, headless/headed browser instance control, page navigation, tab management, semantic DOM inspection, and text extraction for NAINA OS.

## DAG Position & Architectural Constraints

```
apps → browser-runtime → (runtime, services) → (kernel, capabilities, configuration, logging)
```

### Allowed Direct Dependencies:
- `runtime`
- `services`
- `configuration`
- `logging`

### Forbidden Dependencies:
- `orchestrator`
- `model-providers`
- `model-runtime`
- `context-engine`
- `memory`
- `tool-registry`
- `voice-runtime`
- `desktop-runtime`
- `automation`
- `sdk`
- `ui`
- `kernel`
- `capabilities`
- `event-bus`

## First Alpha Goals
- Launch headless Chromium process (`--headless=new`).
- Open & close browser tabs.
- Navigate URLs (`navigate_to`).
- Extract clean page text payload (`extract_page_text`).
- Query semantic DOM node hierarchy (`inspect_dom`).
- Enforce CDP navigation timeout (`navigation_timeout_ms = 10_000 ms`).
- Prevent Chromium zombie child processes via deterministic `ChildProcessGuard` cleanup on drop.
- Standard library (`std::net::TcpStream`) CDP transport — Tokio is FORBIDDEN.
