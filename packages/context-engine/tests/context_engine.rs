use context_engine::{
    ContextEngine, ContextEngineConfig, ContextEngineError, ConversationId, Role,
};
use memory::{MemoryConfig, MemoryEntry, MemoryId, MemoryStore};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn setup_engine() -> (ContextEngine, Arc<MemoryStore>) {
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    let config = ContextEngineConfig::default();
    let engine = ContextEngine::new(config, Arc::clone(&memory));
    (engine, memory)
}

#[test]
fn test_01_construction_and_configuration() {
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    let config = ContextEngineConfig {
        max_turns: 10,
        max_tokens: 1024,
        memory_retrieval_limit: 3,
    };
    let engine = ContextEngine::new(config.clone(), Arc::clone(&memory));
    assert!(format!("{:?}", engine).contains("ContextEngine"));

    let root_config = configuration::Config::default();
    let engine_from_root = ContextEngine::from_root_config(&root_config, memory);
    assert!(format!("{:?}", engine_from_root).contains("ContextEngine"));
}

#[test]
fn test_02_conversation_creation_and_id_generation() {
    let (engine, _) = setup_engine();
    let c1 = engine.create_conversation().unwrap();
    let c2 = engine.create_conversation().unwrap();

    assert_ne!(c1, c2);
    assert_eq!(c1.0, 1);
    assert_eq!(c2.0, 2);
}

#[test]
fn test_03_missing_conversation() {
    let (engine, _) = setup_engine();
    let missing_id = ConversationId(999);

    let err = engine.get_history(missing_id).unwrap_err();
    assert!(matches!(err, ContextEngineError::ConversationNotFound { id } if id == missing_id));

    let add_err = engine
        .add_turn(missing_id, Role::User, "Hello")
        .unwrap_err();
    assert!(matches!(
        add_err,
        ContextEngineError::ConversationNotFound { .. }
    ));

    let assemble_err = engine.assemble_context(missing_id, None).unwrap_err();
    assert!(matches!(
        assemble_err,
        ContextEngineError::ConversationNotFound { .. }
    ));

    let clear_err = engine.clear_conversation(missing_id).unwrap_err();
    assert!(matches!(
        clear_err,
        ContextEngineError::ConversationNotFound { .. }
    ));
}

#[test]
fn test_04_turn_insertion_roles_and_token_estimation() {
    let (engine, _) = setup_engine();
    let cid = engine.create_conversation().unwrap();

    let t1 = engine
        .add_turn(cid, Role::System, "You are NAINA OS.")
        .unwrap();
    assert_eq!(t1.role, Role::System);
    assert_eq!(
        t1.token_count,
        ContextEngine::estimate_tokens("You are NAINA OS.")
    );

    let t2 = engine.add_turn(cid, Role::User, "Hello AI!").unwrap();
    assert_eq!(t2.role, Role::User);

    let t3 = engine.add_turn(cid, Role::Assistant, "Greetings!").unwrap();
    assert_eq!(t3.role, Role::Assistant);

    let t4 = engine
        .add_turn(cid, Role::Tool, "Tool result data")
        .unwrap();
    assert_eq!(t4.role, Role::Tool);

    let history = engine.get_history(cid).unwrap();
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].role, Role::System);
    assert_eq!(history[1].role, Role::User);
    assert_eq!(history[2].role, Role::Assistant);
    assert_eq!(history[3].role, Role::Tool);
}

#[test]
fn test_05_token_estimation_accuracy() {
    assert_eq!(ContextEngine::estimate_tokens(""), 0);
    assert_eq!(ContextEngine::estimate_tokens("a"), 1);
    assert_eq!(ContextEngine::estimate_tokens("abcd"), 1);
    assert_eq!(ContextEngine::estimate_tokens("abcdefgh"), 2);
}

#[test]
fn test_06_five_user_turn_retention_verification() {
    let (engine, _) = setup_engine();
    let cid = engine.create_conversation().unwrap();

    engine.add_turn(cid, Role::System, "System prompt").unwrap();

    // Add 6 user/assistant pairs (6 user turns)
    for i in 1..=6 {
        engine
            .add_turn(cid, Role::User, format!("User turn {i}"))
            .unwrap();
        engine
            .add_turn(cid, Role::Assistant, format!("Assistant turn {i}"))
            .unwrap();
    }

    let window = engine.assemble_context(cid, None).unwrap();
    let user_turns: Vec<_> = window
        .turns
        .iter()
        .filter(|t| t.role == Role::User)
        .collect();

    // Verification requirement: MUST retain at least 5 user turns
    assert!(
        user_turns.len() >= 5,
        "Context window must retain at least 5 user turns"
    );
    assert_eq!(user_turns.len(), 6);
}

