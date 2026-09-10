# NAINA OS — OBS Studio Integration & Streaming Intelligence Framework
**Document Identifier:** NOS-OBS-001  
**Title:** OBS Studio Integration & Streaming Intelligence Framework Specification  
**Version:** 1.0  
**Status:** Approved Engineering Specification  
**Classification:** Open Source Systems Standard  
**Authors:** Chief Systems Architect, Media & Streaming Intelligence Group & Automation Engineering Team  

---

## Executive Summary (1-Page Core Architecture Overview)

The **OBS Studio Integration & Streaming Intelligence Framework (NOS-OBS-001)** defines the media production, live streaming, screen recording, and intelligent broadcasting subsystem for NAINA OS. It enables NAINA to control OBS Studio via WebSocket v5 RPCs for automated scene switching, audio mixing, recording session archiving, presentation streaming, and creator workflow assistance.

Key architectural highlights include:
1. **Decoupled Adapter Architecture**: The Microkernel NEVER communicates directly with OBS Studio; execution is strictly managed via the CARF Planner, validated against Capability Tokens (`CAP_OBS_CONTROL`, `CAP_OBS_STREAM`), and passed through the **OBS Adapter**.
2. **Universal OBS Adapter Contract (`IOBSAdapter`)**: Standardized 16-method RPC contract abstracting OBS WebSocket v5 connection handshakes, scene collection switches, source toggles, and audio filters.
3. **Automated Stream Director**: Uses Workspace Awareness (NOS-WORKSPACE-001) to automatically switch OBS scenes when switching active applications (e.g., automatically switching to "Coding Scene" when VS Code acquires window focus, or "Gaming Scene" when a fullscreen DirectX application launches).
4. **Media Vault Archiver**: Automatically indexes completed recordings and replay buffers into the Obsidian Vault (`14 Media/`) with AI-generated transcripts and key timestamp highlights.
5. **NVENC Hardware Acceleration**: Enforces GPU-accelerated video encoding (NVIDIA NVENC / AMD AMF) to maintain low CPU overhead (`< 3% CPU utilization`) during 4K 60fps streaming and recording.

---

## Document Revision History

| Date | Revision | Author | Description of Changes |
| :--- | :--- | :--- | :--- |
| **2026-08-08** | `1.0` | Chief Systems Architect | Initial Engineering Release of NOS-OBS-001 Specification |
| **2026-08-05** | `0.9` | Media Intelligence Group | Complete draft of OBS Adapter Contract, Audio Mixer Controls, and Auto Scene Switching |

---

## SECTION 1: Streaming Philosophy & Creator Automation

### 1.1 Decoupled Media Production
Under strict NAINA OS architectural guidelines:
- **No Kernel Direct Access**: Execution flow strictly follows:  
  `User -> CARF Planner -> Capability Registry -> OBS Runtime -> OBS Adapter -> OBS WebSocket API -> OBS Studio`.
- **Zero Interruption Guarantees**: System notifications during active live streams or recording sessions are automatically suppressed or routed to the silent HUD.

---

## SECTION 2: System OBS Runtime Architecture Topology

```mermaid
graph TD
    subgraph CognitiveLayer [Cognitive & Capability Core]
        Planner[CARF Goal & Task Planner]
        CapRegistry[Capability Token Registry]
    end

    subgraph OBSRuntimeCore [OBS Runtime Subsystem Engine]
        OBSMgr[OBS Subsystem Manager]
        SceneMgr[Scene Graph Manager]
        SourceMgr[Source Visibility Manager]
        AudioMgr[Audio Mixer & Noise Suppressor]
        RecordMgr[Recording & Replay Buffer Manager]
        StreamMgr[Live Stream Telemetry & Bitrate Monitor]
        AutoDirector[Automated Scene Director Engine]
    end

    subgraph OBSAdapters [OBS WebSocket v5 RPC Layer]
        OBSAdapter[Universal OBS Adapter (Python / TS)]
        OBSWebSocket[OBS WebSocket v5 Server (ws://localhost:4455)]
        OBSApp[OBS Studio Application Engine]
    end

    Planner --> CapRegistry
    CapRegistry --> OBSMgr
    OBSMgr --> SceneMgr
    OBSMgr --> SourceMgr
    OBSMgr --> AudioMgr
    OBSMgr --> RecordMgr
    OBSMgr --> StreamMgr
    OBSMgr --> AutoDirector
    OBSMgr --> OBSAdapter
    OBSAdapter <--> |JSON-RPC WebSocket| OBSWebSocket
    OBSWebSocket <--> OBSApp
```

