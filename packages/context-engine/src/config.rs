//! Configuration for the NAINA OS context-engine package.

/// Configuration parameters for the context engine.
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
