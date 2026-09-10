//! Configuration for the NAINA OS model-runtime package.

/// Configuration parameters for the model runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRuntimeConfig {
    pub max_vram_bytes: usize,
    pub default_model: String,
}

impl Default for ModelRuntimeConfig {
    fn default() -> Self {
        Self {
            // 4.8 GB VRAM budget limit from FIRST_ALPHA_SPEC.md
            max_vram_bytes: 4_800_000_000,
            default_model: "qwen-7b-gguf".to_string(),
        }
    }
}
