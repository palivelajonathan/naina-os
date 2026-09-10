# NAINA OS — Marketplace, Package Registry & Ecosystem Framework
**Document Identifier:** NOS-MARKETPLACE-001  
**Title:** Marketplace, Package Registry & Ecosystem Framework Specification  
**Version:** 1.0  
**Status:** Approved Engineering Specification  
**Classification:** Open Source Systems Standard  
**Authors:** Chief Systems Architect, Ecosystem Group & Package Registry Team  

---

## Executive Summary (1-Page Core Architecture Overview)

The **Marketplace, Package Registry & Ecosystem Framework (NOS-MARKETPLACE-001)** defines the official package distribution, security verification, extension discovery, and ecosystem governance subsystem for NAINA OS. It enables developers and enterprise organizations to securely publish, discover, audit, install, and update plugins, agents, skills, themes, voice packs, model adapters, MCP servers, and workflow automation packs.

Key architectural highlights include:
1. **Decoupled Multi-Stage Installation Pipeline**: Packages NEVER install directly into the runtime. The installation flow follows a strict 7-stage quality gate:  
   `Marketplace -> Verification -> Static Security Scan -> Capability Validation -> Permission Review -> Runtime Installation -> Activation`.
2. **Mandatory Cryptographic Signing**: Every package artifact must be cryptographically signed using `Ed25519` key pairs and verified against trusted publisher root certificates.
3. **Canonical Package Manifest (`naina-package.json` / `naina-package.yaml`)**: Standardized package definition schema specifying dependencies, required system capabilities (`CAP_FS_READ`, `CAP_NET_CONNECT`), license, and SHA-256 integrity checksums.
4. **Enterprise Private Registries & Air-Gapped Deployments**: Support for internal corporate registries, RBAC approval workflows, and offline deployment bundles.
5. **Obsidian Integration**: Auto-generated documentation for installed marketplace packages is indexed directly into the local Obsidian Vault (`04 System/Marketplace/`).

---

## Document Revision History

| Date | Revision | Author | Description of Changes |
| :--- | :--- | :--- | :--- |
| **2026-08-08** | `1.0` | Chief Systems Architect | Initial Engineering Release of NOS-MARKETPLACE-001 Specification |
| **2026-08-05** | `0.9` | Ecosystem Group | Complete draft of Package Manifest Schema, Ed25519 Verification, and Enterprise Registry |

---

## SECTION 1: Marketplace Philosophy & Trust Mandates

### 1.1 Secure, Transparent Extension Ecosystem
Under strict NAINA OS architectural guidelines:
- **Never Direct Install**: Extensions cannot mutate the host filesystem or execute binary code without explicit permission review.
- **Zero-Trust Extensions**: Extensions run inside isolated WASM or sub-process sandboxes governed by Capability-Based Access Control (CBAC).

---

## SECTION 2: System Marketplace Architecture Topology

```mermaid
graph TD
    subgraph MarketplaceCloud [NAINA OS Central Registry / Enterprise Mirror]
        MarketplaceAPI[Marketplace Registry API]
        PkgDB[Package Database & CDN Index]
        SecScanner[Static Code & AST Security Scanner]
    end

    subgraph LocalInstaller [Local OS Package Subsystem]
        PkgManager[NAINA Package Manager (NPM/NPMX)]
        SigVerifier[Ed25519 Cryptographic Signature Verifier]
        CapValidator[Capability & Permission Reviewer]
        SandboxInstaller[Isolated Runtime Package Installer]
    end

    subgraph SystemRuntime [NAINA OS Kernel Runtime Services]
        PluginRuntime[Plugin & Extension Runtime]
        ObsidianVault[Obsidian Vault (04 System/Marketplace)]
    end

    MarketplaceAPI --> PkgDB
    PkgDB --> SecScanner
    SecScanner --> PkgManager
    PkgManager --> SigVerifier
    SigVerifier --> CapValidator
    CapValidator --> SandboxInstaller
    SandboxInstaller --> PluginRuntime
    SandboxInstaller --> ObsidianVault
```

---

## SECTION 3 & 4: Package Types & SPECIAL REQUIREMENT — Official Package Manifest

### 3.1 Package Manifest JSON Specification (`naina-package.json`)

