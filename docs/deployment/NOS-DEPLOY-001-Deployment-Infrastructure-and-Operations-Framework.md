# NAINA OS — Deployment, Infrastructure & Operations Framework
**Document Identifier:** NOS-DEPLOY-001  
**Title:** Deployment, Infrastructure & Operations Framework Architecture Specification  
**Version:** 1.0  
**Status:** Approved Engineering Specification  
**Classification:** Open Source Systems Standard  
**Authors:** Chief Systems Architect, DevOps Leads & Operations Engineering Group  

---

## Document Revision History

| Date | Revision | Author | Description of Changes |
| :--- | :--- | :--- | :--- |
| **2026-07-31** | `1.0` | Chief Systems Architect | Initial Engineering Release of NOS-DEPLOY-001 Specification |
| **2026-07-30** | `0.9` | DevOps Operations Group | Complete draft of Docker Compose Stack, Update Engine, and Disaster Recovery |

---

## SECTION 1: Deployment Philosophy & Operational Mandates

### 1.1 Local-First & Reproducible Operations
The **Deployment, Infrastructure & Operations Framework** defines how NAINA OS is packaged, installed, updated, containerized, self-hosted, and restored.

Under strict NAINA OS architectural guidelines:
- **Deployment is NOT only Docker**. It encompasses native Windows installers, portable USB setups, mobile APK sideloads, and automated updates.
- **Local-First & Offline-First**: NAINA OS must remain fully operational without external internet connectivity.
- **Reproducible Infrastructure**: All system services (pgvector, ChromaDB, Ollama, Event Bus) are specified declaratively via Docker Compose and Infrastructure-as-Code (IaC).

```
Developer Code Push ──> GitHub Actions CI/CD ──> Build & Signing Pipeline
                                                         │
[Cloud / K8s Cluster] <── [Android Sideload / Play] <── [Windows MSI / Portable]
```

---

## SECTION 2: System Deployment Architecture Topology

```mermaid
graph TD
    subgraph HostOS [Local Workstation Host OS (Windows 11 / POSIX)]
        WinApp[NAINA Desktop App Host]
        OllamaDaemon[Ollama Local GPU Service]
    end

    subgraph ContainerStack [Containerized Microservices - Docker Compose]
        PGVector[PostgreSQL 16 + pgvector Database]
        ChromaDB[ChromaDB Vector Store]
        RedisCache[Redis Event Cache & PubSub]
        NainaKernel[NKRS Microkernel Daemon]
    end

    subgraph MobileDevice [Mobile Companion Device]
        AndroidAPK[NAINA Android Companion App]
    end

    WinApp <-->|Local IPC Socket| NainaKernel
    OllamaDaemon <-->|HTTP / Port 11434| NainaKernel
    NainaKernel <--> PGVector
    NainaKernel <--> ChromaDB
    NainaKernel <--> RedisCache
    AndroidAPK <-->|mTLS / TLS 1.3 WebSocket| NainaKernel
```

---

## SECTION 3 & 4: Environment Types & Installation Subsystems

### 3.1 Environment Classifications
1. **Development**: Hot-reloading Vite frontend, local Python debuggers, fake AI mocks.
2. **Production**: Bundled Tauri v2 desktop binary, production Docker Compose microservices, optimized TensorRT AI models.
3. **Offline Air-Gapped**: Fully self-contained local models (Qwen 14B Q4) and offline Obsidian sync.
4. **Portable USB**: Executable directly from high-speed USB 3.2 flash storage without host Windows registry modification.

### 3.2 Windows & Mobile Installation
- **Windows MSI Installer**: Inno Setup script registering startup shortcuts, background services, and GPU drivers.
- **Silent CLI Installation**: `naina install --silent --with-gpu --model qwen2.5:14b`.
- **Android APK Sideloading**: Direct APK download and Play Store deployment.

---

## SECTION 5: Configuration Management Hierarchy

NAINA OS merges configuration layers using a strict priority cascade:

```
1. CLI Overrides (--port 8443 --gpu-vram 12GB)
       ▲
2. Environment Variables (NAINA_MODEL_DIR="D:\models")
       ▲
3. User Configuration (C:\Users\jonat\.naina\naina.config.yaml)
       ▲
4. System Defaults (Default embedded YAML configuration)
```

---

## SECTION 6: Docker Containerization Architecture

