//! NAINA OS context-engine package.

pub mod config;
pub mod context_engine;
pub mod error;
pub mod traits;
pub mod types;

pub use config::ContextEngineConfig;
pub use context_engine::ContextEngine;
pub use error::{ContextEngineError, Result};
pub use types::{ContextTurn, ContextWindow, ConversationId, Role};
