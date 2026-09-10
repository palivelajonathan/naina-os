//! Default values for the NAINA OS configuration package.
//!
//! Centralizing default constants keeps the configuration package
//! deterministic and makes the root configuration model easier to reason about.

pub const DEFAULT_NAME: &str = "NAINA OS";
pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_LOG_DIRECTORY: &str = "./logs";
pub const DEFAULT_STORAGE_PATH: &str = "./data";
pub const DEFAULT_MEMORY_STORAGE_PATH: &str = "./memory";
pub const DEFAULT_VOICE_WAKE_WORD: &str = "hey naina";
pub const DEFAULT_MAX_RAM_MB: u64 = 1024;
pub const DEFAULT_MAX_LATENCY_MS: u64 = 700;
pub const DEFAULT_MAX_VRAM_MB: u64 = 4800;
