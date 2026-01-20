use thiserror::Error;

/// LLM provider errors
#[derive(Error, Debug)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Streaming error: {0}")]
    StreamingError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Timeout error")]
    TimeoutError,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for LLM operations
pub type LLMResult<T> = Result<T, LLMError>;

impl LLMError {
    /// Check if the error is retriable
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            LLMError::RateLimitError(_) | LLMError::HttpError(_) | LLMError::TimeoutError
        )
    }

    /// Check if the error is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, LLMError::RateLimitError(_))
    }
}
