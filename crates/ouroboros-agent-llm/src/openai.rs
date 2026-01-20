use crate::error::{LLMError, LLMResult};
use crate::provider::{CompletionRequest, CompletionResponse, LLMProvider, StreamChunk};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, ChatCompletionRequestAssistantMessage,
        ChatCompletionRequestToolMessage,
        CreateChatCompletionRequestArgs, ChatCompletionTool, ChatCompletionToolType,
        FunctionObject,
    },
    Client,
};
use async_trait::async_trait;
use futures::StreamExt;
use ouroboros_agent_core::{Message, Role, ToolCall, TokenUsage};
use std::sync::Arc;
use tracing::debug;

/// OpenAI LLM provider
pub struct OpenAIProvider {
    client: Arc<Client<OpenAIConfig>>,
    default_model: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key.into());
        let client = Client::with_config(config);

        Self {
            client: Arc::new(client),
            default_model: "gpt-4".to_string(),
        }
    }

    /// Create a new OpenAI provider with custom configuration
    pub fn with_config(config: OpenAIConfig) -> Self {
        Self {
            client: Arc::new(Client::with_config(config)),
            default_model: "gpt-4".to_string(),
        }
    }

    /// Set default model
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Convert agent message to OpenAI message format
    fn convert_message(&self, msg: &Message) -> LLMResult<ChatCompletionRequestMessage> {
        match msg.role {
            Role::System => Ok(ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessage {
                    content: msg.content.clone().into(),
                    name: msg.name.clone(),
                },
            )),
            Role::User => Ok(ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: msg.content.clone().into(),
                    name: msg.name.clone(),
                },
            )),
            Role::Assistant => {
                let mut assistant_msg = ChatCompletionRequestAssistantMessage {
                    content: Some(msg.content.clone().into()),
                    name: msg.name.clone(),
                    tool_calls: None,
                    refusal: None,
                    audio: None,
                    function_call: None,
                };

                // Convert tool calls if present
                if let Some(tool_calls) = &msg.tool_calls {
                    assistant_msg.tool_calls = Some(
                        tool_calls
                            .iter()
                            .map(|tc| async_openai::types::ChatCompletionMessageToolCall {
                                id: tc.id.clone(),
                                r#type: async_openai::types::ChatCompletionToolType::Function,
                                function: async_openai::types::FunctionCall {
                                    name: tc.name.clone(),
                                    arguments: tc.arguments.to_string(),
                                },
                            })
                            .collect(),
                    );
                }

                Ok(ChatCompletionRequestMessage::Assistant(assistant_msg))
            }
            Role::Tool => {
                let tool_call_id = msg
                    .tool_call_id
                    .clone()
                    .ok_or_else(|| LLMError::InvalidRequest("Tool message missing tool_call_id".into()))?;

                Ok(ChatCompletionRequestMessage::Tool(
                    ChatCompletionRequestToolMessage {
                        content: msg.content.clone().into(),
                        tool_call_id,
                    },
                ))
            }
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn provider_name(&self) -> &str {
        "openai"
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-4-turbo-preview".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-3.5-turbo-16k".to_string(),
        ]
    }

    async fn complete(&self, request: CompletionRequest) -> LLMResult<CompletionResponse> {
        debug!("OpenAI completion request for model: {}", request.model);

        // Validate model
        self.validate_model(&request.model)?;

        // Convert messages
        let messages: Vec<ChatCompletionRequestMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect::<LLMResult<Vec<_>>>()?;

        // Build request
        let mut req_builder = CreateChatCompletionRequestArgs::default();
        req_builder.model(&request.model).messages(messages);

        if let Some(temp) = request.temperature {
            req_builder.temperature(temp);
        }

        if let Some(max_tokens) = request.max_tokens {
            req_builder.max_tokens(max_tokens);
        }

        if let Some(top_p) = request.top_p {
            req_builder.top_p(top_p);
        }

        if let Some(stop) = request.stop {
            req_builder.stop(stop);
        }

        // Convert tools if present
        if let Some(tools) = request.tools {
            let openai_tools: Vec<ChatCompletionTool> = tools
                .iter()
                .map(|t| ChatCompletionTool {
                    r#type: ChatCompletionToolType::Function,
                    function: FunctionObject {
                        name: t.name.clone(),
                        description: Some(t.description.clone()),
                        parameters: Some(t.parameters.clone()),
                        strict: None,
                    },
                })
                .collect();
            req_builder.tools(openai_tools);
        }

        let req = req_builder
            .build()
            .map_err(|e| LLMError::InvalidRequest(e.to_string()))?;

        // Make API call
        let response = self
            .client
            .chat()
            .create(req)
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        // Extract first choice
        let choice = response
            .choices
            .first()
            .ok_or_else(|| LLMError::ApiError("No choices in response".into()))?;

        let content = choice
            .message
            .content
            .clone()
            .unwrap_or_default()
            .to_string();

        // Extract tool calls if present
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .map(|tc| ToolCall {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: serde_json::from_str(&tc.function.arguments)
                        .unwrap_or_else(|_| serde_json::json!({})),
                })
                .collect()
        });

        // Extract usage stats
        let usage = response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }).unwrap_or_default();

        Ok(CompletionResponse {
            content,
            tool_calls,
            finish_reason: choice.finish_reason.as_ref().map(|f| format!("{:?}", f)).unwrap_or_else(|| "stop".to_string()),
            usage,
            model: response.model,
            metadata: Default::default(),
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamChunk>> + Send + Unpin>> {
        debug!("OpenAI streaming completion request for model: {}", request.model);

        // Validate model
        self.validate_model(&request.model)?;

        // Convert messages
        let messages: Vec<ChatCompletionRequestMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect::<LLMResult<Vec<_>>>()?;

        // Build request (similar to complete, but with stream=true)
        let mut req_builder = CreateChatCompletionRequestArgs::default();
        req_builder.model(&request.model).messages(messages).stream(true);

        if let Some(temp) = request.temperature {
            req_builder.temperature(temp);
        }

        if let Some(max_tokens) = request.max_tokens {
            req_builder.max_tokens(max_tokens);
        }

        let req = req_builder
            .build()
            .map_err(|e| LLMError::InvalidRequest(e.to_string()))?;

        // Make streaming API call
        let stream = self
            .client
            .chat()
            .create_stream(req)
            .await
            .map_err(|e| LLMError::StreamingError(e.to_string()))?;

        // Transform stream
        let chunk_stream = stream.map(|result| match result {
            Ok(response) => {
                let choice = response.choices.first();
                let content = choice
                    .and_then(|c| c.delta.content.clone())
                    .unwrap_or_default();

                let finish_reason = choice
                    .and_then(|c| c.finish_reason.as_ref().map(|f| format!("{:?}", f)));

                Ok(StreamChunk {
                    content,
                    tool_calls: None,
                    finish_reason: finish_reason.clone(),
                    is_final: finish_reason.is_some(),
                })
            }
            Err(e) => Err(LLMError::StreamingError(e.to_string())),
        });

        Ok(Box::new(Box::pin(chunk_stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = OpenAIProvider::new("test-key");
        assert_eq!(provider.provider_name(), "openai");
    }

    #[test]
    fn test_supported_models() {
        let provider = OpenAIProvider::new("test-key");
        let models = provider.supported_models();
        assert!(models.contains(&"gpt-4".to_string()));
        assert!(models.contains(&"gpt-3.5-turbo".to_string()));
    }
}
