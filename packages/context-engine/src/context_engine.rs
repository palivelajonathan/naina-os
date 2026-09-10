//! Primary ContextEngine implementation for NAINA OS.

use crate::config::ContextEngineConfig;
use crate::error::{ContextEngineError, Result};
use crate::types::{ContextTurn, ContextWindow, ConversationId, Role};
use memory::MemoryStore;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Multi-turn conversation context manager for NAINA OS.
#[derive(Debug)]
pub struct ContextEngine {
    config: ContextEngineConfig,
    memory: Arc<MemoryStore>,
    conversations: RwLock<BTreeMap<ConversationId, Vec<ContextTurn>>>,
    next_conversation_id: AtomicU64,
}

impl ContextEngine {
    /// Pure Rust deterministic token estimator (~4 UTF-8 characters per estimated token).
    pub fn estimate_tokens(text: &str) -> usize {
        let char_count = text.chars().count();
        if char_count == 0 {
            0
        } else {
            char_count.div_ceil(4)
        }
    }

    /// Creates a new [`ContextEngine`] with the given configuration and memory store reference.
    pub fn new(config: ContextEngineConfig, memory: Arc<MemoryStore>) -> Self {
        Self {
            config,
            memory,
            conversations: RwLock::new(BTreeMap::new()),
            next_conversation_id: AtomicU64::new(1),
        }
    }

    /// Constructs a [`ContextEngine`] directly from the root [`configuration::Config`].
    pub fn from_root_config(root_config: &configuration::Config, memory: Arc<MemoryStore>) -> Self {
        let config = ContextEngineConfig {
            max_tokens: root_config.model.context_window,
            ..Default::default()
        };
        Self::new(config, memory)
    }

    /// Creates a new conversation and returns its unique [`ConversationId`].
    pub fn create_conversation(&self) -> Result<ConversationId> {
        let cid = ConversationId(self.next_conversation_id.fetch_add(1, Ordering::SeqCst));
        let mut map = self
            .conversations
            .write()
            .map_err(|e| ContextEngineError::LockError {
                message: e.to_string(),
            })?;
        map.insert(cid, Vec::new());
        Ok(cid)
    }

    /// Adds a turn to an existing conversation.
    pub fn add_turn(
        &self,
        conversation_id: ConversationId,
        role: Role,
        content: impl Into<String>,
    ) -> Result<ContextTurn> {
        let mut map = self
            .conversations
            .write()
            .map_err(|e| ContextEngineError::LockError {
                message: e.to_string(),
            })?;

        let turns =
            map.get_mut(&conversation_id)
                .ok_or(ContextEngineError::ConversationNotFound {
                    id: conversation_id,
                })?;

        let content_str = content.into();
        let token_count = Self::estimate_tokens(&content_str);
        let turn_id = (turns.len() as u64) + 1;

        let turn = ContextTurn {
            id: turn_id,
            role,
            content: content_str,
            token_count,
        };

        turns.push(turn.clone());
        Ok(turn)
    }

    /// Retrieves the raw history of turns for a conversation.
    pub fn get_history(&self, conversation_id: ConversationId) -> Result<Vec<ContextTurn>> {
        let map = self
            .conversations
            .read()
            .map_err(|e| ContextEngineError::LockError {
                message: e.to_string(),
            })?;

        let turns = map
            .get(&conversation_id)
            .ok_or(ContextEngineError::ConversationNotFound {
                id: conversation_id,
            })?;

        Ok(turns.clone())
    }

    /// Clears and removes a conversation history.
    pub fn clear_conversation(&self, conversation_id: ConversationId) -> Result<()> {
        let mut map = self
            .conversations
            .write()
            .map_err(|e| ContextEngineError::LockError {
                message: e.to_string(),
            })?;

        map.remove(&conversation_id)
            .ok_or(ContextEngineError::ConversationNotFound {
                id: conversation_id,
            })?;

        Ok(())
    }