#[test]
fn test_07_insufficient_token_budget_handling() {
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    // Set a tiny token limit of 10 tokens
    let config = ContextEngineConfig {
        max_turns: 20,
        max_tokens: 10,
        memory_retrieval_limit: 5,
    };
    let engine = ContextEngine::new(config, memory);
    let cid = engine.create_conversation().unwrap();

    // Add 5 long user turns requiring > 10 tokens
    for i in 1..=5 {
        engine
            .add_turn(
                cid,
                Role::User,
                format!("This is a long user query number {i}"),
            )
            .unwrap();
    }

    let err = engine.assemble_context(cid, None).unwrap_err();
    assert!(matches!(err, ContextEngineError::TokenLimitExceeded { .. }));
}

#[test]
fn test_08_memorystore_retrieval_integration() {
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    memory
        .store(MemoryEntry {
            id: MemoryId(0),
            title: "Kernel Architecture".to_string(),
            content: "NKRS zero-trust security policy".to_string(),
            tags: vec!["kernel".to_string()],
            file_path: None,
        })
        .unwrap();

    let engine = ContextEngine::new(ContextEngineConfig::default(), Arc::clone(&memory));
    let cid = engine.create_conversation().unwrap();
    engine
        .add_turn(cid, Role::User, "Tell me about NKRS zero-trust")
        .unwrap();

    let window = engine.assemble_context(cid, None).unwrap();
    assert!(!window.retrieved_memories.is_empty());
    assert_eq!(
        window.retrieved_memories[0].entry.title,
        "Kernel Architecture"
    );
}

#[test]
fn test_09_query_override() {
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    memory
        .store(MemoryEntry {
            id: MemoryId(0),
            title: "Specific Topic".to_string(),
            content: "Quantum computing algorithms".to_string(),
            tags: vec!["quantum".to_string()],
            file_path: None,
        })
        .unwrap();

    let engine = ContextEngine::new(ContextEngineConfig::default(), Arc::clone(&memory));
    let cid = engine.create_conversation().unwrap();
    engine
        .add_turn(cid, Role::User, "Irrelevant chatter")
        .unwrap();

    let window = engine
        .assemble_context(cid, Some("Quantum computing"))
        .unwrap();
    assert!(!window.retrieved_memories.is_empty());
    assert_eq!(window.retrieved_memories[0].entry.title, "Specific Topic");
}

#[test]
fn test_10_deterministic_context_assembly_ordering() {
    let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));
    memory
        .store(MemoryEntry {
            id: MemoryId(0),
            title: "Memory Note".to_string(),
            content: "Retrieved memory payload".to_string(),
            tags: vec![],
            file_path: None,
        })
        .unwrap();

    let engine = ContextEngine::new(ContextEngineConfig::default(), Arc::clone(&memory));
    let cid = engine.create_conversation().unwrap();
    engine.add_turn(cid, Role::System, "System prompt").unwrap();
    engine
        .add_turn(cid, Role::User, "Retrieved memory payload")
        .unwrap();
    engine.add_turn(cid, Role::Assistant, "Response").unwrap();

    let window = engine.assemble_context(cid, None).unwrap();
    assert_eq!(window.turns[0].role, Role::System);
    assert!(!window.retrieved_memories.is_empty());
    assert_eq!(window.turns[1].role, Role::User);
    assert_eq!(window.turns[2].role, Role::Assistant);
}

#[test]
fn test_11_clear_conversation() {
    let (engine, _) = setup_engine();
    let cid = engine.create_conversation().unwrap();
    engine.add_turn(cid, Role::User, "Hello").unwrap();

    assert!(engine.clear_conversation(cid).is_ok());
    assert!(matches!(
        engine.get_history(cid).unwrap_err(),
        ContextEngineError::ConversationNotFound { .. }
    ));
}

#[test]
fn test_12_concurrent_conversations_and_turns() {
    let (engine, _) = setup_engine();
    let engine = Arc::new(engine);

    let mut handles = Vec::new();
    for _ in 0..8 {
        let eng = Arc::clone(&engine);
        handles.push(thread::spawn(move || {
            let cid = eng.create_conversation().unwrap();
            for i in 0..10 {
                eng.add_turn(cid, Role::User, format!("Msg {i}")).unwrap();
                eng.add_turn(cid, Role::Assistant, format!("Reply {i}"))
                    .unwrap();
            }
            let window = eng.assemble_context(cid, None).unwrap();
            assert!(!window.turns.is_empty());
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_13_performance_context_assembly_target() {
    let (engine, _) = setup_engine();
    let cid = engine.create_conversation().unwrap();
    engine
        .add_turn(cid, Role::System, "System directive prompt")
        .unwrap();

    for i in 1..=10 {
        engine
            .add_turn(cid, Role::User, format!("User message number {i}"))
            .unwrap();
        engine
            .add_turn(cid, Role::Assistant, format!("Assistant reply number {i}"))
            .unwrap();
    }

    let start = Instant::now();
    let window = engine.assemble_context(cid, None).unwrap();
    let elapsed = start.elapsed();

    println!("Context assembly duration: {:?}", elapsed);
    assert!(!window.turns.is_empty());
    assert!(
        elapsed.as_millis() < 15,
        "Context assembly exceeded 15ms target: {}ms",
        elapsed.as_millis()
    );
}
