//! Configuration types for the NAINA OS voice-runtime package.

use configuration::VoiceConfig;

/// Configuration parameters for voice processing, Whisper STT, and Piper TTS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRuntimeConfig {
    /// Audio sample rate in Hz (Default: 16000 Hz).
    pub sample_rate: u32,
    /// Number of audio channels (Default: 1 channel, Mono).
    pub channels: u16,
    /// Audio frame size in bytes (Default: 640 bytes, 10ms @ 16kHz i16 mono).
    pub frame_size_bytes: usize,
    /// Maximum allowed voice-to-voice turn latency budget in milliseconds (Default: 700 ms).
    pub latency_target_ms: u64,
    /// Default Whisper STT model identifier.
    pub stt_model_id: String,
    /// Default Piper TTS voice identifier.
    pub tts_voice_id: String,
    /// Whether voice service is enabled in configuration.
    pub enabled: bool,
    /// Configured wake word for voice activation.
    pub wake_word: String,
}

impl Default for VoiceRuntimeConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            frame_size_bytes: 640,
            latency_target_ms: 700,
            stt_model_id: "whisper-base-en".to_string(),
            tts_voice_id: "piper-en-medium".to_string(),
            enabled: true,
            wake_word: "hey naina".to_string(),
        }
    }
}

impl VoiceRuntimeConfig {
    /// Creates a [`VoiceRuntimeConfig`] derived from a root [`configuration::VoiceConfig`].
    pub fn from_voice_config(voice_cfg: &VoiceConfig) -> Self {
        Self {
            enabled: voice_cfg.enabled,
            wake_word: voice_cfg.wake_word.clone(),
            ..Self::default()
        }
    }
}
