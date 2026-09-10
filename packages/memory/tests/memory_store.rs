use memory::{
    MemoryConfig, MemoryEntry, MemoryError, MemoryId, MemoryStore, QueryFilter, SearchMatchType,
    VaultIndexer,
};
use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn sample_entry(title: &str, content: &str, tags: Vec<&str>) -> MemoryEntry {
    MemoryEntry {
        id: MemoryId(0),
        title: title.to_string(),
        content: content.to_string(),
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
        file_path: None,
    }
}

#[test]
fn test_01_construction_and_configuration() {
    let config = MemoryConfig {
        enabled: true,
        storage_path: "./test_vault".to_string(),
    };
    let store = MemoryStore::new(config);
    assert_eq!(store.count().unwrap(), 0);

    let root_config = configuration::Config::default();
    let store_from_root = MemoryStore::from_root_config(&root_config);
    assert_eq!(store_from_root.count().unwrap(), 0);
}

#[test]
fn test_02_store_and_get() {
    let store = MemoryStore::new(MemoryConfig::default());
    let entry = sample_entry(
        "Rust Architecture",
        "Microkernel design pattern",
        vec!["rust", "architecture"],
    );

    let id = store.store(entry).unwrap();
    assert_eq!(id.0, 1);

    let retrieved = store.get(id).unwrap();
    assert_eq!(retrieved.title, "Rust Architecture");
    assert_eq!(retrieved.content, "Microkernel design pattern");
    assert_eq!(retrieved.tags, vec!["rust", "architecture"]);
}

#[test]
fn test_03_memory_id_generation() {
    let store = MemoryStore::new(MemoryConfig::default());
    let id1 = store
        .store(sample_entry("Doc 1", "Content 1", vec![]))
        .unwrap();
    let id2 = store
        .store(sample_entry("Doc 2", "Content 2", vec![]))
        .unwrap();

    assert_eq!(id1.0, 1);
    assert_eq!(id2.0, 2);
}

#[test]
fn test_04_delete() {
    let store = MemoryStore::new(MemoryConfig::default());
    let id = store
        .store(sample_entry("To Delete", "Transient data", vec![]))
        .unwrap();

    assert_eq!(store.count().unwrap(), 1);
    assert!(store.delete(id).is_ok());
    assert_eq!(store.count().unwrap(), 0);

    let err = store.get(id).unwrap_err();
    assert!(matches!(err, MemoryError::EntryNotFound { .. }));
}

#[test]
fn test_05_missing_entry() {
    let store = MemoryStore::new(MemoryConfig::default());
    let err = store.get(MemoryId(999)).unwrap_err();
    assert!(matches!(err, MemoryError::EntryNotFound { .. }));

    let del_err = store.delete(MemoryId(999)).unwrap_err();
    assert!(matches!(del_err, MemoryError::EntryNotFound { .. }));
}

#[test]
fn test_06_markdown_parsing_and_indexing() {
    let temp_dir = std::env::temp_dir().join("naina_test_vault_06");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let note_path = temp_dir.join("architecture.md");
    let note_content = "# System Architecture\n\nThis is a microkernel system. #system #rust\n";
    fs::write(&note_path, note_content).unwrap();

    let parsed = VaultIndexer::parse_file(&note_path).unwrap();
    assert_eq!(parsed.title, "System Architecture");
    assert!(parsed.tags.contains(&"system".to_string()));
    assert!(parsed.tags.contains(&"rust".to_string()));

    let store = MemoryStore::new(MemoryConfig::default());
    let count = store.index_vault(temp_dir.to_str().unwrap()).unwrap();
    assert_eq!(count, 1);
    assert_eq!(store.count().unwrap(), 1);

    // Source Markdown file MUST remain untouched
    let re_read = fs::read_to_string(&note_path).unwrap();
    assert_eq!(re_read, note_content);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_07_missing_vault() {
    let store = MemoryStore::new(MemoryConfig::default());
    let err = store
        .index_vault("./non_existent_vault_dir_12345")
        .unwrap_err();
    assert!(matches!(err, MemoryError::VaultNotFound { .. }));
}

#[test]
fn test_08_bm25_search() {
    let store = MemoryStore::new(MemoryConfig::default());
    store
        .store(sample_entry(
            "Alpha Spec",
            "First Alpha specification document",
            vec!["spec"],
        ))
        .unwrap();
    store
        .store(sample_entry(
            "Beta Spec",
            "Future Beta cloud sync features",
            vec!["spec"],
        ))
        .unwrap();

    let results = store.search("Alpha", None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].entry.title, "Alpha Spec");
}

