//! Pure Rust BM25 inverted keyword search index.

use crate::types::MemoryId;
use std::collections::{BTreeMap, BTreeSet};

/// BM25 scoring parameters.
const K1: f32 = 1.2;
const B: f32 = 0.75;

/// In-memory BM25 index.
#[derive(Debug, Default)]
pub struct Bm25Index {
    /// Inverted index: term -> (doc_id -> term_frequency)
    term_docs: BTreeMap<String, BTreeMap<MemoryId, usize>>,
    /// Document length in token count: doc_id -> length
    doc_lengths: BTreeMap<MemoryId, usize>,
    /// Total number of tokens across all documents.
    total_tokens: usize,
}

impl Bm25Index {
    /// Creates a new empty [`Bm25Index`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Tokenizes text into lowercase alphanumeric terms.
    pub fn tokenize(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect()
    }

    /// Indexes or re-indexes a document.
    pub fn index(&mut self, id: MemoryId, text: &str) {
        self.remove(id);

        let tokens = Self::tokenize(text);
        let doc_len = tokens.len();
        if doc_len == 0 {
            return;
        }

        self.doc_lengths.insert(id, doc_len);
        self.total_tokens += doc_len;

        for token in tokens {
            self.term_docs
                .entry(token)
                .or_default()
                .entry(id)
                .and_modify(|freq| *freq += 1)
                .or_insert(1);
        }
    }

    /// Removes a document from the index.
    pub fn remove(&mut self, id: MemoryId) {
        if let Some(doc_len) = self.doc_lengths.remove(&id) {
            self.total_tokens = self.total_tokens.saturating_sub(doc_len);

            let mut empty_terms = Vec::new();
            for (term, docs) in self.term_docs.iter_mut() {
                docs.remove(&id);
                if docs.is_empty() {
                    empty_terms.push(term.clone());
                }
            }

            for term in empty_terms {
                self.term_docs.remove(&term);
            }
        }
    }

    /// Computes BM25 raw scores for all matching documents for a query.
    pub fn search(&self, query: &str) -> BTreeMap<MemoryId, f32> {
        let query_tokens = Self::tokenize(query);
        if query_tokens.is_empty() || self.doc_lengths.is_empty() {
            return BTreeMap::new();
        }

        let n = self.doc_lengths.len() as f32;
        let avgdl = (self.total_tokens as f32) / n;

        let query_terms: BTreeSet<String> = query_tokens.into_iter().collect();
        let mut scores: BTreeMap<MemoryId, f32> = BTreeMap::new();

        for term in &query_terms {
            if let Some(docs) = self.term_docs.get(term) {
                let doc_freq = docs.len() as f32;
                let idf = ((n - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();

                for (&id, &tf) in docs {
                    let doc_len = *self.doc_lengths.get(&id).unwrap_or(&1) as f32;
                    let tf_f = tf as f32;
                    let term_score =
                        idf * (tf_f * (K1 + 1.0)) / (tf_f + K1 * (1.0 - B + B * (doc_len / avgdl)));

                    *scores.entry(id).or_insert(0.0) += term_score;
                }
            }
        }

        scores
    }
}
