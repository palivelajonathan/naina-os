# ADR-001: NAINA OS Capability-Based Access Control

- **Title:** ADR-001: NAINA OS Capability-Based Access Control (`CBAC`)
- **Status:** PROPOSED — PENDING REVIEW
- **Date:** 2026-08-21
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/capabilities`

---

## 1. Status
**PROPOSED — PENDING REVIEW**

---

## 2. Context
NAINA OS operates on Zero-Trust Security principles (`PROJECT_CHARTER.md` Section 2). Every subsystem operation (file access, desktop control, browser automation, model inference) must be gated by a cryptographically or registry-verified Capability Token (`CBAC`).

Having completed `configuration`, `logging`, and `event-bus`, the next package in `engineering/DEPENDENCY_MAP.md` is `capabilities`.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. Explicit Documented Requirements:
- **Dependency Hierarchy (`DEPENDENCY_MAP.md`)**: `capabilities` depends on `configuration` and `logging`. It is consumed by `runtime` and `services` (and `kernel`).
- **Security Principles (`PROJECT_CHARTER.md`)**: Section 2 mandates Zero-Trust Security: *"Every subsystem operation must present a cryptographically verified Capability Token (`CBAC`)."* Section 14 notes Ed25519 digital signatures and WASM sandboxing for Beta marketplace plugins.
- **Resource Constraints (`FIRST_ALPHA_SPEC.md`)**: Subsystem boot overhead `< 2.0s`, idle RAM `< 1.0 GB`. Avoid unvetted third-party crates.

### B. Requirements NOT Specified (Ambiguities resolved by this ADR):
- Exact Rust struct definition for `CapabilityToken` and `CapabilityRegistry` methods.
- Alpha vs. Beta cryptographic enforcement boundaries (in-memory registry vs Ed25519 signatures).

---

## 4. Security Problem
Subsystems in NAINA OS require permission checks to prevent unauthorized file deletion, arbitrary desktop command execution, or credential exfiltration. However, adding full Ed25519/WASM signature verification in early Alpha before local runtimes are integrated would introduce unnecessary external crypto dependencies and slow down Alpha bootstrapping.

---

## 5. Threat Model
- **Confused Deputy Attack**: A lower-privilege subsystem tricks a higher-privilege service into performing unauthorized actions.
- **Token Leakage & Replay**: Intercepted tokens reused after expiration or revocation.
- **Privilege Escalation**: Subsystem issuing unauthorized tokens to itself.

---

## 6. Architectural Decisions (ADR-001)

1. **In-Memory Token Registry for Alpha**: `CapabilityRegistry` acts as the single in-memory authority for issuing, revoking, and authorizing capability tokens.
2. **Deny-by-Default Authorization**: Any missing, expired, or revoked token causes `authorize()` to return `Err(CapabilityError::Unauthorized { .. })`.
3. **Structured Capability Tokens**: Tokens use standard string identifiers (`CAP_<SUBSYSTEM>_<ACTION>`, e.g., `CAP_DESKTOP_CONTROL`, `CAP_OBSIDIAN_READ`).
4. **Deterministic Token IDs**: Tokens are assigned monotonic IDs (`cap_tok_<seq>`) in memory without adding external UUID or crypto crates.
5. **Deferred Cryptographic Signatures (Ed25519/HMAC)**: Cryptographic signature verification is explicitly deferred to Beta when out-of-process WASM plugins enter the monorepo.

---

## 7. Capability Model
Capabilities are represented by string identifiers formatted as `CAP_<SUBSYSTEM>_<ACTION>`:
- `CAP_DESKTOP_CONTROL`: Execution of desktop automation commands.
- `CAP_OBSIDIAN_READ`: Reading local Markdown notes in the Obsidian Vault.
- `CAP_OBSIDIAN_WRITE`: Modifying local Markdown notes.
- `CAP_BROWSER_AUTOMATE`: Headless/headful browser CDP navigation.
- `CAP_MODEL_INFERENCE`: Executing local LLM prompts.

---

## 8. Token Model

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityToken {
    id: String,
    capability_id: String,
    subject: String,
    issued_at: std::time::SystemTime,
    expires_at: Option<std::time::SystemTime>,
    revoked: bool,
}
```

---

## 9. Permission Model
Permissions are checked against required capability strings (`&str`). A caller presenting a `CapabilityToken` must match the required `capability_id`.

---

## 10. Registry Model

```rust
pub struct CapabilityRegistry {
    _config: CapabilityConfig,
    tokens: std::sync::RwLock<std::collections::BTreeMap<String, CapabilityToken>>,
    next_id: std::sync::atomic::AtomicU64,
}
```

---

## 11. Authorization Model
`authorize(&self, token: &CapabilityToken, required_capability: &str) -> Result<()>`:
1. Verify `token.capability_id == required_capability`.
2. Check `expires_at`: if `SystemTime::now() > expires_at`, return `Err(CapabilityError::TokenExpired)`.
3. Check `CapabilityRegistry` lookup: if token is marked `revoked` or missing, return `Err(CapabilityError::TokenRevoked)` / `Err(CapabilityError::TokenNotFound)`.
4. On success, return `Ok(())`.