```json
{
  "$schema": "https://naina.io/schemas/v1/naina-package.schema.json",
  "id": "io.naina.plugins.github-automation",
  "name": "GitHub Workflow Automation Skill",
  "version": "1.4.2",
  "author": {
    "name": "NAINA Engineering Core",
    "email": "dev@naina.io",
    "url": "https://naina.io"
  },
  "license": "MIT",
  "package_type": "skill",
  "description": "Autonomous GitHub Issue triage, PR code review, and CI pipeline monitoring.",
  "dependencies": {
    "naina-os-core": ">=1.0.0",
    "io.naina.mcp.git-server": "^2.1.0"
  },
  "capabilities_required": [
    "CAP_NET_CONNECT",
    "CAP_SECRET_READ"
  ],
  "permissions_requested": [
    {
      "resource": "https://api.github.com/*",
      "reason": "Fetch issues and post automated PR review comments."
    }
  ],
  "compatibility": {
    "min_os_version": "1.0.0",
    "supported_platforms": ["windows-x64", "linux-x64", "android-arm64"]
  },
  "integrity": {
    "checksum_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "signature_ed25519": "4d8a1c9e0f3b...8f7e2a1b9c"
  },
  "documentation": {
    "readme": "README.md",
    "changelog": "CHANGELOG.md"
  }
}
```

### 3.2 Package Manifest YAML Specification (`naina-package.yaml`)

```yaml
# NAINA Package Manifest Schema (YAML)
id: io.naina.plugins.github-automation
name: GitHub Workflow Automation Skill
version: 1.4.2
author:
  name: NAINA Engineering Core
  email: dev@naina.io
  url: https://naina.io
license: MIT
package_type: skill
description: Autonomous GitHub Issue triage, PR code review, and CI pipeline monitoring.
dependencies:
  naina-os-core: ">=1.0.0"
  io.naina.mcp.git-server: "^2.1.0"
capabilities_required:
  - CAP_NET_CONNECT
  - CAP_SECRET_READ
permissions_requested:
  - resource: "https://api.github.com/*"
    reason: "Fetch issues and post automated PR review comments."
compatibility:
  min_os_version: "1.0.0"
  supported_platforms:
    - windows-x64
    - linux-x64
    - android-arm64
integrity:
  checksum_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  signature_ed25519: "4d8a1c9e0f3b...8f7e2a1b9c"
documentation:
  readme: README.md
  changelog: CHANGELOG.md
```

---

## SECTION 5 & 6: Security Verification & Versioning Solver

- **Ed25519 Cryptographic Verification**: Packages are verified against the publisher's public key prior to unpacking.
- **SemVer Dependency Resolution**: Resolves package dependencies deterministically to prevent dependency confusion and version mismatch errors.

---

## SECTION 7 & 8: Enterprise Private Registry & Air-Gapped Mirrors

- **Private Registry Mirroring**: Enterprises can deploy local private mirrors (`npm.corp.naina.internal`) for air-gapped environments.
- **RBAC Package Approvals**: Requires security team sign-off before new third-party extensions can be deployed to production workstations.

---

## GLOSSARY OF TERMS

- **NPM (NAINA Package Manager)**: The CLI and runtime service for managing NAINA extensions.
- **Ed25519**: High-speed, high-security public-key signature system used for package signing.
- **CBAC**: Capability-Based Access Control enforcing granular capability boundaries.

---

## DEPENDENCY MATRIX

| Subsystem Component | Required System Capability | Upstream/Downstream Dependency |
| :--- | :--- | :--- |
| **Package Installer** | `CAP_FS_WRITE` | Plugin Runtime Specification (NOS-PLUGIN-001) |
| **Security Scanner** | `CAP_CPU_COMPUTE` | Zero Trust Security Framework (NOS-SECURITY-001) |
| **Obsidian Package Logger** | `CAP_OBSIDIAN_ACCESS` | Obsidian Knowledge Framework (NOS-OBSIDIAN-001) |
| **Enterprise Sync** | `CAP_NET_CONNECT` | Unified API & Event Bus (NOS-API-001) |

---

## IMPLEMENTATION READINESS CHECKLIST

- [x] Official Package Manifest defined in JSON & YAML.
- [x] 7-stage installation quality gate pipeline verified.
- [x] Ed25519 digital signature validation logic implemented.
- [x] SemVer dependency resolution engine tested.
- [x] Enterprise private registry mirror configuration documented.
- [x] All document IDs and cross-references validated against NAINA OS Index.

---

## ARCHITECTURE DECISION RECORDS (ADRs)

### ADR-078: Mandatory Ed25519 Package Signing & Zero-Trust Verification
- **Status**: Approved.
- **Decision**: Mandate cryptographic Ed25519 signatures for all published marketplace packages to guarantee supply chain integrity.

### ADR-079: Decoupled Multi-Stage Package Installation Pipeline
- **Status**: Approved.
- **Decision**: Enforce a strict 7-stage verification gate before allowing any extension to activate within the OS runtime.

### ADR-080: Enterprise Private Registry & Air-Gapped Mirrors
- **Status**: Approved.
- **Decision**: Provide built-in support for self-hosted private package registries and air-gapped deployment bundles.

---
*End of NOS-MARKETPLACE-001 — Marketplace, Package Registry & Ecosystem Framework Specification (v1.0)*
