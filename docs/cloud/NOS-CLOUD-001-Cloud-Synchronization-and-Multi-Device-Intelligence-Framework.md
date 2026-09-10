# NAINA OS — Cloud, Synchronization & Multi-Device Intelligence Framework
**Document Identifier:** NOS-CLOUD-001  
**Title:** Cloud, Synchronization & Multi-Device Intelligence Framework Specification  
**Version:** 1.0  
**Status:** Approved Engineering Specification  
**Classification:** Open Source Systems Standard  
**Authors:** Chief Systems Architect, Cloud Engineering Group & Distributed Sync Team  

---

## Document Revision History

| Date | Revision | Author | Description of Changes |
| :--- | :--- | :--- | :--- |
| **2026-08-08** | `1.0` | Chief Systems Architect | Initial Engineering Release of NOS-CLOUD-001 Specification |
| **2026-08-05** | `0.9` | Distributed Sync Team | Complete draft of NAINA Sync Protocol (NSP), E2EE Key Derivation, and CRDT Resolver |

---

## SECTION 1: Cloud Philosophy & Offline-First Guarantees

### 1.1 Cloud Optionality & Local-First Mandates
The **Cloud, Synchronization & Multi-Device Intelligence Framework** defines how NAINA OS synchronizes knowledge, memories, workflows, device states, and user preferences across trusted endpoints.

Under strict NAINA OS architectural guidelines:
- **Cloud is OPTIONAL**: NAINA OS is a local-first system. The OS must function at 100% capability without an active internet connection or cloud service.
- **Local Data Authority**: Local storage (Obsidian Vault & SQLite/pgvector) is ALWAYS authoritative. The cloud is strictly a zero-knowledge sync relay.
- **End-to-End Encryption (E2EE)**: All data synchronized to external providers is encrypted client-side using `AES-256-GCM` before leaving the local host.

```
[Local Endpoint] ──> [E2EE Encryption Engine] ──> [NSP Transport] ──> [Cloud Provider Adapter]
       │                                                                      │
  Authoritative                                                          Zero-Knowledge
  Local Storage                                                            Sync Relay
```

---

## SECTION 2: System Cloud & Sync Architecture Topology

```mermaid
graph TD
    subgraph LocalDevice [Local Desktop / Companion Device]
        LocalVault[Obsidian Vault & Local Storage]
        SyncRuntime[NSP Sync Runtime Engine]
        CRDTResolver[Vector Clock & CRDT Conflict Resolver]
        CryptoEngine[E2EE Encryption Engine (AES-256-GCM)]
    end

    subgraph CloudAdapters [Pluggable Provider Adapters]
        LocalOnly[Local-Only / P2P Mesh Mode]
        OneDriveAdapter[OneDrive Provider Adapter]
        GDriveAdapter[Google Drive Provider Adapter]
        S3Adapter[Self-Hosted S3 / MinIO Adapter]
    end

    LocalVault <--> SyncRuntime
    SyncRuntime <--> CRDTResolver
    SyncRuntime <--> CryptoEngine
    CryptoEngine <--> LocalOnly
    CryptoEngine <--> OneDriveAdapter
    CryptoEngine <--> GDriveAdapter
    CryptoEngine <--> S3Adapter
```

---

## SECTION 3 & 4: Device Registry & Synchronization Engine

- **Device Pairing Protocol**: Devices are paired via cryptographically signed QR codes exchanging public keys (`Ed25519`).
- **Synchronized Data Domains**: Obsidian Vault Markdown, WDL Workflows, Memory Vectors, User Preferences, Agent Capabilities, and Workspace Session Snapshots.

---

## SECTION 5: Conflict Resolution Engine (CRDT & Vector Clocks)

- **State-based CRDTs**: Uses LWW-Element-Set (Last-Write-Wins) for configuration toggles and Conflict-free Replicated Data Types (CRDTs) for concurrent text edits in Markdown files.
- **Vector Clocks**: Maintains logical clock vector `V(d) = [d1: v1, d2: v2]` per file chunk to detect simultaneous offline edits. Unresolvable textual diffs trigger a non-destructive side-by-side branch file (`note.conflict-device2.md`).

---

## SECTION 6 & 7: Cloud Provider Adapters & E2EE Encryption

- **Supported Adapters**: Local-only P2P, OneDrive Graph API, Google Drive REST v3, Dropbox API v2, and S3-Compatible Storage (AWS, MinIO, Cloudflare R2).
- **Key Rotation**: Master key derived locally via `Argon2id` using user passphrase + device salt.

---

## SECTION 8: SPECIAL REQUIREMENT — NAINA Sync Protocol (NSP) Specification

The **NAINA Sync Protocol (NSP)** governs frame transport, version handshake, delta compression, and encrypted synchronization between devices:

```python
# NAINA Sync Protocol (NSP) Frame Specification & Engine (Python)
from pydantic import BaseModel
from typing import Dict, Any, Optional
import enum

class NSPFrameType(str, enum.Enum):
    HANDSHAKE_REQ = "handshake_request"
    HANDSHAKE_RESP = "handshake_response"
    DELTA_SYNC = "delta_sync"
    ACKNOWLEDGE = "acknowledge"
    CONFLICT_NOTIFY = "conflict_notify"

class NSPFrame(BaseModel):
    nsp_version: str = "1.0"
    frame_type: NSPFrameType
    sender_device_id: str
    target_device_id: str
    sequence_number: int
    vector_clock: Dict[str, int]
    compression: str = "zstd"  # zstd compressed payload
    encryption: str = "AES-256-GCM"
    payload_ciphertext_b64: str
    auth_tag_b64: str

class INSPTransport(BaseModel):
    async def send_frame(self, frame: NSPFrame) -> bool:
        """Transmit encrypted NSP frame over TCP socket or WebRTC data channel."""
        # Frame Serialization & Transmission logic
        return True
```

---

## SECTION 9 & 10: Performance & Security

- **Delta Sync Optimization**: Uses `zstd` binary diffing to transmit only modified byte ranges (averaging `< 5 KB` per sync frame).
- **Zero-Trust Controls**: Replay protection with monotonically increasing frame sequence numbers and anti-tampering HMAC tags.

---

## ARCHITECTURE DECISION RECORDS (ADRs)

### ADR-069: NAINA Sync Protocol (NSP) E2EE Frame Standard
- **Status**: Approved.
- **Decision**: Standardize on NSP binary JSON frames encrypted via client-side AES-256-GCM for all multi-device synchronization.

### ADR-070: Local-First CRDT & Vector Clock Conflict Resolution
- **Status**: Approved.
- **Decision**: Enforce vector clock tracking and LWW-Element-Set CRDTs for seamless conflict resolution without requiring a central server.

### ADR-071: Pluggable Cloud Storage Provider Adapter Architecture
- **Status**: Approved.
- **Decision**: Decouple sync logic from cloud backends using an abstract `ICloudStorageAdapter` interface supporting OneDrive, GDrive, Dropbox, and MinIO.

---
*End of NOS-CLOUD-001 — Cloud, Synchronization & Multi-Device Intelligence Framework Specification (v1.0)*
