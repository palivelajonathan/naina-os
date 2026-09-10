# NAINA OS — Automation Package (`packages/automation`)

The `packages/automation` crate provides the multi-step workflow automation engine for NAINA OS. It supervises workflow step dispatch, tool routing, action triggering, conditional branching, step retry logic, cancellation protocols, and telemetry for multi-step tasks.

## Architecture & DAG Position

```
apps → automation → (runtime, services) → (kernel, capabilities, configuration, logging)
```

## Approved Direct Dependencies
- `runtime`
- `services`
- `configuration`
- `logging`

## Key Capabilities
- **Workflow Supervision**: Sequential and conditional step execution (`WorkflowSpec`, `WorkflowStep`).
- **Fault Isolation**: Controlled return of `AutomationError` values without crashing host microkernel supervisor.
- **Resource Protection**: Max 20 steps per workflow, max 5 branch levels, per-step timeout (5,000ms), workflow timeout (30,000ms), max 3 retry attempts with exponential backoff.
- **Zero-Trust Security**: Every workflow step action authorized via `ServiceRegistry` and `Runtime` capability tokens (`CBAC`). Zero plain-text secret logging.
