//! Desktop Host application configuration parameters and default models.

use std::fmt;

/// Configuration options for the [`DesktopHostApp`](crate::desktop_host::DesktopHostApp).
#[derive(Clone, PartialEq, Eq)]
pub struct DesktopHostConfig {
    /// File path to local Qwen 7B GGUF model weights.
    pub qwen_model_path: String,
    /// File path to Whisper STT model file.
    pub whisper_model_path: String,
    /// File path to Piper TTS model file.
    pub piper_model_path: String,
    /// Directory path to the local Obsidian Vault.
    pub obsidian_vault_path: String,
    /// Global hotkey trigger shortcut string.
    pub hotkey_trigger: String,
}

impl fmt::Debug for DesktopHostConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopHostConfig")
            .field("qwen_model_path", &self.qwen_model_path)
            .field("whisper_model_path", &self.whisper_model_path)
            .field("piper_model_path", &self.piper_model_path)
            .field("obsidian_vault_path", &self.obsidian_vault_path)
            .field("hotkey_trigger", &self.hotkey_trigger)
            .finish()
    }
}

impl Default for DesktopHostConfig {
    fn default() -> Self {
        Self {
            qwen_model_path: "models/qwen7b.gguf".to_string(),
            whisper_model_path: "models/whisper.bin".to_string(),
            piper_model_path: "models/piper.onnx".to_string(),
            obsidian_vault_path: "vault/".to_string(),
            hotkey_trigger: "Ctrl+Shift+Space".to_string(),
        }
    }
}