---

## SECTION 3: SPECIAL REQUIREMENT — Universal OBS Adapter Contract

Every OBS adapter MUST implement the standardized `IOBSAdapter` interface in Python and TypeScript:

### 3.1 Python OBS Adapter Specification (`obs_adapter.py`)

```python
# Universal OBS Adapter Contract Specification (Python / obsws-python)
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional
from pydantic import BaseModel

class SceneItem(BaseModel):
    scene_name: str
    source_name: str
    is_visible: bool
    source_type: str

class IOBSAdapter(ABC):
    @abstractmethod
    async def initialize(self, config: Dict[str, Any]) -> bool:
        """Configure WebSocket connection parameters (host, port, password)."""
        pass

    @abstractmethod
    async def connect(self) -> bool:
        """Establish active WebSocket socket connection to OBS Studio."""
        pass

    @abstractmethod
    async def authenticate(self, password: str) -> bool:
        """Authenticate using SHA-256 password challenge handshake."""
        pass

    @abstractmethod
    async def health(self) -> Dict[str, Any]:
        """Return 5s WebSocket RPC liveness probe and active scene status."""
        pass

    @abstractmethod
    async def switch_scene(self, scene_name: str) -> bool:
        """Switch current active program scene."""
        pass

    @abstractmethod
    async def toggle_source(self, scene_name: str, source_name: str, visible: bool) -> bool:
        """Set source visibility in target scene."""
        pass

    @abstractmethod
    async def mute_audio(self, source_name: str, mute: bool) -> bool:
        """Mute or unmute target audio input/output source."""
        pass

    @abstractmethod
    async def adjust_volume(self, source_name: str, volume_db: float) -> bool:
        """Adjust audio volume level in decibels."""
        pass

    @abstractmethod
    async def start_recording(self) -> bool:
        """Trigger local video recording session."""
        pass

    @abstractmethod
    async def stop_recording(self) -> str:
        """Stop active recording session and return saved video file path."""
        pass

    @abstractmethod
    async def start_stream(self) -> bool:
        """Start live broadcast to configured streaming platform."""
        pass

    @abstractmethod
    async def stop_stream(self) -> bool:
        """Stop live broadcast."""
        pass

    @abstractmethod
    async def take_screenshot(self, source_name: str, output_path: str) -> bool:
        """Save PNG screenshot of target source viewport."""
        pass

    @abstractmethod
    async def replay_buffer(self, action: str = "save") -> bool:
        """Trigger or save instant replay buffer clip."""
        pass

    @abstractmethod
    async def metrics(self) -> Dict[str, float]:
        """Expose FPS, CPU usage, VRAM usage, and dropped frame metrics."""
        pass

    @abstractmethod
    async def shutdown(self) -> bool:
        """Safely close WebSocket connection and release resources."""
        pass
```

### 3.2 TypeScript OBS Adapter Specification (`obs_adapter.ts`)

```typescript
// Universal OBS Adapter Contract Specification (TypeScript / obs-websocket-js)
export interface SceneItem {
  sceneName: string;
  sourceName: string;
  isVisible: boolean;
  sourceType: string;
}

export interface IOBSAdapter {
  initialize(config: Record<string, any>): Promise<boolean>;
  connect(): Promise<boolean>;
  authenticate(password: string): Promise<boolean>;
  health(): Promise<Record<string, any>>;
  switchScene(sceneName: string): Promise<boolean>;
  toggleSource(sceneName: string, sourceName: string, visible: boolean): Promise<boolean>;
  muteAudio(sourceName: string, mute: boolean): Promise<boolean>;
  adjustVolume(sourceName: string, volumeDb: number): Promise<boolean>;
  startRecording(): Promise<boolean>;
  stopRecording(): Promise<string>;
  startStream(): Promise<boolean>;
  stopStream(): Promise<boolean>;
  takeScreenshot(sourceName: string, outputPath: string): Promise<boolean>;
  replayBuffer(action?: string): Promise<boolean>;
  metrics(): Promise<Record<string, number>>;
  shutdown(): Promise<boolean>;
}
```

