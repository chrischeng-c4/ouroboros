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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Status Code Mapping Tests
    // ========================================================================

    #[test]
    fn test_validation_error_status_code() {
        let error = ApiError::Validation(ValidationErrors::new());
        assert_eq!(error.status_code(), 422);
    }

    #[test]
    fn test_not_found_status_code() {
        let error = ApiError::NotFound("/users/123".to_string());
        assert_eq!(error.status_code(), 404);
    }

    #[test]
    fn test_method_not_allowed_status_code() {
        let error = ApiError::MethodNotAllowed("POST".to_string());
        assert_eq!(error.status_code(), 405);
    }

    #[test]
    fn test_bad_request_status_code() {
        let error = ApiError::BadRequest("Invalid JSON".to_string());
        assert_eq!(error.status_code(), 400);
    }

    #[test]
    fn test_unauthorized_status_code() {
        let error = ApiError::Unauthorized;
        assert_eq!(error.status_code(), 401);
    }

    #[test]
    fn test_forbidden_status_code() {
        let error = ApiError::Forbidden;
        assert_eq!(error.status_code(), 403);
    }

    #[test]
    fn test_internal_error_status_code() {
        let error = ApiError::Internal("Database connection failed".to_string());
        assert_eq!(error.status_code(), 500);
    }

    #[test]
    fn test_serialization_error_status_code() {
        let error = ApiError::Serialization("JSON error".to_string());
        assert_eq!(error.status_code(), 500);
    }

    #[test]
    fn test_handler_error_status_code() {
        let error = ApiError::Handler("Handler panicked".to_string());
        assert_eq!(error.status_code(), 500);
    }

    // ========================================================================
    // ApiError Display Tests
    // ========================================================================

    #[test]
    fn test_api_error_display_validation() {
        let error = ApiError::Validation(ValidationErrors::new());
        let display = format!("{}", error);
        assert!(display.contains("Validation"));
    }

    #[test]
    fn test_api_error_display_not_found() {
        let error = ApiError::NotFound("/api/v1/users".to_string());
        let display = format!("{}", error);
        assert!(display.contains("/api/v1/users"));
    }

    #[test]
    fn test_api_error_display_method_not_allowed() {
        let error = ApiError::MethodNotAllowed("DELETE".to_string());
        let display = format!("{}", error);
        assert!(display.contains("DELETE"));
    }

    #[test]
    fn test_api_error_display_internal() {
        let error = ApiError::Internal("Unexpected error".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Unexpected error"));
    }

    // ========================================================================
    // ValidationErrors Tests
    // ========================================================================

    #[test]
    fn test_validation_errors_new() {
        let errors = ValidationErrors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.errors.len(), 0);
    }

    #[test]
    fn test_validation_errors_default() {
        let errors = ValidationErrors::default();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validation_errors_add() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError {
            location: "body".to_string(),
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
            error_type: "value_error".to_string(),
        });

        assert!(!errors.is_empty());
        assert_eq!(errors.errors.len(), 1);
    }

    #[test]
    fn test_validation_errors_multiple() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError {
            location: "body".to_string(),
            field: "email".to_string(),
            message: "Required field".to_string(),
            error_type: "missing".to_string(),
        });
        errors.add(ValidationError {
            location: "query".to_string(),
            field: "page".to_string(),
            message: "Must be positive".to_string(),
            error_type: "value_error".to_string(),
        });

        assert_eq!(errors.errors.len(), 2);
    }

    #[test]
    fn test_validation_errors_display() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError {
            location: "body".to_string(),
            field: "name".to_string(),
            message: "Too short".to_string(),
            error_type: "value_error".to_string(),
        });
        errors.add(ValidationError {
            location: "body".to_string(),
            field: "age".to_string(),
            message: "Must be positive".to_string(),
            error_type: "value_error".to_string(),
        });

        let display = format!("{}", errors);
        assert!(display.contains("2 validation errors"));
    }

    // ========================================================================
    // ValidationError Tests
    // ========================================================================

    #[test]
    fn test_validation_error_fields() {
        let error = ValidationError {
            location: "path".to_string(),
            field: "user_id".to_string(),
            message: "Invalid UUID format".to_string(),
            error_type: "type_error".to_string(),
        };

        assert_eq!(error.location, "path");
        assert_eq!(error.field, "user_id");
        assert_eq!(error.message, "Invalid UUID format");
        assert_eq!(error.error_type, "type_error");
    }

    #[test]
    fn test_validation_error_clone() {
        let original = ValidationError {
            location: "header".to_string(),
            field: "authorization".to_string(),
            message: "Missing".to_string(),
            error_type: "missing".to_string(),
        };

        let cloned = original.clone();
        assert_eq!(cloned.location, original.location);
        assert_eq!(cloned.field, original.field);
        assert_eq!(cloned.message, original.message);
        assert_eq!(cloned.error_type, original.error_type);
    }

    #[test]
    fn test_validation_error_debug() {
        let error = ValidationError {
            location: "query".to_string(),
            field: "limit".to_string(),
            message: "Must be less than 100".to_string(),
            error_type: "value_error".to_string(),
        };

        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ValidationError"));
        assert!(debug_str.contains("limit"));
    }

    // ========================================================================
    // Error Type Tests (Common Pydantic-compatible types)
    // ========================================================================

    #[test]
    fn test_error_type_missing() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError {
            location: "body".to_string(),
            field: "required_field".to_string(),
            message: "Field is required".to_string(),
            error_type: "missing".to_string(),
        });

        assert_eq!(errors.errors[0].error_type, "missing");
    }

    #[test]
    fn test_error_type_type_error() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError {
            location: "body".to_string(),
            field: "age".to_string(),
            message: "Expected integer, got string".to_string(),
            error_type: "type_error".to_string(),
        });

        assert_eq!(errors.errors[0].error_type, "type_error");
    }

    #[test]
    fn test_error_type_value_error() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError {
            location: "body".to_string(),
            field: "score".to_string(),
            message: "Value must be between 0 and 100".to_string(),
            error_type: "value_error".to_string(),
        });

        assert_eq!(errors.errors[0].error_type, "value_error");
    }

    // ========================================================================
    // ApiResult Tests
    // ========================================================================

    #[test]
    fn test_api_result_ok() {
        let result: ApiResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_api_result_err() {
        let result: ApiResult<i32> = Err(ApiError::NotFound("Not found".to_string()));
        assert!(result.is_err());
    }

    // ========================================================================
    // ApiError Debug Tests
    // ========================================================================

    #[test]
    fn test_api_error_debug() {
        let error = ApiError::BadRequest("Invalid input".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("BadRequest"));
        assert!(debug_str.contains("Invalid input"));
    }
}
