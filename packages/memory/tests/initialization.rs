use memory::{MemoryConfig, MemoryStore};

#[test]
fn test_memory_store_instantiation() {
    let store = MemoryStore::new(MemoryConfig::default());
    assert!(format!("{:?}", store).contains("MemoryStore"));
}
