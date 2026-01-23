use thiserror::Error;

/// Core agent errors
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent execution failed: {0}")]
    ExecutionError(String),

    #[error("Tool execution failed: {0}")]
    ToolError(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool timeout after {0}s")]
    ToolTimeout(u64),

    #[error("LLM provider error: {0}")]
    LLMError(String),

    #[error("Invalid agent state: {0}")]
    InvalidState(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Maximum turns reached: {0}")]
    MaxTurnsReached(u32),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Async runtime error: {0}")]
    RuntimeError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

impl AgentError {
    /// Check if the error is retriable
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            AgentError::LLMError(_) | AgentError::ToolTimeout(_) | AgentError::RuntimeError(_)
        )
    }

    /// Check if the error is a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, AgentError::ToolTimeout(_))
    }
}
