# ADR-001: NAINA OS Memory Subsystem Architecture

- **Title:** ADR-001: NAINA OS Memory Subsystem Architecture
- **Status:** APPROVED
- **Date:** 2026-08-22
- **Author:** NAINA OS Core Systems Engineering
- **Target Component:** `packages/memory`

---

## 1. Status
**APPROVED**

*Implementation Permitted: YES*

---

## 2. Context
NAINA OS operates on a microkernel architecture (`PROJECT_CHARTER.md` Section 3). Lower-level packages (`configuration`, `logging`, `event-bus`, `capabilities`, `kernel`, `runtime`, `services`, `tool-registry`) are fully implemented and verified. `memory` is the Engine/Storage package providing persistent memory storage, document indexing, and hybrid vector/BM25 retrieval for context engines and orchestrators (`DEPENDENCY_MAP.md`).

Having completed `tool-registry` and initialized `packages/memory`, an Architecture Decision Record is required to specify `MemoryStore` data models, indexing algorithms, and performance contracts before implementation.

---

## 3. Existing Documented Requirements (Source of Truth)

### A. SOURCE-SUPPORTED REQUIREMENTS:
- **Dependency Matrix (`DEPENDENCY_MAP.md`)**: `memory` depends ONLY on `configuration` and `logging`. Used by `orchestrator` and `context-engine`. Public API: `MemoryStore`.
- **First Alpha Specification (`FIRST_ALPHA_SPEC.md`)**:
  - Section 2 (Core Pipeline): "Obsidian Memory Vault" handles local note retrieval.
  - Section 3 (Performance Targets): Memory Retrieval (Hybrid Vector/BM25 Search) latency MUST be **`< 300 ms`**.
  - Section 6 (Milestone Tracker): Week 5 - Memory Engine & Obsidian Vault Indexer (Vector + BM25 Hybrid).
  - Section 7 (Acceptance Checklist Item 7): Reads and searches local Markdown notes in `< 300ms` without file corruption.
- **Root Configuration (`packages/configuration/src/models.rs`)**:
  - `configuration::MemoryConfig { pub enabled: bool, pub storage_path: String }`.
- **Zero-Trust Security & Microkernel Rules (`PROJECT_CHARTER.md` & `PACKAGE_RULES.md`)**:
  - No direct Cargo dependencies on `runtime`, `kernel`, `capabilities`, `services`, or `orchestrator`.
  - Memory operations must be memory-safe, thread-safe, and deterministic.

### B. ARCHITECTURAL DECISIONS PROPOSED BY THIS ADR:
- Struct definitions for `MemoryStore`, `MemoryConfig`, `MemoryId(pub u64)`, `MemoryEntry`, `QueryFilter`, `SearchResult`, `SearchMatchType`.
- In-memory pure Rust BM25 inverted keyword index combined with lightweight sparse TF-IDF vector similarity.
- Hybrid ranking score calculation: `Score = 0.5 * Normalized_BM25 + 0.5 * Normalized_SparseVector`.
- Read-only vault indexer parsing `.md` files without modifying target Markdown documents.
- Error handling model via `MemoryError` and `Result<T>`.

---

## 4. Problem Statement
The primary engineering documents mandate that `memory` provides hybrid vector/BM25 retrieval in `< 300ms` for local Obsidian Markdown notes, but do not specify internal data models, scoring formulas, or index synchronization contracts. An ADR is required to establish a clean, pure-Rust, lightweight Alpha architecture without external database dependencies.

---

## 5. Architectural Decisions (ADR-001)

### 1. MemoryStore Internal Architecture
`MemoryStore` is an in-memory thread-safe store wrapping a document store, a BM25 inverted keyword index, and a sparse TF-IDF vector index.
```rust
pub struct MemoryStore {
    config: MemoryConfig,
    entries: std::sync::RwLock<std::collections::BTreeMap<MemoryId, MemoryEntry>>,
    path_index: std::sync::RwLock<std::collections::BTreeMap<String, MemoryId>>,
    bm25_index: std::sync::RwLock<Bm25Index>,
    vector_index: std::sync::RwLock<SparseVectorIndex>,
    next_memory_id: std::sync::atomic::AtomicU64,
}
```

