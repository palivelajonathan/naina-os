# NAINA OS — Client SDK (`packages/sdk`)

The `packages/sdk` crate provides the primary client-facing facade (`SDKFacade`) for applications (`apps/`) and developer tooling interacting with NAINA OS.

## Architecture & DAG Position

```
apps → sdk → (runtime, services) → (kernel, capabilities, configuration, logging)
```

## Approved Direct Dependencies
- `runtime`
- `services`
- `configuration`
- `logging`

## Key Capabilities
- **SDKFacade & Builder**: Initialization pipeline (`SDKBuilder`) creating shared `Runtime`, `ServiceRegistry`, and `Logger` handles.
- **Session Management**: Session tracking (`SDKSession`) mapping client sessions directly to microkernel `ExecutionContextId` handles.
- **Service-Mediated Architecture**: Action dispatch across high-level operating system runtimes via dynamic `ServiceRegistry` lookups and `Runtime` context executions.
- **Zero-Trust Security**: CBAC token verification delegated to `Runtime` and `ServiceRegistry` boundaries; zero plain-text credential logging.
