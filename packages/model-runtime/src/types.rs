//! Data types for the NAINA OS model-runtime package.

use std::sync::mpsc::Receiver;

/// Inference hyper-parameters for a model execution request.
#[derive(Clone, Debug, PartialEq)]
pub struct InferenceParams {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: usize,
    pub stop_sequences: Vec<String>,
}

impl Default for InferenceParams {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 512,
            stop_sequences: Vec::new(),
        }
    }
}

/// Prompt request payload dispatched to a model provider.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelRequest {
    pub model_name: String,
    pub prompt: String,
    pub params: InferenceParams,
}

/// Reason for inference completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    Error,
}

/// Token accounting metrics for an inference completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Completion response payload returned from a model provider.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelResponse {
    pub text: String,
    pub tokens_generated: usize,
    pub finish_reason: FinishReason,
    pub usage: Option<TokenUsage>,
}

/// Detailed inference response payload with latency breakdown.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailedModelResponse {
    pub response: ModelResponse,
    pub ttft: std::time::Duration,
    pub token_generation_duration: std::time::Duration,
}

/// Token streaming handle wrapping a thread-safe token receiver.
#[derive(Debug)]
pub struct TokenStream {
    pub receiver: Receiver<String>,
}
