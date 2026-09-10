# NAINA OS — Context Engine Package (`packages/context-engine`)

## Responsibility
The `context-engine` package provides multi-turn conversation context management, context window token budgeting, memory retrieval integration, and prompt context assembly for NAINA OS.

## Role of `ContextEngine`
`ContextEngine` manages multi-turn conversation turns across user interactions, enforcing token limits, pruning old turns, and retrieving long-term memory entries from `memory::MemoryStore` for downstream orchestrator consumption.

## Layer Position & Dependency DAG
`context-engine` resides in the Engine Layer of the NAINA OS package hierarchy (`DEPENDENCY_MAP.md`).

### Allowed Direct Dependencies:
- `memory = { path = "../memory" }`
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`

### Forbidden Direct Dependencies:
- `runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, `tool-registry`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`

## FIRST_ALPHA Relevance
HIGH (`FIRST_ALPHA_SPEC.md` Section 7 Checklist Item 9: Retains multi-turn conversation memory across at least 5 user turns).

## Current Status
INITIALIZED ONLY — Pending API discovery and ADR-001 review before implementation.
