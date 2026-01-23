use thiserror::Error;

/// Tool execution errors
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool timeout after {0}s")]
    Timeout(u64),

    #[error("Invalid tool arguments: {0}")]
    InvalidArguments(String),

    #[error("Tool validation failed: {0}")]
    ValidationFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for tool operations
pub type ToolResult<T> = Result<T, ToolError>;
