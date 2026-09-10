# NAINA OS — First Alpha Engineering Specification & MVP Blueprint
**Document Identifier:** ALPHA-SPEC-001  
**Title:** NAINA OS First Alpha Specification & Minimum Viable NAINA (MVN) Definition  
**Version:** 1.0  
**Status:** ACTIVE -- Authoritative Alpha Scope Definition  
**Target Path:** `C:\naina-os\engineering\FIRST_ALPHA_SPEC.md`  
**Authors:** Chief Systems Architect, Lead Platform Engineer, Security Lead & Core Engineering Group  

---

## 1. Executive Summary & Purpose

This specification defines the **Minimum Viable NAINA (MVN)** and establishes the exact acceptance criteria required to ship **NAINA OS Alpha (v0.8.0)**.

While previous specifications outlined total system capabilities across 35 subsystems, this document answers one single operational question:

> *"What exactly must work end-to-end before we tag and release NAINA OS Alpha?"*

By establishing strict resource budgets, quantitative latency targets, explicit non-goals, weekly milestones, and a 10-point Alpha Acceptance Test, this specification prevents scope creep and guarantees a fast, working prototype.

---

## 2. Minimum Viable NAINA (MVN) Core Pipeline

The core execution path for Alpha is strictly defined as a single end-to-end cognitive loop:

```
                    MINIMUM VIABLE NAINA (MVN) CORE PIPELINE
  ┌─────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
  │ Voice Input │ ──> │ CARF Planner │ ──> │ Qwen GGUF    │ ──> │ Obsidian     │
  │ (Whisper)   │     │ (Intent Routing)  │ (ARAL Model) │     │ Memory Vault │
  └─────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
                                                                       │
  ┌─────────────┐     ┌──────────────┐                                 ▼
  │ Voice Output│ <── │ Desktop      │ <───────────────────────────────┘
  │ (Piper TTS) │     │ Control      │
  └─────────────┘     └──────────────┘
```

If this end-to-end loop executes cleanly with zero process crashes, **NAINA OS Alpha is functional**.

---

## 3. Engineering Success Metrics & Performance Targets

| Metric Domain | Target Parameter | Maximum Allowed Threshold | Verification Tool / Benchmark |
| :--- | :--- | :---: | :--- |
| **Voice Processing** | Voice-to-Voice Latency | **`< 700 ms`** | `tests/benchmarks/test_voice_latency.py` |
| **Memory Retrieval** | Hybrid Vector/BM25 Search | **`< 300 ms`** | `tests/benchmarks/test_memory_speed.py` |
| **Desktop Control** | Command Execution Latency | **`< 500 ms`** | `tests/benchmarks/test_desktop_exec.py` |
| **Plugin Subsystem** | WASM Plugin Load Time | **`< 100 ms`** | `tests/benchmarks/test_plugin_boot.py` |
| **System Cold Boot** | Microkernel NKRS Startup | **`< 2.0 s`** | `tests/benchmarks/test_cold_boot.py` |

---

## 4. Hardware Resource Budget Constraints

All Alpha components MUST fit strictly within the following resource allocations on the target developer workstation (16GB RAM, 6-Core CPU, GTX 1660 / RTX 3060 6GB VRAM):

| System State | Metric | Resource Budget |
| :--- | :--- | :---: |
| **System Idle State** | Total RAM Consumption | **`< 1.0 GB`** |
| **System Idle State** | Background CPU Usage | **`< 5.0 %`** |
| **System Idle State** | GPU VRAM Consumption | **`0.0 MB (Offloaded)`** |
| **Active Pipeline** | Voice Processing RAM | **`< 1.0 GB`** |
| **Active Pipeline** | Desktop Overlay Host RAM | **`< 200 MB`** |
| **Active Pipeline** | GPU VRAM Peak (Qwen 7B GGUF) | **`< 4.8 GB`** |

---

## 5. Explicit Non-Goals ("What We WON'T Build" in Alpha)

To ensure shipping velocity, the following features are **EXPLICITLY EXCLUDED** from the Alpha release:

- ❌ **NO Cloud Synchronization (NSP)**: Cloud P2P sync is strictly deferred to Beta (`NOS-CLOUD-001`).
- ❌ **NO Physical Robotics / Hardware HAL**: ROS 2 real-time hardware drivers deferred to Beta (`NOS-ROBOTICS-001`).
- ❌ **NO Ecosystem Marketplace**: Package registry installer deferred to Beta (`NOS-MARKETPLACE-001`).
- ❌ **NO Multi-Tenant / Enterprise Auth**: Enterprise LDAP/OAuth multi-user support deferred to Beta.
- ❌ **NO VR / AR Spatial Interfaces**: Immersive headsets deferred to post-1.0 releases.

---

## 6. Optimized 12-Week Milestone Tracker (Inverted Architecture Model)

*Note: In accordance with optimal prototype velocity, Model Runtime integration is inverted to occur BEFORE Voice OS and Memory Vault, allowing early validation of adapters, logging, and IPC channels.*

```
Week 1: Monorepo Setup, Pnpm/Cargo Workspace & Shared Types (@naina/types)
  ↓
Week 2: Microkernel NKRS Process Supervisor & Zero-Trust Security (CBAC)
  ↓
Week 3: ARAL Model Runtime & Qwen 7B GGUF Adapter (Early Prompt Execution)
  ↓
Week 4: Voice OS Pipeline (Whisper STT + Piper TTS Integration)
  ↓
Week 5: Memory Engine & Obsidian Vault Indexer (Vector + BM25 Hybrid)
  ↓
Week 6: Windows Desktop Overlay Host & System Control API
  ↓
Week 7: Browser CDP Adapter & Semantic DOM Inspector
  ↓
Week 8: Workflow Automation Engine & CARF Planner Integration
  ↓
Week 9: End-to-End Minimum Viable NAINA (MVN) Integration Testing
  ↓
Week 10: Performance Optimization & Latency Tuning (<700ms Voice Goal)
  ↓
Week 11: Security Audit & Capability Token Permission Verification
  ↓
Week 12: NAINA OS Alpha Release (v0.8.0) Verification & Tagging
```

---

## 7. Alpha Acceptance Test Checklist

The **NAINA OS Alpha (v0.8.0)** build is officially accepted ONLY when all 10 checklist criteria pass:

- [ ] **1. Cold Boot Success**: Microkernel NKRS initializes and registers services in `< 2.0s`.
- [ ] **2. Local Model Execution**: Loads Qwen 7B GGUF model via `IModelAdapter` and streams response tokens.
- [ ] **3. Voice Pipeline Functional**: Captures microphone input, converts via Whisper STT, and speaks output via Piper TTS.
- [ ] **4. Voice Output Latency**: Total voice-to-voice turn latency measures `< 700ms`.
- [ ] **5. Windows App Launching**: Launches Windows applications (e.g., Notepad, VS Code) via desktop control.
- [ ] **6. Browser Control**: Opens browser tabs, navigates URLs, and extracts page text via `IBrowserAdapter`.
- [ ] **7. Obsidian Memory Access**: Reads and searches local Markdown notes in `< 300ms` without file corruption.
- [ ] **8. Automation Workflow**: Executes 1 complete multi-step automation workflow (e.g., summarize webpage -> save note to Obsidian).
- [ ] **9. Context Preservation**: Retains multi-turn conversation memory across at least 5 user turns.
- [ ] **10. Graceful Crash Recovery**: Automatically restarts failed worker processes without crashing the NKRS supervisor.

---
*End of NAINA OS First Alpha Specification (FIRST_ALPHA_SPEC.md v1.0)*