    /// Assembles a pruned [`ContextWindow`] with System context, retrieved long-term memories, and retained history.
    pub fn assemble_context(
        &self,
        conversation_id: ConversationId,
        query_override: Option<&str>,
    ) -> Result<ContextWindow> {
        let raw_turns = self.get_history(conversation_id)?;

        let system_turn = raw_turns.iter().find(|t| t.role == Role::System).cloned();

        let system_tokens = system_turn.as_ref().map(|t| t.token_count).unwrap_or(0);

        // Memory Store Retrieval
        let memory_query = if let Some(q) = query_override {
            q.to_string()
        } else {
            raw_turns
                .iter()
                .rfind(|t| t.role == Role::User)
                .map(|t| t.content.clone())
                .unwrap_or_default()
        };

        let retrieved_memories = if !memory_query.trim().is_empty() {
            let filter = memory::QueryFilter {
                tags: Vec::new(),
                path_prefix: None,
                limit: self.config.memory_retrieval_limit,
            };
            self.memory.search(&memory_query, Some(filter))?
        } else {
            Vec::new()
        };

        let memory_tokens: usize = retrieved_memories
            .iter()
            .map(|m| Self::estimate_tokens(&format!("{} {}", m.entry.title, m.entry.content)))
            .sum();

        let total_reserved_tokens = system_tokens + memory_tokens;
        if total_reserved_tokens > self.config.max_tokens {
            return Err(ContextEngineError::TokenLimitExceeded {
                limit: self.config.max_tokens,
                requested: total_reserved_tokens,
            });
        }

        let available_token_budget = self.config.max_tokens - total_reserved_tokens;

        let non_system_turns: Vec<ContextTurn> = raw_turns
            .into_iter()
            .filter(|t| t.role != Role::System)
            .collect();

        // 5-User-Turn Retention Rule Verification
        let user_turn_indices: Vec<usize> = non_system_turns
            .iter()
            .enumerate()
            .filter(|(_, t)| t.role == Role::User)
            .map(|(idx, _)| idx)
            .collect();

        let total_user_turns = user_turn_indices.len();
        let target_user_turns = std::cmp::min(total_user_turns, 5);

        let required_start_idx = if target_user_turns > 0 {
            user_turn_indices[total_user_turns - target_user_turns]
        } else {
            0
        };

        // Calculate minimum tokens required to satisfy 5 user turns
        let min_required_tokens: usize = non_system_turns[required_start_idx..]
            .iter()
            .map(|t| t.token_count)
            .sum();

        if min_required_tokens > available_token_budget {
            return Err(ContextEngineError::TokenLimitExceeded {
                limit: self.config.max_tokens,
                requested: total_reserved_tokens + min_required_tokens,
            });
        }

        // Sliding-window pruning from newest to oldest
        let mut selected_history: Vec<ContextTurn> = Vec::new();
        let mut current_tokens = 0;

        for turn in non_system_turns.iter().rev() {
            if current_tokens + turn.token_count <= available_token_budget {
                selected_history.push(turn.clone());
                current_tokens += turn.token_count;
            } else if selected_history.len() < (non_system_turns.len() - required_start_idx) {
                // If pruning would drop required user turns under 5 when budget permits, enforce error
                return Err(ContextEngineError::TokenLimitExceeded {
                    limit: self.config.max_tokens,
                    requested: total_reserved_tokens + current_tokens + turn.token_count,
                });
            } else {
                break;
            }
        }

        selected_history.reverse();

        // Deterministic Assembly Order: System -> Retrieved Memories -> Chronological History
        let mut final_turns = Vec::new();
        if let Some(sys) = system_turn {
            final_turns.push(sys);
        }
        final_turns.extend(selected_history);

        let total_tokens = total_reserved_tokens + current_tokens;

        Ok(ContextWindow {
            turns: final_turns,
            retrieved_memories,
            total_tokens,
        })
    }
}
