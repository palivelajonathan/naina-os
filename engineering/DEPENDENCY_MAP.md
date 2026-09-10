# Dependency Map

This document is the single source of truth for package dependencies in the repository.

## Package dependency matrix

| Package | Depends On | Used By | Public API |
| --- | --- | --- | --- |
| configuration | none | almost everything | ConfigLoader, ConfigProvider |
| logging | configuration | runtime, kernel | Logger |
| event-bus | logging | runtime, orchestrator | EventBus |
| capabilities | configuration, logging | runtime, services | CapabilityRegistry |
| kernel | configuration, logging, event-bus, capabilities | runtime | Kernel |
| runtime | kernel | apps, services, orchestrator | Runtime |
| services | runtime, capabilities | orchestrator | ServiceRegistry |
| orchestrator | runtime, memory, context-engine, model-runtime, tool-registry | apps | Orchestrator |
| tool-registry | capabilities, runtime | orchestrator | ToolRegistry |
| memory | configuration, logging | orchestrator, context-engine | MemoryStore |
| context-engine | memory, configuration, logging | orchestrator | ContextEngine |
| model-runtime | configuration, logging | orchestrator, model-providers | ModelRuntime |
| model-providers | model-runtime, configuration, logging | orchestrator | ModelProvider |
| voice-runtime | runtime, services | apps | VoiceRuntime |
| browser-runtime | runtime, services | apps | BrowserRuntime |
| desktop-runtime | runtime, services | apps | DesktopRuntime |
| automation | runtime, services | apps | AutomationEngine |
| sdk | none | apps, tooling | SDKFacade |
| shared | none | all packages | SharedTypes |
| types | none | all packages | CoreTypes |
| ui | runtime | apps | UIFramework |

## Dependency rules

- Dependencies must form a directed acyclic graph.
- Higher-level packages may depend on lower-level packages, but lower-level packages must not depend on higher-level ones.
- The orchestrator may depend on the runtime and engine packages, but runtime-level packages should not depend on orchestrator-level concerns.
