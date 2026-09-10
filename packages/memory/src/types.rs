//! Types for the NAINA OS memory package.

/// Unique identifier for a memory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryId(pub u64);

/// Represents a single document or note in memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryEntry {
    pub id: MemoryId,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub file_path: Option<String>,
}

/// Filter options for querying the memory store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryFilter {
    pub tags: Vec<String>,
    pub path_prefix: Option<String>,
    pub limit: usize,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            path_prefix: None,
            limit: 10,
        }
    }
}

/// Indicates the matching algorithm used for a search result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMatchType {
    Bm25,
    Vector,
    Hybrid,
}

/// Search result representing a matched memory entry and its relevance score.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub id: MemoryId,
    pub entry: MemoryEntry,
    pub score: f32,
    pub match_type: SearchMatchType,
}