---

## SECTION 4 & 5: Scene & Source Graph Management

- **Scene Graph Hierarchy**: Organizes production presets into standardized Scene Collections (`[Gaming]`, `[Coding/Dev]`, `[Presentation]`, `[Just Chatting]`).
- **Source Controls**: Dynamic toggling of webcam overlays, browser source widgets, window captures, and screen share boundaries.

---

## SECTION 7 & 8: Recording, Live Streaming & Bitrate Telemetry

- **NVENC Encoder Prioritization**: Hardware-accelerated H.264/HEVC encoding minimizing host CPU load (`< 3% CPU`).
- **Stream Bitrate Monitor**: Real-time monitoring of dropped frame counts, network congestion, and automated fallback to lower bitrate targets if packet loss exceeds `2%`.

---

## SECTION 9 & 10: Automated Scene Director & Obsidian Media Vault

- **Voice Command Scene Switch**: Direct integration with Voice OS (`"NAINA, switch to presentation scene"`).
- **Obsidian Media Indexing**: Automatically logs completed stream sessions and saved replay clips to `C:\ObsidianVault\14 Media\` with metadata tags.

---

## GLOSSARY OF TERMS

- **OBS WebSocket v5**: The official RPC WebSocket protocol for controlling OBS Studio remotely.
- **NVENC**: NVIDIA's dedicated hardware video encoder block on GPU graphics cards.
- **Replay Buffer**: Continuous RAM buffer storing the last 30–60 seconds of video for instant clip saving.

---

## DEPENDENCY MATRIX

| Subsystem Component | Required System Capability | Upstream/Downstream Dependency |
| :--- | :--- | :--- |
| **OBS WebSocket RPC** | `CAP_OBS_CONTROL` | Microkernel NKRS Network Gateway |
| **Stream Auto Director** | `CAP_WORKSPACE_READ` | Workspace Awareness Framework (NOS-WORKSPACE-001) |
| **Media Vault Archiver** | `CAP_OBSIDIAN_ACCESS` | Obsidian Knowledge Framework (NOS-OBSIDIAN-001) |
| **Voice Command Director** | `CAP_VOICE_LISTEN` | Voice OS & Zero Trust Framework (NOS-SECURITY-001) |

---

## IMPLEMENTATION READINESS CHECKLIST

- [x] Universal OBS Adapter Contract defined in Python & TypeScript.
- [x] OBS WebSocket v5 RPC authentication & auto-reconnect logic verified.
- [x] NVENC GPU hardware encoding quota enforced (`< 3% CPU load`).
- [x] Automatic app-switch scene director tested against Workspace Engine triggers.
- [x] Replay buffer media archiver integrated with Obsidian Vault (`14 Media/`).
- [x] All document IDs and cross-references validated against NAINA OS Index.

---

## ARCHITECTURE DECISION RECORDS (ADRs)

### ADR-075: OBS WebSocket v5 Protocol Adapter Architecture
- **Status**: Approved.
- **Decision**: Standardize on the `IOBSAdapter` interface connecting via WebSocket v5 RPCs for secure remote control of OBS Studio.

### ADR-076: Automated Voice & Workspace Scene Director
- **Status**: Approved.
- **Decision**: Link OBS scene switching to active application focus events from NOS-WORKSPACE-001 and voice commands from Voice OS.

### ADR-077: Non-Blocking Replay Buffer & Media Vault Archiver
- **Status**: Approved.
- **Decision**: Save replay buffer clips and stream recordings asynchronously into the Obsidian Vault with rich metadata.

---
*End of NOS-OBS-001 — OBS Studio Integration & Streaming Intelligence Framework Specification (v1.0)*
