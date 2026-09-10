//! Pure Rust sparse TF-IDF vector similarity search index.

use crate::bm25::Bm25Index;
use crate::types::MemoryId;
use std::collections::BTreeMap;

/// Sparse TF-IDF vector similarity index.
#[derive(Debug, Default)]
pub struct SparseVectorIndex {
    /// Document term frequencies: doc_id -> (term -> term_frequency)
    doc_tfs: BTreeMap<MemoryId, BTreeMap<String, usize>>,
    /// Document term count: doc_id -> total_tokens
    doc_token_counts: BTreeMap<MemoryId, usize>,
    /// Global term document counts: term -> count of documents containing term
    term_doc_counts: BTreeMap<String, usize>,
}

impl SparseVectorIndex {
    /// Creates a new empty [`SparseVectorIndex`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Indexes or re-indexes a document.
    pub fn index(&mut self, id: MemoryId, text: &str) {
        self.remove(id);

        let tokens = Bm25Index::tokenize(text);
        let count = tokens.len();
        if count == 0 {
            return;
        }

        let mut tfs: BTreeMap<String, usize> = BTreeMap::new();
        for token in tokens {
            *tfs.entry(token).or_insert(0) += 1;
        }

        for term in tfs.keys() {
            *self.term_doc_counts.entry(term.clone()).or_insert(0) += 1;
        }

        self.doc_token_counts.insert(id, count);
        self.doc_tfs.insert(id, tfs);
    }

    /// Removes a document from the index.
    pub fn remove(&mut self, id: MemoryId) {
        if let Some(tfs) = self.doc_tfs.remove(&id) {
            self.doc_token_counts.remove(&id);

            let mut empty_terms = Vec::new();
            for term in tfs.keys() {
                if let Some(cnt) = self.term_doc_counts.get_mut(term) {
                    *cnt = cnt.saturating_sub(1);
                    if *cnt == 0 {
                        empty_terms.push(term.clone());
                    }
                }
            }

            for term in empty_terms {
                self.term_doc_counts.remove(&term);
            }
        }
    }

    /// Computes cosine similarity scores between query TF-IDF vector and document vectors.
    pub fn search(&self, query: &str) -> BTreeMap<MemoryId, f32> {
        let query_tokens = Bm25Index::tokenize(query);
        if query_tokens.is_empty() || self.doc_tfs.is_empty() {
            return BTreeMap::new();
        }

        let n = self.doc_tfs.len() as f32;
        let mut query_tfs: BTreeMap<String, usize> = BTreeMap::new();
        let query_len = query_tokens.len() as f32;

        for token in query_tokens {
            *query_tfs.entry(token).or_insert(0) += 1;
        }

        // Build query vector
        let mut query_vec: BTreeMap<String, f32> = BTreeMap::new();
        let mut query_norm_sq = 0.0;

        for (term, &tf) in &query_tfs {
            let doc_cnt = *self.term_doc_counts.get(term).unwrap_or(&0) as f32;
            let idf = ((1.0 + n) / (1.0 + doc_cnt)).ln() + 1.0;
            let weight = ((tf as f32) / query_len) * idf;
            query_vec.insert(term.clone(), weight);
            query_norm_sq += weight * weight;
        }

        if query_norm_sq == 0.0 {
            return BTreeMap::new();
        }

        let query_norm = query_norm_sq.sqrt();
        let mut scores: BTreeMap<MemoryId, f32> = BTreeMap::new();

        for (&id, doc_tf_map) in &self.doc_tfs {
            let doc_len = *self.doc_token_counts.get(&id).unwrap_or(&1) as f32;
            let mut dot_product = 0.0;
            let mut doc_norm_sq = 0.0;

            for (term, &tf) in doc_tf_map {
                let doc_cnt = *self.term_doc_counts.get(term).unwrap_or(&1) as f32;
                let idf = ((1.0 + n) / (1.0 + doc_cnt)).ln() + 1.0;
                let weight = ((tf as f32) / doc_len) * idf;

                doc_norm_sq += weight * weight;

                if let Some(&q_weight) = query_vec.get(term) {
                    dot_product += q_weight * weight;
                }
            }

            if dot_product > 0.0 && doc_norm_sq > 0.0 {
                let sim = dot_product / (query_norm * doc_norm_sq.sqrt());
                scores.insert(id, sim);
            }
        }

        scores
    }
}
