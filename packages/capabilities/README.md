# NAINA OS Capabilities Package (`capabilities`)

The `capabilities` package provides Zero-Trust Capability-Based Access Control (`CBAC`) token management and permission verification for NAINA OS.

## Responsibility
Manages capability tokens, permission verification, and subsystem access constraints in accordance with zero-trust security policies.

## CBAC Purpose & Zero-Trust Role
Serves as the security foundation verifying capability permissions before sensitive subsystem operations are executed. Under NAINA OS zero-trust principles, every sensitive subsystem request must present a verified capability token.

## Dependency Position
- **Depends On**: `configuration`, `logging`
- **Used By**: `runtime`, `services` (and `kernel`)

## Allowed Dependencies
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`

## Forbidden Dependencies
- `event-bus`
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
Package initialized. Exposes the `CapabilityRegistry` placeholder. All cryptographic signing (Ed25519/HMAC), token serialization, and permission verification semantics are explicitly awaiting an approved Architecture Decision Record (ADR).
