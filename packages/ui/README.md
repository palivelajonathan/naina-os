# NAINA OS — User Interface Framework (`packages/ui`)

The `packages/ui` crate provides the client-side user interface framework (`UIFramework`), desktop overlay host abstractions, and event handling bindings for NAINA OS desktop applications (`apps/desktop`).

## Architecture & DAG Position

```
apps → ui → runtime → (kernel, capabilities, configuration, logging)
```

## Approved Direct Dependencies
- `runtime`
- `configuration`
- `logging`

## Key Capabilities
- **UIFramework & UIBuilder**: Initialization pipeline creating shared `Runtime` and `Logger` handles.
- **State Machine**: Deterministic lifecycle state management (`Uninitialized` → `Initializing` → `Ready` ⇄ `OverlayVisible` / `OverlayHidden` → `Shutdown`).
- **Event Architecture**: Event-driven channel transport using `std::sync::mpsc`.
- **Privacy & Security**: Zero-Trust CBAC token validation delegated to `Runtime`; raw keyboard inputs/credentials are NEVER stored or logged (`input_length` telemetry only).
- **Physical GUI Rendering Backend**: **UNSPECIFIED / DEFERRED** (No physical rendering engine like egui/Slint/Winit is tied to Alpha).
