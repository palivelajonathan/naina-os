# NAINA OS — Services Package (`packages/services`)

## Responsibility
The `services` package provides the user-space service registration, service discovery, lifecycle management, and health tracking framework for NAINA OS.

## Role of `ServiceRegistry`
`ServiceRegistry` acts as the central registry for system and application services running above the NAINA OS runtime execution framework, managing service lookup and status.

## Layer Position & Dependency DAG
`services` resides in Layer 3 (Service Layer) of the NAINA OS package hierarchy.

### Allowed Direct Dependencies:
- `runtime = { path = "../runtime" }`
- `capabilities = { path = "../capabilities" }`

### Forbidden Direct Dependencies:
- `orchestrator`, `tool-registry`, `memory`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`

## Current Status
**INITIALIZED ONLY**

Service lifecycle state machine, registration contracts, discovery semantics, and capability security boundaries are strictly deferred until `adr_001_services.md` is drafted, reviewed, and approved.
