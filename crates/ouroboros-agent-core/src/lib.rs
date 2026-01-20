//! # ouroboros-agent-core
//!
//! Core agent abstractions and execution engine for the ouroboros agent framework.
//!
//! This crate provides the fundamental building blocks for creating and executing agents:
//!
//! - **Types**: Core data structures (Message, AgentConfig, AgentResponse, etc.)
//! - **Agent Trait**: The main abstraction that all agents must implement
//! - **Context**: Agent execution context with conversation history and state
//! - **Executor**: Execution engine with GIL release, retries, and timeouts
//! - **State Management**: Copy-on-Write state management using Arc
//!
//! ## Architecture Principles
//!
//! Following ouroboros architecture principles:
//! - **Zero Python Byte Handling**: All operations happen in Rust
//! - **GIL Release**: Release GIL for operations > 1ms
//! - **Copy-on-Write State**: Efficient state management using Arc
//! - **Async-first**: Built on Tokio for high-performance async execution
//!
//! ## Example
//!
//! ```rust,ignore
//! use ouroboros_agent_core::{Agent, AgentConfig, AgentContext, Message, AgentResponse};
//! use async_trait::async_trait;
//!
//! struct MyAgent {
//!     config: AgentConfig,
//! }
//!
//! #[async_trait]
//! impl Agent for MyAgent {
//!     fn config(&self) -> &AgentConfig {
//!         &self.config
//!     }
//!
//!     async fn execute(
//!         &self,
//!         context: &mut AgentContext,
//!         input: Message,
//!     ) -> AgentResult<AgentResponse> {
//!         // Agent logic here
//!         Ok(AgentResponse {
//!             content: "Hello!".to_string(),
//!             // ...
//!         })
//!     }
//! }
//! ```

pub mod agent;
pub mod context;
pub mod error;
pub mod executor;
pub mod state;
pub mod types;

// Re-export commonly used types
pub use agent::{Agent, AgentRef, BaseAgent};
pub use context::AgentContext;
pub use error::{AgentError, AgentResult};
pub use executor::AgentExecutor;
pub use state::StateManager;
pub use types::{
    AgentConfig, AgentId, AgentResponse, Message, Role, SharedState, ToolCall, ToolResult,
    TokenUsage,
};
