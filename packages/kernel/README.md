# NAINA OS Kernel Package (`kernel`)

The `kernel` package provides the Microkernel NKRS process supervisor and lifecycle management for NAINA OS.

## Responsibility
Serves as the core microkernel process supervisor overseeing process startup, cold boot initialization (< 2.0s), process isolation boundaries, and service registration lifecycle across NAINA OS runtimes.

## NKRS & Supervisor Roles
- **Process Supervision**: Monitors system worker processes and enforces recovery policies.
- **Lifecycle Coordination**: Orchestrates cold-boot sequencing and graceful shutdown transitions.
- **Process Isolation**: Enforces boundaries between low-level microkernel subsystems and user-space processes.
- **Service Registration**: Manages subsystem entry point registrations.

## Dependency Position
- **Depends On**: `configuration`, `logging`, `event-bus`, `capabilities`
- **Used By**: `runtime`

## Allowed Dependencies
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`
- `event-bus = { path = "../event-bus" }`
- `capabilities = { path = "../capabilities" }`

## Forbidden Dependencies
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
Package initialized. Exposes the `Kernel` placeholder type. Exact lifecycle method signatures (`boot`, `start`, `shutdown`), process supervision algorithms, and restart policies require an approved Architecture Decision Record (ADR).
