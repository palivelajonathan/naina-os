# NAINA OS Event Bus Package (`event-bus`)

The `event-bus` package provides system event publication and subscription capabilities for NAINA OS.

## Responsibility
Manages event dispatches, subscriptions, and message routing across NAINA OS subsystems (`runtime`, `kernel`, and `orchestrator`).

## Dependency Position
- **Depends On**: `logging`
- **Used By**: `runtime`, `orchestrator`

## Allowed Dependency
- `logging = { path = "../logging" }`

## Forbidden Dependencies
- `configuration`
- `capabilities`
- `kernel`
- `runtime`
- `services`
- `orchestrator`
- `tool-registry`
- `memory`
- `context-engine`
- `model-runtime`
- `model-providers`
- `voice-runtime`
- `browser-runtime`
- `desktop-runtime`
- `automation`
- `sdk`
- `ui`

## Current Implementation Status
Package initialized. Defines the `EventBus` type placeholder and package layout. No channels, Tokio runtime, event types, or routing behaviors have been implemented yet.
