//! Handler trait and metadata
//!
//! Handlers are the core units of request processing.

use crate::request::Request;
use crate::response::Response;
use crate::error::ApiResult;
use async_trait::async_trait;

/// Handler trait for processing requests
#[async_trait]
pub trait Handler: Send + Sync {
    /// Process a request and return a response
    async fn handle(&self, req: Request) -> ApiResult<Response>;
}

/// Handler metadata (extracted from Python function at registration time)
#[derive(Debug, Clone)]
pub struct HandlerMeta {
    /// Handler name (function name)
    pub name: String,
    /// Path parameters (e.g., ["user_id", "post_id"] for /users/{user_id}/posts/{post_id})
    pub path_params: Vec<String>,
    /// Query parameters with types
    pub query_params: Vec<ParamMeta>,
    /// Request body schema (if any)
    pub body_schema: Option<String>,
    /// Response schema
    pub response_schema: Option<String>,
    /// OpenAPI summary
    pub summary: Option<String>,
    /// OpenAPI description
    pub description: Option<String>,
    /// OpenAPI tags
    pub tags: Vec<String>,
    /// OpenAPI operation ID
    pub operation_id: Option<String>,
    /// Response description
    pub response_description: Option<String>,
    /// Default response status code
    pub status_code: u16,
    /// Deprecated flag
    pub deprecated: bool,
}

/// Parameter metadata
#[derive(Debug, Clone)]
pub struct ParamMeta {
    pub name: String,
    pub param_type: String,  // "str", "int", "float", "bool"
    pub required: bool,
    pub default: Option<String>,
}

impl HandlerMeta {
    /// Create new handler metadata
    pub fn new(name: String) -> Self {
        Self {
            name,
            path_params: Vec::new(),
            query_params: Vec::new(),
            body_schema: None,
            response_schema: None,
            summary: None,
            description: None,
            tags: Vec::new(),
            operation_id: None,
            response_description: None,
            status_code: 200,
            deprecated: false,
        }
    }
}
