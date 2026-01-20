use crate::error::{LLMError, LLMResult};
use async_trait::async_trait;
use ouroboros_agent_core::{Message, ToolCall, TokenUsage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// List of messages in the conversation
    pub messages: Vec<Message>,

    /// Model identifier (e.g., "gpt-4", "claude-3-opus")
    pub model: String,

    /// Temperature for sampling (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,

    /// Tools available for the model to call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,

    /// Additional provider-specific parameters
    #[serde(default)]
    pub extras: HashMap<String, serde_json::Value>,
}

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated content
    pub content: String,

    /// Tool calls requested by the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Finish reason (e.g., "stop", "length", "tool_calls")
    pub finish_reason: String,

    /// Token usage statistics
    pub usage: TokenUsage,

    /// Model used for generation
    pub model: String,

    /// Additional provider-specific data
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Streaming chunk from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Delta content in this chunk
    pub content: String,

    /// Tool calls delta (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Finish reason (if this is the last chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,

    /// Whether this is the final chunk
    pub is_final: bool,
}

/// Unified LLM provider trait
/// All LLM providers (OpenAI, Anthropic, etc.) must implement this trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get provider name (e.g., "openai", "anthropic")
    fn provider_name(&self) -> &str;

    /// Get supported models
    fn supported_models(&self) -> Vec<String>;

    /// Generate a completion
    async fn complete(&self, request: CompletionRequest) -> LLMResult<CompletionResponse>;

    /// Generate a streaming completion
    /// Returns a stream of chunks
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamChunk>> + Send + Unpin>>;

    /// Validate a model name is supported
    fn validate_model(&self, model: &str) -> LLMResult<()> {
        if self.supported_models().contains(&model.to_string()) {
            Ok(())
        } else {
            Err(LLMError::ModelNotFound(format!(
                "Model '{}' is not supported by provider '{}'",
                model,
                self.provider_name()
            )))
        }
    }
}

impl CompletionRequest {
    pub fn new(messages: Vec<Message>, model: impl Into<String>) -> Self {
        Self {
            messages,
            model: model.into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: false,
            tools: None,
            extras: HashMap::new(),
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}
