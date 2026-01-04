//! API framework error types

use thiserror::Error;

/// API framework result type
pub type ApiResult<T> = Result<T, ApiError>;

/// API framework errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Validation error")]
    Validation(ValidationErrors),

    #[error("Route not found: {0}")]
    NotFound(String),

    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Handler error: {0}")]
    Handler(String),
}

impl ApiError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            ApiError::Validation(_) => 422,
            ApiError::NotFound(_) => 404,
            ApiError::MethodNotAllowed(_) => 405,
            ApiError::BadRequest(_) => 400,
            ApiError::Unauthorized => 401,
            ApiError::Forbidden => 403,
            ApiError::Internal(_) | ApiError::Serialization(_) | ApiError::Handler(_) => 500,
        }
    }
}

/// Collection of validation errors (Pydantic-compatible format)
#[derive(Debug, Clone)]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add(&mut self, error: ValidationError) {
        self.errors.push(error);
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} validation errors", self.errors.len())
    }
}

/// Single validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Location: "path", "query", "body", "header"
    pub location: String,
    /// Field name or path
    pub field: String,
    /// Error message
    pub message: String,
    /// Error type: "type_error", "value_error", "missing"
    pub error_type: String,
}
