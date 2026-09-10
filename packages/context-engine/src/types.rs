//! Types for the NAINA OS context-engine package.

/// Unique identifier for a conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationId(pub u64);

/// Role of a participant in a conversation turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Represents a single turn within a conversation history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextTurn {
    pub id: u64,
    pub role: Role,
    pub content: String,
    pub token_count: usize,
}

/// Assembled context window containing pruned conversation history and retrieved memories.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextWindow {
    pub turns: Vec<ContextTurn>,
    pub retrieved_memories: Vec<memory::SearchResult>,
    pub total_tokens: usize,
}
