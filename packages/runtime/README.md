# NAINA OS Runtime Package (`runtime`)

The `runtime` package provides the unified runtime execution framework for NAINA OS.

## Responsibility
Serves as the execution framework bridging microkernel process supervision (`kernel`) to user-space services, managing runtime execution contexts and execution state.

## Dependency Position
- **Depends On**: `kernel`
- **Used By**: `apps`, `services`, `orchestrator`

## Allowed Dependencies
- `kernel = { path = "../kernel" }`

## Forbidden Dependencies
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
Package initialized. Exposes the `Runtime` placeholder type. Exact execution context models, lifecycle method signatures, and kernel integration require an approved Architecture Decision Record (ADR).
