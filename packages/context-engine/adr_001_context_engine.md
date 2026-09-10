# ADR-001: NAINA OS Context Engine Architecture

- **Title:** ADR-001: NAINA OS Context Engine Architecture
- **Status:** APPROVED
- **Date:** 2026-08-22
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/context-engine`

---

## 1. Status
**APPROVED**

*Implementation Permitted: YES*

---

## 2. Context
NAINA OS operates on a microkernel architecture (`PROJECT_CHARTER.md` Section 3). Lower-level packages (`configuration`, `logging`, `event-bus`, `capabilities`, `kernel`, `runtime`, `services`, `tool-registry`, `memory`) are fully implemented and verified. `context-engine` is the Engine Layer package providing multi-turn conversation context management, context window token budgeting, memory retrieval integration, and prompt context assembly for agent planners and orchestrators (`DEPENDENCY_MAP.md`).

Having completed `memory` and initialized `packages/context-engine`, an Architecture Decision Record is required to specify `ContextEngine` data models, pruning algorithms, and context assembly contracts before implementation.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **Dependency Matrix (`DEPENDENCY_MAP.md`)**: `context-engine` depends ONLY on `memory`, `configuration`, and `logging`. Used by `orchestrator`. Public API: `ContextEngine`.
- **First Alpha Specification (`FIRST_ALPHA_SPEC.md`)**:
  - Section 7 (Acceptance Checklist Item 9): Retains multi-turn conversation memory across at least 5 user turns.
  - Section 4 (Workstation Budget): System idle state total RAM consumption `< 1.0 GB`.
- **Root Configuration (`packages/configuration/src/models.rs`)**:
  - `configuration::ModelConfig { pub provider: String, pub model_name: String, pub context_window: usize }` (default context window 2048 tokens).
- **Zero-Trust Security & Microkernel Rules (`PROJECT_CHARTER.md` & `PACKAGE_RULES.md`)**:
  - No direct Cargo dependencies on `runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, or `tool-registry`.
  - Context operations must be memory-safe, thread-safe, and deterministic.

### B. ARCHITECTURAL DECISIONS PROPOSED BY THIS ADR:
- Struct definitions for `ContextEngine`, `ContextEngineConfig`, `ConversationId(pub u64)`, `ContextTurn`, `ContextWindow`.
- Enum definitions for `Role` (`System`, `User`, `Assistant`, `Tool`).
- Pure Rust deterministic token estimation rule (~4 UTF-8 characters per estimated token for Alpha approximation).
- Sliding-window context pruning algorithm that retains at least 5 user turns while enforcing `max_tokens` context limits.
- Integration contract querying `memory::MemoryStore` using the latest User turn or explicit `query_override`.
- Error handling model via `ContextEngineError` and `Result<T>`.

---

## 4. Problem Statement
The primary engineering documents mandate that `context-engine` manages multi-turn conversation history across at least 5 user turns within the configured model context window (default 2048 tokens), but do not specify internal data models, token estimation formulas, pruning rules, or memory retrieval integration contracts. An ADR is required to establish a clean, pure-Rust, lightweight Alpha architecture without external tokenizer crate dependencies.

---

## 5. Architectural Decisions (ADR-001)

### 1. ContextEngine Internal Architecture
`ContextEngine` is an in-memory thread-safe context manager storing active conversation histories and referencing `memory::MemoryStore`.
```rust
pub struct ContextEngine {
    config: ContextEngineConfig,
    memory: std::sync::Arc<memory::MemoryStore>,
    conversations: std::sync::RwLock<std::collections::BTreeMap<ConversationId, Vec<ContextTurn>>>,
    next_conversation_id: std::sync::atomic::AtomicU64,
}
```

