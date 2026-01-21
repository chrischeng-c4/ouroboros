use crate::error::{LLMError, LLMResult};
use crate::provider::{CompletionRequest, CompletionResponse, LLMProvider, StreamChunk};
use async_trait::async_trait;
use futures::StreamExt;
use ouroboros_agent_core::{Message, Role, ToolCall, TokenUsage};
use ouroboros_http::{HttpClient, HttpClientConfig, RequestBuilder, HttpMethod};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Anthropic Claude LLM provider
///
/// Supports Claude 3.5 Sonnet, Claude 3 Opus, Claude 3 Sonnet, Claude 3 Haiku
pub struct ClaudeProvider {
    client: Arc<HttpClient>,
    api_key: String,
    default_model: String,
}

impl ClaudeProvider {
    /// Create a new Claude provider with API key
    pub fn new(api_key: impl Into<String>) -> LLMResult<Self> {
        let config = HttpClientConfig::new()
            .base_url("https://api.anthropic.com")
            .timeout_secs(120.0); // Claude can take longer

        let client = HttpClient::new(config)
            .map_err(|e| LLMError::HttpError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
            api_key: api_key.into(),
            default_model: "claude-3-5-sonnet-20241022".to_string(),
        })
    }

    /// Set default model
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Convert agent message to Claude message format
    fn convert_message(&self, msg: &Message) -> LLMResult<ClaudeMessage> {
        match msg.role {
            Role::System => {
                // System messages are handled separately in Claude API
                Err(LLMError::InvalidRequest(
                    "System messages should be passed as system parameter".to_string(),
                ))
            }
            Role::User => Ok(ClaudeMessage {
                role: "user".to_string(),
                content: vec![ClaudeContent::Text {
                    text: msg.content.clone(),
                }],
            }),
            Role::Assistant => {
                let mut content = vec![];

                // Add text content if present
                if !msg.content.is_empty() {
                    content.push(ClaudeContent::Text {
                        text: msg.content.clone(),
                    });
                }

                // Convert tool calls if present
                if let Some(tool_calls) = &msg.tool_calls {
                    for tool_call in tool_calls {
                        content.push(ClaudeContent::ToolUse {
                            id: tool_call.id.clone(),
                            name: tool_call.name.clone(),
                            input: serde_json::from_str(&tool_call.arguments).unwrap_or_default(),
                        });
                    }
                }

                Ok(ClaudeMessage {
                    role: "assistant".to_string(),
                    content,
                })
            }
            Role::Tool => {
                // Tool results are passed as user messages with tool_result content
                Ok(ClaudeMessage {
                    role: "user".to_string(),
                    content: vec![ClaudeContent::ToolResult {
                        tool_use_id: msg
                            .tool_call_id
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        content: msg.content.clone(),
                    }],
                })
            }
        }
    }

    /// Convert tool definition to Claude tool format
    fn convert_tool(&self, tool: &serde_json::Value) -> LLMResult<ClaudeTool> {
        // Extract tool information from the JSON
        let name = tool
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LLMError::InvalidRequest("Tool missing name".to_string()))?
            .to_string();

        let description = tool
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Get parameters schema
        let input_schema = tool
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

        Ok(ClaudeTool {
            name,
            description,
            input_schema,
        })
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-sonnet-20240620".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]
    }

    async fn complete(&self, request: CompletionRequest) -> LLMResult<CompletionResponse> {
        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());

        debug!("Claude completion request: model={}", model);

        // Separate system message from conversation messages
        let mut system_content = String::new();
        let mut claude_messages = Vec::new();

        for msg in &request.messages {
            if msg.role == Role::System {
                if !system_content.is_empty() {
                    system_content.push_str("\n\n");
                }
                system_content.push_str(&msg.content);
            } else {
                claude_messages.push(self.convert_message(msg)?);
            }
        }

        // Convert tools if present
        let tools = if let Some(tools_json) = &request.tools {
            let tools_vec: Vec<ClaudeTool> = tools_json
                .as_array()
                .ok_or_else(|| LLMError::InvalidRequest("Tools must be an array".to_string()))?
                .iter()
                .map(|t| self.convert_tool(t))
                .collect::<LLMResult<Vec<_>>>()?;
            Some(tools_vec)
        } else {
            None
        };

        // Build request body
        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "messages": claude_messages,
        });

        if !system_content.is_empty() {
            body["system"] = serde_json::Value::String(system_content);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if let Some(tools) = tools {
            body["tools"] = serde_json::to_value(tools)
                .map_err(|e| LLMError::SerializationError(e.to_string()))?;
        }

        // Make API request
        let response = self
            .client
            .post("/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .map_err(|e| LLMError::NetworkError(e.to_string()))?
            .await
            .map_err(|e| LLMError::NetworkError(e.to_string()))?;

        if !response.is_success() {
            let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LLMError::ApiError(format!(
                "Claude API error ({}): {}",
                response.status_code,
                error_text
            )));
        }

        // Parse response
        let claude_response: ClaudeResponse = response
            .json()
            .map_err(|e| LLMError::SerializationError(e.to_string()))?;

        // Convert response to our format
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for content_block in claude_response.content {
            match content_block {
                ClaudeContent::Text { text } => {
                    if !content.is_empty() {
                        content.push_str("\n");
                    }
                    content.push_str(&text);
                }
                ClaudeContent::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: serde_json::to_string(&input)
                            .unwrap_or_else(|_| "{}".to_string()),
                    });
                }
                _ => {} // Ignore other content types
            }
        }

        let finish_reason = match claude_response.stop_reason.as_deref() {
            Some("end_turn") => "stop",
            Some("max_tokens") => "length",
            Some("tool_use") => "tool_calls",
            Some("stop_sequence") => "stop",
            _ => "unknown",
        }
        .to_string();

        Ok(CompletionResponse {
            content,
            finish_reason,
            model: claude_response.model,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            usage: Some(TokenUsage {
                prompt_tokens: claude_response.usage.input_tokens as u32,
                completion_tokens: claude_response.usage.output_tokens as u32,
                total_tokens: (claude_response.usage.input_tokens
                    + claude_response.usage.output_tokens) as u32,
            }),
        })
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamChunk>> + Send + Unpin>> {
        // TODO: Implement streaming for Claude
        // Claude supports streaming with Server-Sent Events (SSE)
        // We'll need to handle the stream properly
        Err(LLMError::NotImplemented(
            "Streaming not yet implemented for Claude - coming in Phase 2".to_string(),
        ))
    }
}

// Claude API request/response types

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    id: String,
    model: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ClaudeContent>,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: ClaudeUsage,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = ClaudeProvider::new("test-key");
        assert_eq!(provider.provider_name(), "anthropic");
    }

    #[test]
    fn test_supported_models() {
        let provider = ClaudeProvider::new("test-key");
        let models = provider.supported_models();
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
        assert!(models.contains(&"claude-3-opus-20240229".to_string()));
    }

    #[test]
    fn test_default_model() {
        let provider = ClaudeProvider::new("test-key")
            .with_default_model("claude-3-opus-20240229");
        assert_eq!(provider.default_model, "claude-3-opus-20240229");
    }
}
