//! Primary MemoryStore implementation for NAINA OS.

use crate::bm25::Bm25Index;
use crate::config::MemoryConfig;
use crate::error::{MemoryError, Result};
use crate::indexer::VaultIndexer;
use crate::types::{MemoryEntry, MemoryId, QueryFilter, SearchMatchType, SearchResult};
use crate::vector::SparseVectorIndex;
use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Persistent memory store for NAINA OS.
#[derive(Debug)]
pub struct MemoryStore {
    _config: MemoryConfig,
    entries: RwLock<BTreeMap<MemoryId, MemoryEntry>>,
    path_index: RwLock<BTreeMap<String, MemoryId>>,
    bm25_index: RwLock<Bm25Index>,
    vector_index: RwLock<SparseVectorIndex>,
    next_memory_id: AtomicU64,
}

impl MemoryStore {
    /// Creates a new [`MemoryStore`] with the given configuration.
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            _config: config,
            entries: RwLock::new(BTreeMap::new()),
            path_index: RwLock::new(BTreeMap::new()),
            bm25_index: RwLock::new(Bm25Index::new()),
            vector_index: RwLock::new(SparseVectorIndex::new()),
            next_memory_id: AtomicU64::new(1),
        }
    }

    /// Constructs a [`MemoryStore`] directly from the root [`configuration::Config`].
    pub fn from_root_config(root_config: &configuration::Config) -> Self {
        let memory_cfg: MemoryConfig = root_config.memory.clone().into();
        Self::new(memory_cfg)
    }

    /// Stores a [`MemoryEntry`] in the memory store and indexes its content.
    pub fn store(&self, mut entry: MemoryEntry) -> Result<MemoryId> {
        let mid = if entry.id.0 == 0 {
            MemoryId(self.next_memory_id.fetch_add(1, Ordering::SeqCst))
        } else {
            entry.id
        };
        entry.id = mid;

        let full_text = format!("{} {} {}", entry.title, entry.content, entry.tags.join(" "));

        let mut bm25 = self
            .bm25_index
            .write()
            .map_err(|e| MemoryError::LockError {
                message: e.to_string(),
            })?;
        let mut vector = self
            .vector_index
            .write()
            .map_err(|e| MemoryError::LockError {
                message: e.to_string(),
            })?;
        let mut entries_map = self.entries.write().map_err(|e| MemoryError::LockError {
            message: e.to_string(),
        })?;
        let mut path_map = self
            .path_index
            .write()
            .map_err(|e| MemoryError::LockError {
                message: e.to_string(),
            })?;

        bm25.index(mid, &full_text);
        vector.index(mid, &full_text);

        if let Some(ref path) = entry.file_path {
            path_map.insert(path.clone(), mid);
        }

        entries_map.insert(mid, entry);
        Ok(mid)
    }

    /// Retrieves a [`MemoryEntry`] by its [`MemoryId`].
    pub fn get(&self, id: MemoryId) -> Result<MemoryEntry> {
        let entries_map = self.entries.read().map_err(|e| MemoryError::LockError {
            message: e.to_string(),
        })?;

        entries_map
            .get(&id)
            .cloned()
            .ok_or(MemoryError::EntryNotFound { id })
    }

    /// Deletes a [`MemoryEntry`] by its [`MemoryId`].
    pub fn delete(&self, id: MemoryId) -> Result<()> {
        let mut entries_map = self.entries.write().map_err(|e| MemoryError::LockError {
            message: e.to_string(),
        })?;

        let entry = entries_map
            .remove(&id)
            .ok_or(MemoryError::EntryNotFound { id })?;

        let mut bm25 = self
            .bm25_index
            .write()
            .map_err(|e| MemoryError::LockError {
                message: e.to_string(),
            })?;
        let mut vector = self
            .vector_index
            .write()
            .map_err(|e| MemoryError::LockError {
                message: e.to_string(),
            })?;
        let mut path_map = self
            .path_index
            .write()
            .map_err(|e| MemoryError::LockError {
                message: e.to_string(),
            })?;

        bm25.remove(id);
        vector.remove(id);

        if let Some(ref path) = entry.file_path {
            path_map.remove(path);
        }

        Ok(())
    }

    /// Performs a hybrid BM25 and sparse-vector search across all indexed entries.
    pub fn search(&self, query: &str, filter: Option<QueryFilter>) -> Result<Vec<SearchResult>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let bm25_scores = {
            let bm25 = self.bm25_index.read().map_err(|e| MemoryError::LockError {
                message: e.to_string(),
            })?;
            bm25.search(query)
        };

        let vector_scores = {
            let vector = self
                .vector_index
                .read()
                .map_err(|e| MemoryError::LockError {
                    message: e.to_string(),
                })?;
            vector.search(query)
        };

        let max_bm25 = bm25_scores
            .values()
            .copied()
            .fold(0.0f32, |a, b| if b > a { b } else { a });
        let max_vec = vector_scores
            .values()
            .copied()
            .fold(0.0f32, |a, b| if b > a { b } else { a });

        let entries_map = self.entries.read().map_err(|e| MemoryError::LockError {
            message: e.to_string(),
        })?;

        let active_filter = filter.unwrap_or_default();
        let mut candidates: BTreeMap<MemoryId, (f32, f32)> = BTreeMap::new();

        for (&id, &score) in &bm25_scores {
            let norm_bm25 = if max_bm25 > 0.0 {
                score / max_bm25
            } else {
                0.0
            };
            candidates.entry(id).or_insert((0.0, 0.0)).0 = norm_bm25;
        }

        for (&id, &score) in &vector_scores {
            let norm_vec = if max_vec > 0.0 { score / max_vec } else { 0.0 };
            candidates.entry(id).or_insert((0.0, 0.0)).1 = norm_vec;
        }

        let mut results = Vec::new();

        for (id, (norm_bm25, norm_vec)) in candidates {
            let entry = match entries_map.get(&id) {
                Some(e) => e,
                None => continue,
            };

            // Apply QueryFilter
            if !active_filter.tags.is_empty()
                && !active_filter.tags.iter().any(|t| entry.tags.contains(t))
            {
                continue;
            }

            if let Some(ref prefix) = active_filter.path_prefix {
                if let Some(ref path) = entry.file_path {
                    if !path.starts_with(prefix) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            let hybrid_score = 0.5 * norm_bm25 + 0.5 * norm_vec;
            if hybrid_score < 0.001 {
                continue;
            }

            let match_type = if norm_bm25 > 0.0 && norm_vec > 0.0 {
                SearchMatchType::Hybrid
            } else if norm_bm25 > 0.0 {
                SearchMatchType::Bm25
            } else {
                SearchMatchType::Vector
            };

            results.push(SearchResult {
                id,
                entry: entry.clone(),
                score: hybrid_score,
                match_type,
            });
        }

        // Sort descending by score, deterministic tie-breaking by MemoryId
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        if results.len() > active_filter.limit {
            results.truncate(active_filter.limit);
        }

        Ok(results)
    }

    /// Scans a directory read-only for Markdown notes and indexes them into the store.
    pub fn index_vault(&self, vault_path: &str) -> Result<usize> {
        let parsed_notes = VaultIndexer::scan_directory(vault_path)?;
        let mut count = 0;

        for note in parsed_notes {
            let existing_id = {
                let path_map = self.path_index.read().map_err(|e| MemoryError::LockError {
                    message: e.to_string(),
                })?;
                path_map.get(&note.file_path).copied()
            };

            let entry = MemoryEntry {
                id: existing_id.unwrap_or(MemoryId(0)),
                title: note.title,
                content: note.content,
                tags: note.tags,
                file_path: Some(note.file_path),
            };

            self.store(entry)?;
            count += 1;
        }

        Ok(count)
    }

    /// Returns the total number of stored entries in the memory store.
    pub fn count(&self) -> Result<usize> {
        let entries_map = self.entries.read().map_err(|e| MemoryError::LockError {
            message: e.to_string(),
        })?;
        Ok(entries_map.len())
    }
}
