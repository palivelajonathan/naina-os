# NAINA OS — Tool Registry Package (`packages/tool-registry`)

## Responsibility
The `tool-registry` package provides capability-controlled tool registration, tool metadata management, tool execution lookup, and security-enforced tool discovery for NAINA OS.

## Role of `ToolRegistry`
`ToolRegistry` acts as the central registry for system and agent tools, verifying `CAP_TOOL_REGISTER` and `CAP_TOOL_EXECUTE` capability tokens via `CapabilityRegistry` and linking tools to `runtime::ExecutionContextId` execution environments.

## Layer Position & Dependency DAG
`tool-registry` resides in Layer 3 (Tool Layer) of the NAINA OS package hierarchy.

### Allowed Direct Dependencies:
- `capabilities = { path = "../capabilities" }`
- `runtime = { path = "../runtime" }`

### Forbidden Direct Dependencies:
- `orchestrator`, `memory`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`