#[test]
fn test_09_vector_similarity_search() {
    let store = MemoryStore::new(MemoryConfig::default());
    store
        .store(sample_entry(
            "Kernel Design",
            "Process supervisor and zero trust security",
            vec!["kernel"],
        ))
        .unwrap();
    store
        .store(sample_entry(
            "Voice Pipeline",
            "Whisper STT and Piper TTS integration",
            vec!["voice"],
        ))
        .unwrap();

    let results = store.search("supervisor security", None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].entry.title, "Kernel Design");
}

#[test]
fn test_10_hybrid_search_and_score_normalization() {
    let store = MemoryStore::new(MemoryConfig::default());
    store
        .store(sample_entry(
            "Hybrid Search Engine",
            "BM25 keyword plus sparse vector similarity ranking",
            vec!["search"],
        ))
        .unwrap();

    let results = store.search("BM25 sparse vector", None).unwrap();
    assert!(!results.is_empty());
    let first = &results[0];
    assert!(first.score > 0.0 && first.score <= 1.0);
    assert_eq!(first.match_type, SearchMatchType::Hybrid);
}

#[test]
fn test_11_deterministic_ordering() {
    let store = MemoryStore::new(MemoryConfig::default());
    store
        .store(sample_entry(
            "Doc A",
            "Identical matching content keyword",
            vec![],
        ))
        .unwrap();
    store
        .store(sample_entry(
            "Doc B",
            "Identical matching content keyword",
            vec![],
        ))
        .unwrap();

    let r1 = store.search("keyword", None).unwrap();
    let r2 = store.search("keyword", None).unwrap();

    assert_eq!(r1.len(), 2);
    assert_eq!(r1[0].id, r2[0].id);
    assert_eq!(r1[1].id, r2[1].id);
}

#[test]
fn test_12_query_filter() {
    let store = MemoryStore::new(MemoryConfig::default());
    let mut e1 = sample_entry("Note 1", "Contains target keyword", vec!["tag_a"]);
    e1.file_path = Some("/vault/folder1/note1.md".to_string());
    store.store(e1).unwrap();

    let mut e2 = sample_entry("Note 2", "Contains target keyword", vec!["tag_b"]);
    e2.file_path = Some("/vault/folder2/note2.md".to_string());
    store.store(e2).unwrap();

    let filter = QueryFilter {
        tags: vec!["tag_a".to_string()],
        path_prefix: Some("/vault/folder1".to_string()),
        limit: 10,
    };

    let results = store.search("target", Some(filter)).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.title, "Note 1");
}

#[test]
fn test_13_concurrent_reads_and_searches() {
    let store = Arc::new(MemoryStore::new(MemoryConfig::default()));

    for i in 0..50 {
        store
            .store(sample_entry(
                &format!("Concurrent Note {i}"),
                "Testing thread safe search",
                vec!["test"],
            ))
            .unwrap();
    }

    let mut handles = Vec::new();
    for _ in 0..8 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                let results = store_clone.search("testing thread", None).unwrap();
                assert!(!results.is_empty());
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_14_concurrent_reads_and_writes() {
    let store = Arc::new(MemoryStore::new(MemoryConfig::default()));
    let mut handles = Vec::new();

    for i in 0..4 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for j in 0..20 {
                let title = format!("Writer {i} Note {j}");
                store_clone
                    .store(sample_entry(&title, "Dynamic concurrent write", vec![]))
                    .unwrap();
            }
        }));
    }

    for _ in 0..4 {
        let store_clone = Arc::clone(&store);
        handles.push(thread::spawn(move || {
            for _ in 0..20 {
                let _ = store_clone.search("concurrent write", None);
                let _ = store_clone.count();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(store.count().unwrap(), 80);
}

#[test]
fn test_15_empty_query_and_no_results() {
    let store = MemoryStore::new(MemoryConfig::default());
    store
        .store(sample_entry("Title", "Content text", vec![]))
        .unwrap();

    let empty_res = store.search("", None).unwrap();
    assert!(empty_res.is_empty());

    let whitespace_res = store.search("   ", None).unwrap();
    assert!(whitespace_res.is_empty());

    let no_match_res = store.search("xyz123nonexistent", None).unwrap();
    assert!(no_match_res.is_empty());
}

#[test]
fn test_16_performance_retrieval_under_300ms() {
    let store = MemoryStore::new(MemoryConfig::default());

    // Populate dataset with 500 documents
    for i in 0..500 {
        let title = format!("Document Title Index {i}");
        let content = format!(
            "This is the content of synthetic test document number {i} discussing software architecture, memory stores, and vector search."
        );
        store
            .store(sample_entry(&title, &content, vec!["perf", "test"]))
            .unwrap();
    }

    let start = Instant::now();
    let results = store
        .search("software architecture vector search", None)
        .unwrap();
    let elapsed = start.elapsed();

    println!("Performance test search duration: {:?}", elapsed);
    assert!(!results.is_empty());
    assert!(
        elapsed.as_millis() < 300,
        "Memory retrieval exceeded 300ms budget: {}ms",
        elapsed.as_millis()
    );
}