### 2. Context Engine Data Model
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextTurn {
    pub id: u64,
    pub role: Role,
    pub content: String,
    pub token_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContextWindow {
    pub turns: Vec<ContextTurn>,
    pub retrieved_memories: Vec<memory::SearchResult>,
    pub total_tokens: usize,
}
```

### 3. ContextEngineConfig Defaults & Rationale
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextEngineConfig {
    pub max_turns: usize,
    pub max_tokens: usize,
    pub memory_retrieval_limit: usize,
}

impl Default for ContextEngineConfig {
    fn default() -> Self {
        Self {
            max_turns: 20,
            max_tokens: 2048,
            memory_retrieval_limit: 5,
        }
    }
}
```
- `max_tokens`: Inherits default 2048 tokens from `configuration::ModelConfig.context_window`.
- `max_turns`: Default 20 turns guarantees retaining at least 5 user turns (User + Assistant pairs).
- `memory_retrieval_limit`: Default 5 top relevant memory notes attached during prompt assembly.

### 4. Pure Rust Token Estimation (Alpha Approximation)
- **Alpha Rule**: `token_count = (utf8_char_count + 3) / 4`.
- **Rationale**: External C/Python tokenizer crates (`tiktoken`, `tokenizers`) introduce heavy binary footprint, C++ toolchain dependencies, and native binding overhead. A pure-Rust ~4 chars/token approximation provides fast, deterministic token budgeting within `< 1ms`.
- **Constraint**: Explicitly treated as an Alpha approximation, NOT equivalent to a full LLM BPE tokenizer.

### 5. Deterministic Token Pruning Strategy
1. Always retain the `System` turn (if present).
2. Retain the most recent conversation turns working backward from the latest turn.
3. **5-User-Turn Preservation**: The pruning algorithm ensures at least 5 `User` turns (and corresponding `Assistant` responses) remain in the context window.
4. If total tokens exceed `max_tokens`, older non-system turns are pruned. If `max_tokens` is configured too low to hold 5 user turns, return `Err(ContextEngineError::TokenLimitExceeded)`.

### 6. MemoryStore Retrieval Integration
- `assemble_context` queries `memory::MemoryStore` using either an explicit `query_override` or the text of the latest `User` turn.
- Passes `memory::QueryFilter { limit: config.memory_retrieval_limit, .. }`.
- Returned `memory::SearchResult` items are included in `ContextWindow.retrieved_memories`.

### 7. Deterministic Context Assembly Order
`ContextWindow` orders elements deterministically:
1. **System Turn** (role: `Role::System`)
2. **Retrieved Long-Term Memories** (from `MemoryStore`)
3. **Retained Multi-Turn Conversation History** (ordered chronologically)

### 8. Concurrency & Synchronization
- Synchronized using standard library `std::sync::RwLock` and `AtomicU64`.
- Supports multi-threaded concurrent searches and turn additions (`Send + Sync`, `Arc<ContextEngine>`).
- Zero unsafe code.

### 9. Performance Target (ADR Target / Engineering Assumption)
- **ADR Target**: In-memory context assembly, turn pruning, and memory store retrieval execute in **`< 15 ms`**.
- *Note*: This performance target is an engineering assumption for context assembly, distinct from the source-supported memory search budget (`< 300ms`).

---

## 6. Error Model (`ContextEngineError`)

```rust
#[derive(Debug)]
pub enum ContextEngineError {
    ConversationNotFound { id: ConversationId },
    TokenLimitExceeded { limit: usize, requested: usize },
    ConfigError { message: String },
    LockError { message: String },
    Memory(memory::MemoryError),
    Configuration(configuration::ConfigError),
}
```

---

## 7. Security & Dependency DAG Compliance
- **Direct Dependencies**: `memory`, `configuration`, and `logging` ONLY.
- **Forbidden Dependencies**: `runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, `tool-registry`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`.
- **Privacy Rules**: No sensitive conversation data or capability credentials logged.

---

## 8. Alpha Scope & Deferred Features

### Alpha Scope:
- In-memory conversation state management (`ContextEngine`).
- Monotonic deterministic `ConversationId` generation.
- Pure Rust character-ratio token estimation (~4 chars/token).
- Multi-turn context pruning preserving at least 5 user turns.
- `MemoryStore` search retrieval integration.
- Deterministic prompt context assembly.
- Thread-safe API (`create_conversation`, `add_turn`, `get_history`, `assemble_context`, `clear_conversation`).

### Explicitly Deferred to Beta:
- ❌ External BPE tokenizer crates (`tiktoken`, `tokenizers`).
- ❌ Neural LLM context summarization workers.
- ❌ Persistent SQL conversation database storage.
- ❌ Remote cloud context synchronization.

---

## 9. Architecture Risks & Consequences

### Positive Consequences:
- 100% pure Rust implementation with zero external crates.
- Strictly complies with DAG rules (`memory` + `configuration` + `logging` -> `context-engine`).
- Guarantees retaining 5 user turns for Alpha Acceptance Test Item 9.

### Architecture Risks & Mitigation:
- *Token Estimation Divergence*: Pure Rust ~4 chars/token rule may under/over-estimate model BPE tokens. *Mitigation*: Include a 10% safety margin in token budgeting.
- *Unbounded Conversation Storage*: Retaining active conversations indefinitely in memory. *Mitigation*: Provide `clear_conversation()` to release memory state.

---

## 10. Open Architectural Questions
*None — All struct fields, method signatures, token budgeting rules, pruning algorithms, and error models are fully specified.*

---

## IMPLEMENTATION CONTRACT

### Public Structs:
- `pub struct ContextEngine`
- `pub struct ContextEngineConfig`
- `pub struct ConversationId(pub u64)`
- `pub struct ContextTurn`
- `pub struct ContextWindow`

### Public Enums:
- `pub enum Role`
- `pub enum ContextEngineError`

### Public Type Aliases:
- `pub type Result<T> = std::result::Result<T, ContextEngineError>`

### Exact Method Signatures:
- `impl ContextEngine`:
  - `pub fn new(config: ContextEngineConfig, memory: std::sync::Arc<memory::MemoryStore>) -> Self`
  - `pub fn from_root_config(root_config: &configuration::Config, memory: std::sync::Arc<memory::MemoryStore>) -> Self`
  - `pub fn create_conversation(&self) -> Result<ConversationId>`
  - `pub fn add_turn(&self, conversation_id: ConversationId, role: Role, content: impl Into<String>) -> Result<ContextTurn>`
  - `pub fn get_history(&self, conversation_id: ConversationId) -> Result<Vec<ContextTurn>>`
  - `pub fn assemble_context(&self, conversation_id: ConversationId, query_override: Option<&str>) -> Result<ContextWindow>`
  - `pub fn clear_conversation(&self, conversation_id: ConversationId) -> Result<()>`

### Dependencies:
- `memory = { path = "../memory" }`
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`

### Forbidden Dependencies:
- `runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, `tool-registry`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`

Status: APPROVED  
Implementation permitted: YES