```yaml
# NAINA OS Docker Compose Production Specification (docker-compose.yml)
version: '3.8'

services:
  pgvector-db:
    image: pgvector/pgvector:pg16
    container_name: naina-pgvector
    environment:
      POSTGRES_DB: naina_memory
      POSTGRES_USER: naina_user
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    restart: always

  chroma-vector:
    image: chromadb/chroma:latest
    container_name: naina-chroma
    volumes:
      - chromadata:/chroma/chroma
    ports:
      - "8000:8000"
    restart: always

  naina-kernel:
    image: nainaos/kernel:latest
    container_name: naina-kernel-core
    environment:
      - NAINA_ENV=production
    volumes:
      - C:\Users\jonat\.naina\obsidian:/app/obsidian_vault
    ports:
      - "8080:8080"
      - "50051:50051"
    depends_on:
      - pgvector-db
      - chroma-vector
    restart: always

volumes:
  pgdata:
  chromadata:
```

---

## SECTION 7 & 8: Core Infrastructure & Update Engine

- **Database Stack**: PostgreSQL 16 (`pgvector` for HNSW vector search) + ChromaDB for fast document chunk retrieval.
- **Automatic Background Updates**: NAINA OS monitors release feeds on GitHub. Updates are downloaded in the background, verified via SHA-256 and GPG signatures, and applied via a transactional delta patcher.
- **One-Click Rollback**: If an update fails health checks (`/health` SLA timeout), the update engine automatically reverts to the previous git commit / binary snapshot.

---

## SECTION 9 & 10: Monitoring, Backup & Disaster Recovery (DR)

### 1. Observability Stack
- **Prometheus Metrics**: Exposes metrics on `/metrics` (CPU, RAM, GPU VRAM %, Task queue throughput, Latency).
- **Grafana Dashboards**: Real-time system monitoring dashboard.

### 2. Disaster Recovery Protocol (DR)
- **RTO (Recovery Time Objective)**: `< 15 minutes` to restore full system from scratch.
- **RPO (Recovery Point Objective)**: `< 1 hour` maximum data loss via automated hourly Obsidian Vault & Vector DB snapshots.

```
Disaster Event ──> Detect Database Corruption ──> Stop Microservices
                                                         │
System Operational <── Run Atomic Database Restore <─────┘
```

---

## SECTION 11: CI/CD Pipeline Automation

```yaml
# GitHub Actions Production Build Pipeline (.github/workflows/build.yml)
name: NAINA OS CI/CD Build & Test

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  build-and-test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - name: Set up Python 3.11
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
      - name: Install Dependencies
        run: pip install -r requirements.txt pytest
      - name: Run Pytest Suite
        run: pytest tests/
      - name: Build Windows Executable (Tauri / Inno)
        run: npm run tauri build
```

---

## SECTION 12 & 13: Performance Benchmarks & Infrastructure Security

- **Startup Performance**: Desktop host cold boot to ready state in `< 2.1 seconds`.
- **Infrastructure Security**: Code signing for Windows MSI installers, secret rotation for database passwords, and SBOM (Software Bill of Materials) supply chain audits.

---

## SECTION 14 & 15: Testing Matrix & Future Cloud Roadmap

- **Deployment Testing**: Verifies clean installs on fresh Windows 11 VMs and Android 14 devices.
- **Future Kubernetes Roadmap**: Declarative Helm charts (`naina-helm`) for multi-tenant enterprise deployments and cloud agent clusters.

---

## ARCHITECTURE DECISION RECORDS (ADRs)

### ADR-033: Docker Compose for Self-Hosted Infrastructure Stack
- **Status**: Approved.
- **Decision**: Standardize on Docker Compose for local microservice orchestration (pgvector, ChromaDB, Redis) to guarantee 100% environment reproducibility.

### ADR-034: Atomic Transactional Update & Rollback Engine
- **Status**: Approved.
- **Decision**: Binary and database schema updates must execute transactionally with automatic fallback rollback triggers on health check failures.

### ADR-035: Offsite Encrypted Backup Strategy for Obsidian Vault & Vector DB
- **Status**: Approved.
- **Decision**: Backups are encrypted locally using AES-256-GCM before being synchronized to user-selected storage (Obsidian sync, local drive, or S3).

---
*End of NOS-DEPLOY-001 — Deployment, Infrastructure & Operations Framework Specification (v1.0)*
