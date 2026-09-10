use context_engine::{ContextEngine, ContextEngineConfig};
use memory::{MemoryConfig, MemoryStore};
use std::sync::Arc;

#[test]
fn test_context_engine_instantiation() {
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    let engine = ContextEngine::new(ContextEngineConfig::default(), memory);
    assert!(format!("{:?}", engine).contains("ContextEngine"));
}
