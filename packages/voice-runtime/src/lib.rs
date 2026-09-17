//! NAINA OS voice-runtime package.

pub mod adapters;
pub mod chunker;
pub mod config;
pub mod error;
pub mod traits;
pub mod types;
pub mod voice_runtime;

pub use adapters::{CandleWhisperSttAdapter, MockSttEngine, MockTtsEngine, PiperTtsAdapter};
pub use chunker::TextChunker;
pub use config::VoiceRuntimeConfig;
pub use error::{Result, VoiceRuntimeError};
pub use traits::{SttEngine, TtsEngine};
pub use types::{
    AudioBuffer, CognitiveTurnResult, StreamTimeline, StreamingTurnResult, SynthesisResult,
    SynthesizedAudioChunk, TranscriptionResult, VoiceState,
};
pub use voice_runtime::VoiceRuntime;
