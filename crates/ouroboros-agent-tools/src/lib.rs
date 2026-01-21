//! # ouroboros-agent-tools
//!
//! Tool registration and execution system for ouroboros-agent.
//!
//! This crate provides:
//! - Tool trait for defining custom tools
//! - Global tool registry with thread-safe access
//! - Tool executor with timeout and retry logic
//! - Built-in tools (HTTP, search, etc. in Phase 4)
//!
//! ## Example
//!
//! ```rust,ignore
//! use ouroboros_agent_tools::{FunctionTool, global_registry, ToolExecutor};
//!
//! // Create a tool
//! let tool = FunctionTool::new(
//!     "search",
//!     "Search the web",
//!     vec![],
//!     |args| Box::pin(async move {
//!         Ok(serde_json::json!({"results": []}))
//!     })
//! );
//!
//! // Register it
//! global_registry().register(Arc::new(tool))?;
//!
//! // Execute it
//! let executor = ToolExecutor::new(Arc::new(global_registry()));
//! let result = executor.execute("search", args).await?;
//! ```

pub mod error;
pub mod executor;
pub mod registry;
pub mod tool;

// Re-export commonly used types
pub use error::{ToolError, ToolResult};
pub use executor::ToolExecutor;
pub use registry::{global_registry, ToolRegistry};
pub use tool::{FunctionTool, Tool, ToolDefinition, ToolParameter};
