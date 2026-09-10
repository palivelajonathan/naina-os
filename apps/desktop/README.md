# NAINA OS — Desktop Host Application (`apps/desktop`)

The `apps/desktop` crate (`desktop-host`) provides the composition root executable binary (`naina-desktop`) for NAINA OS. It wires together all core microkernel packages and engine runtimes to fulfill the Minimum Viable NAINA (MVN) end-to-end cognitive loop (`FIRST_ALPHA_SPEC.md` Section 2).

## Architecture & Composition Root

```
Microphone → Whisper STT → CARF Planner / Orchestrator → Qwen 7B GGUF Model Runtime → Obsidian Memory Vault → Desktop Control → Piper TTS → Speaker
```

## Binary Executable
- Binary name: `naina-desktop`
- Target path: `apps/desktop/src/main.rs`

## Key Responsibilities
- Composition root startup and subsystem initialization.
- Subsystem handle registration into `ServiceRegistry`.
- Windows process signal handling (`SIGINT`, `SIGBREAK`, `WM_CLOSE`).
- Turn coordination across `VoiceRuntime`, `Orchestrator`, `ModelRuntime`, `MemoryStore`, `DesktopRuntime`, `BrowserRuntime`, `AutomationEngine`, and `UIFramework`.
- Graceful shutdown and resource cleanup.
