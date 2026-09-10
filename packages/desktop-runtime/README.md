# NAINA OS — Desktop Runtime Package (`desktop-runtime`)

The `desktop-runtime` package is responsible for supervising desktop automation, Win32 application launching (`CreateProcessW`), window enumeration/state queries, UI Automation COM element inspection, and Win32 `SendInput` event injection in NAINA OS.

## DAG Position & Architectural Constraints

```
apps → desktop-runtime → (runtime, services) → (kernel, capabilities, configuration, logging)
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
- `browser-runtime`
- `automation`
- `sdk`
- `ui`
- `kernel`
- `capabilities`
- `event-bus`

## First Alpha Goals
- Launch native Windows applications (e.g. Notepad, VS Code).
- Query active window enumeration and window state (`WindowInfo`).
- Perform bounded UI Automation element inspection (max depth 5, max 100 elements, max 200ms timeout).
- Inject synthetic keyboard and mouse input events (`SendInput`).
- Enforce strict command execution latency budget of `< 500 ms`.
- Render native Win32/GDI overlay host allocating `< 200 MB` RAM.
