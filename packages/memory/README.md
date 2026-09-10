# NAINA OS — Memory Package (`packages/memory`)

## Responsibility
The `memory` package provides persistent memory storage, hybrid vector and BM25 search retrieval, document indexing, and vault storage management for NAINA OS.

## Role of `MemoryStore`
`MemoryStore` acts as the primary memory store interface for indexing local Markdown notes and supporting hybrid memory search for context engines and orchestrators.

## Layer Position & Dependency DAG
`memory` resides in the Engine/Storage layer of the NAINA OS package hierarchy (`DEPENDENCY_MAP.md`).

### Allowed Direct Dependencies:
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`

### Forbidden Direct Dependencies:
- `runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, `tool-registry`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`
