//! # ouroboros-agent-llm
//!
//! Unified LLM provider interface for ouroboros-agent.
//!
//! This crate provides a unified interface for multiple LLM providers:
//! - OpenAI (GPT-4, GPT-3.5, etc.)
//! - Anthropic Claude (Claude 3.5 Sonnet, Claude 3 Opus, etc.)
//!
//! ## Features
//!
//! - Unified provider trait
//! - Streaming support
//! - Tool calling integration
//! - Efficient HTTP client using ouroboros-http
//!
//! ## Example
//!
//! ```rust,ignore
//! use ouroboros_agent_llm::{OpenAIProvider, LLMProvider, CompletionRequest};
//! use ouroboros_agent_core::Message;
//!
//! let provider = OpenAIProvider::new("your-api-key");
//! let request = CompletionRequest::new(
//!     vec![Message::user("Hello!")],
//!     "gpt-4"
//! );
//! let response = provider.complete(request).await?;
//! ```

// TODO: Re-enable when Claude provider compilation errors are fixed
// pub mod claude;
pub mod error;
pub mod openai;
pub mod provider;

// Re-export commonly used types
// pub use claude::ClaudeProvider;
pub use error::{LLMError, LLMResult};
pub use openai::OpenAIProvider;
pub use provider::{
    CompletionRequest, CompletionResponse, LLMProvider, StreamChunk, ToolDefinition,
};