### 2. Memory Data Model
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub file_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryFilter {
    pub tags: Vec<String>,
    pub path_prefix: Option<String>,
    pub limit: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMatchType {
    Bm25,
    Vector,
    Hybrid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub id: MemoryId,
    pub entry: MemoryEntry,
    pub score: f32,
    pub match_type: SearchMatchType,
}
```

### 3. Storage Architecture
- **Vault Read-Only Isolation**: `index_vault()` recursively scans `.md` files in the configured `storage_path`. Files are opened read-only and parsed into `MemoryEntry` records in-memory. **Existing Markdown files are NEVER mutated or modified.**
- **Index Rebuilding**: Indexes are constructed in-memory during system startup or upon calling `index_vault()`. No external database files (SQLite/Qdrant/LanceDB) are created.

### 4. BM25 Model
- **Tokenization**: Standard lowercasing and alphanumeric term splitting.
- **Scoring Parameters**: $k_1 = 1.2$, $b = 0.75$.
- **Formula**:
  $$\text{BM25}(D, Q) = \sum_{t \in Q} \text{IDF}(t) \cdot \frac{f(t, D) \cdot (k_1 + 1)}{f(t, D) + k_1 \cdot \left(1 - b + b \cdot \frac{|D|}{\text{avgdl}}\right)}$$
- **Index Updates**: Modifying or storing an entry incrementally updates term frequencies, document frequencies, and document lengths in memory.

### 5. Sparse Vector Search Model
- **Choice for Alpha**: Pure Rust sparse TF-IDF cosine similarity.
- **Rationale**: External neural GPU embedding models and heavy vector databases (Qdrant, LanceDB, FAISS) introduce heavy binary dependencies, GPU memory consumption, and setup complexity that risk violating the `< 300ms` search latency budget and system idle RAM budget (`< 1.0 GB`). Sparse vectors provide deterministic term-importance similarity natively in pure Rust.
- **Note**: Sparse vector similarity does not provide full neural semantic embeddings, but provides reliable TF-IDF vector similarity for Alpha. Full neural embeddings are explicitly deferred to Beta.

### 6. Hybrid Retrieval Ranking
- **Normalization**: Raw BM25 scores and TF-IDF cosine similarity scores are normalized to $[0.0, 1.0]$.
- **Hybrid Scoring Formula**:
  $$\text{Score}_{\text{Hybrid}} = 0.5 \cdot \text{Score}_{\text{BM25\_Norm}} + 0.5 \cdot \text{Score}_{\text{Vector\_Norm}}$$
- Results above threshold $0.01$ are sorted descending by $\text{Score}_{\text{Hybrid}}$ and capped by `QueryFilter.limit` (defaulting to 10).

### 7. Markdown & Obsidian Vault Indexing
- **Discovery**: Recursively traverses `.md` files under `storage_path`.
- **Title Extraction**: Extracted from the first `# Heading` or file basename.
- **Content Extraction**: Body text of the Markdown file.
- **Tags Extraction**: Extracted from `#tag` patterns or YAML frontmatter `tags: [...]`.
- **Error Handling**: Malformed or unreadable Markdown files produce log warnings and are skipped without failing the overall indexing process.

### 8. Concurrency & Synchronization
- Synchronized using standard library `std::sync::RwLock` and `AtomicU64`.
- Supports multi-threaded concurrent searches (`Send + Sync`, `Arc<MemoryStore>`).
- Zero unsafe code.

### 9. Performance & Latency Targets
- **Search Target**: Warm memory retrieval executes in **`< 300 ms`** (`FIRST_ALPHA_SPEC.md` Section 3).
- **Target Dataset**: Evaluated against local vaults up to 10,000 Markdown documents.

---

## 6. Error Model (`MemoryError`)

```rust
#[derive(Debug)]
pub enum MemoryError {
    EntryNotFound { id: MemoryId },
    VaultNotFound { path: String },
    IndexCorrupted { message: String },
    ConfigError { message: String },
    IoError { message: String },
    LockError { message: String },
    Configuration(configuration::ConfigError),
}
```

---

## 7. Security & Dependency DAG Compliance
- **Direct Dependencies**: `configuration` and `logging` ONLY.
- **Forbidden Dependencies**: `runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, `tool-registry`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`.

---

## 8. Alpha Scope & Deferred Features

### Alpha Scope:
- In-memory `MemoryStore` initialized from `MemoryConfig` or `configuration::Config`.
- Pure Rust BM25 keyword search index.
- Pure Rust sparse TF-IDF vector similarity search.
- Hybrid ranking ($0.5 \cdot \text{BM25} + 0.5 \cdot \text{Vector}$).
- Read-only Obsidian `.md` vault indexer.
- Thread-safe `MemoryStore` API (`store`, `get`, `delete`, `search`, `index_vault`, `count`).
- `< 300ms` warm search response latency.

### Explicitly Deferred to Beta:
- ❌ Neural GPU embedding models (BERT / Qwen-Embedding).
- ❌ Heavy external vector databases (Qdrant, LanceDB, Chroma, FAISS).
- ❌ Real-time filesystem watcher auto-reindexing.
- ❌ Persistent index binary disk caches.
- ❌ Cloud memory synchronization.

---

## 9. Open Architectural Questions
1. *Future Vector Persistence*: Format for serializing sparse TF-IDF index snapshots to disk for instant cold boot (< 10ms) in Beta.

---

## IMPLEMENTATION CONTRACT

### Public Structs:
- `pub struct MemoryStore`
- `pub struct MemoryConfig`
- `pub struct MemoryId(pub u64)`
- `pub struct MemoryEntry`
- `pub struct QueryFilter`
- `pub struct SearchResult`

### Public Enums:
- `pub enum SearchMatchType`
- `pub enum MemoryError`

### Public Type Aliases:
- `pub type Result<T> = std::result::Result<T, MemoryError>`

### Exact Method Signatures:
- `impl MemoryStore`:
  - `pub fn new(config: MemoryConfig) -> Self`
  - `pub fn from_root_config(root_config: &configuration::Config) -> Self`
  - `pub fn store(&self, entry: MemoryEntry) -> Result<MemoryId>`
  - `pub fn get(&self, id: MemoryId) -> Result<MemoryEntry>`
  - `pub fn delete(&self, id: MemoryId) -> Result<()>`
  - `pub fn search(&self, query: &str, filter: Option<QueryFilter>) -> Result<Vec<SearchResult>>`
  - `pub fn index_vault(&self, vault_path: &str) -> Result<usize>`
  - `pub fn count(&self) -> Result<usize>`

### Dependencies:
- `configuration = { path = "../configuration" }`
- `logging = { path = "../logging" }`

### Forbidden Dependencies:
- `runtime`, `kernel`, `capabilities`, `event-bus`, `services`, `orchestrator`, `tool-registry`, `context-engine`, `model-runtime`, `model-providers`, `voice-runtime`, `browser-runtime`, `desktop-runtime`, `automation`, `sdk`, `ui`

Status: APPROVED  
Implementation permitted: YES