---

## 12. Revocation Model
`revoke(&self, token_id: &str) -> Result<bool>`:
- Sets `revoked = true` for the specified token ID in `CapabilityRegistry`.

---

## 13. Expiration Model
Tokens support optional expiration (`expires_at: Option<SystemTime>`).

---

## 14. Cryptographic Model
- **Alpha**: In-memory registry verification (zero external crypto dependencies).
- **Beta**: Ed25519 digital signature validation for out-of-process WASM plugins (`NOS-SECURITY-001`).

---

## 15. Error Model

```rust
#[derive(Debug)]
pub enum CapabilityError {
    Unauthorized { capability_id: String, message: String },
    TokenExpired { token_id: String },
    TokenRevoked { token_id: String },
    TokenNotFound { token_id: String },
    LockError { message: String },
}

pub type Result<T> = std::result::Result<T, CapabilityError>;
```

---

## 16. Configuration Model

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityConfig;
```

---

## 17. Logging & Audit Model
- Security events (token grants, revocations, authorization failures) are logged via `logging::Logger`.
- Plain-text credentials, tokens, or secret keys are NEVER logged.

---

## 18. Concurrency Model
- `CapabilityRegistry` uses `std::sync::RwLock` for thread-safe state access.
- Fully `Send + Sync`. Can be wrapped in `std::sync::Arc<CapabilityRegistry>`.

---

## 19. Dependency Constraints
- **Allowed**: `configuration`, `logging`.
- **Forbidden**: `event-bus`, `kernel`, `runtime`, `services`, `orchestrator`, `tool-registry`, `memory`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`.

---

## 20. Alpha Scope
1. In-memory `CapabilityRegistry` struct.
2. `grant()`, `revoke()`, `authorize()` methods.
3. Unit tests covering token issuance, validation, expiration, and revocation.

---

## 21. Deferred Capabilities (Beta / Production)
- ❌ Ed25519 / HMAC digital signature verification.
- ❌ Out-of-process WASM plugin sandbox boundaries.
- ❌ Persistent token key vault storage (`NOS-SECURITY-001`).

---

## 22. Alternatives Considered
1. **Full Ed25519 Crates in Alpha**: Rejected to prevent adding unvetted third-party crates before MVN pipeline validation.
2. **Boolean Flags**: Rejected. Structured tokens provide auditability and expiration control.

---

## 23. Risks
- *In-Memory Reset*: Registry state resets on process restart. *Mitigation*: Acceptable for Alpha; persistent vault added in Beta.

---

## 24. Consequences
- Microkernel Zero-Trust architecture enforced.
- Zero external crate dependencies added.

---

## IMPLEMENTATION CONTRACT

Exact public Rust types:
- `pub struct CapabilityRegistry`
- `pub struct CapabilityToken`
- `pub struct CapabilityConfig`
- `pub enum CapabilityError`
- `pub type Result<T> = std::result::Result<T, CapabilityError>`

Exact public methods:
- `impl CapabilityRegistry`:
  - `pub fn new(config: CapabilityConfig) -> Self`
  - `pub fn grant(&self, subject: impl Into<String>, capability_id: impl Into<String>, ttl: Option<std::time::Duration>) -> Result<CapabilityToken>`
  - `pub fn revoke(&self, token_id: &str) -> Result<bool>`
  - `pub fn authorize(&self, token: &CapabilityToken, required_capability: &str) -> Result<()>`
- `impl CapabilityToken`:
  - `pub fn id(&self) -> &str`
  - `pub fn capability_id(&self) -> &str`
  - `pub fn subject(&self) -> &str`
  - `pub fn is_expired(&self) -> bool`

Exact traits:
- None required for Alpha.

Exact token fields:
- `id: String`
- `capability_id: String`
- `subject: String`
- `issued_at: std::time::SystemTime`
- `expires_at: Option<std::time::SystemTime>`
- `revoked: bool`

Exact permission representation:
- String capability identifiers formatted as `CAP_<SUBSYSTEM>_<ACTION>`.

Exact authorization semantics:
- Deny by default. Token must match required `capability_id`, must not be expired, and must exist in `CapabilityRegistry` without `revoked == true`.

Exact error semantics:
- `CapabilityError::Unauthorized` on capability mismatch.
- `CapabilityError::TokenExpired` on expired token.
- `CapabilityError::TokenRevoked` on revoked token.
- `CapabilityError::TokenNotFound` on unrecognized token ID.

Dependencies:
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`

Files to implement:
- `packages/capabilities/src/types.rs`
- `packages/capabilities/src/error.rs`
- `packages/capabilities/src/config.rs`
- `packages/capabilities/src/capability_registry.rs`
- `packages/capabilities/src/lib.rs`
- `packages/capabilities/tests/cbac.rs`

Security guarantees:
- Strict in-memory deny-by-default capability permission enforcement.
- Monotonic token ID generation.
- Expiration and explicit revocation checking.

Security guarantees NOT provided:
- Out-of-process cryptographic signature verification (deferred to Beta).
- Persistent token storage across process restarts.

Open questions:
1. Whether capability delegation (re-granting derived tokens) is required in Beta.

Implementation permitted:
NO
