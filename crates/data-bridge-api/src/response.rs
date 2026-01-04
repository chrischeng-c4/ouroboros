//! HTTP response representation with two-phase GIL pattern
//!
//! This module provides intermediate representation types for HTTP responses
//! that enable GIL-free processing, following the same pattern as
//! `crates/data-bridge/src/conversion.rs`.
//!
//! # Architecture
//!
//! ## Response Processing (Rust → Python)
//! 1. **Build** (GIL held, <1ms): Python values → `SerializableResponse`
//! 2. **Serialize** (GIL released): `SerializableResponse` → HTTP bytes
//!
//! # Example
//!
//! ```rust
//! use data_bridge_api::request::SerializableValue;
//! use data_bridge_api::response::Response;
//!
//! // Build response with GIL held
//! let response = Response::json(SerializableValue::Object(vec![
//!     ("status".to_string(), SerializableValue::String("ok".to_string())),
//! ]));
//!
//! // Serialize with GIL released
//! let bytes = response.into_serializable().body_bytes();
//! ```

use std::collections::HashMap;
use crate::request::SerializableValue;
use crate::error::ApiError;

// ============================================================================
// Core Types
// ============================================================================

/// Serializable HTTP response (GIL-free processing)
///
/// Built in Python with GIL, serialized without GIL.
/// All fields are `Send + Sync` for cross-thread usage.
#[derive(Debug, Clone)]
pub struct SerializableResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers (lowercase keys)
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: ResponseBody,
}

/// Response body variants
#[derive(Debug, Clone)]
pub enum ResponseBody {
    /// No body (for 204 No Content, 304 Not Modified, etc.)
    Empty,
    /// JSON body (will be serialized)
    Json(SerializableValue),
    /// Raw bytes (for binary data, files, etc.)
    Bytes(Vec<u8>),
    /// Plain text (UTF-8)
    Text(String),
}

impl SerializableResponse {
    /// Create an empty response with status code
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: HashMap::new(),
            body: ResponseBody::Empty,
        }
    }

    /// Create a JSON response (200 OK)
    pub fn json(body: SerializableValue) -> Self {
        let mut resp = Self::new(200);
        resp.headers.insert(
            "content-type".to_string(),
            "application/json".to_string(),
        );
        resp.body = ResponseBody::Json(body);
        resp
    }

    /// Create a text response (200 OK)
    pub fn text(body: impl Into<String>) -> Self {
        let mut resp = Self::new(200);
        resp.headers.insert(
            "content-type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );
        resp.body = ResponseBody::Text(body.into());
        resp
    }

    /// Create a bytes response (200 OK)
    pub fn bytes(body: Vec<u8>, content_type: impl Into<String>) -> Self {
        let mut resp = Self::new(200);
        resp.headers.insert("content-type".to_string(), content_type.into());
        resp.body = ResponseBody::Bytes(body);
        resp
    }

    /// Set status code (builder pattern)
    pub fn status(mut self, code: u16) -> Self {
        self.status_code = code;
        self
    }

    /// Add header (builder pattern)
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into().to_lowercase(), value.into());
        self
    }

    /// Serialize body to bytes (GIL-free operation)
    pub fn body_bytes(&self) -> Vec<u8> {
        match &self.body {
            ResponseBody::Empty => Vec::new(),
            ResponseBody::Json(value) => {
                serde_json::to_vec(&value.to_json()).unwrap_or_default()
            }
            ResponseBody::Bytes(b) => b.clone(),
            ResponseBody::Text(s) => s.as_bytes().to_vec(),
        }
    }

    /// Get content length in bytes
    pub fn content_length(&self) -> usize {
        match &self.body {
            ResponseBody::Empty => 0,
            ResponseBody::Json(value) => {
                serde_json::to_string(&value.to_json())
                    .map(|s| s.len())
                    .unwrap_or(0)
            }
            ResponseBody::Bytes(b) => b.len(),
            ResponseBody::Text(s) => s.len(),
        }
    }

    /// Check if response has a body
    pub fn has_body(&self) -> bool {
        !matches!(self.body, ResponseBody::Empty)
    }

    /// Get content type header
    pub fn content_type(&self) -> Option<&str> {
        self.headers.get("content-type").map(|s| s.as_str())
    }
}

/// High-level response builder
///
/// Provides a convenient API for building HTTP responses with common patterns.
pub struct Response {
    inner: SerializableResponse,
}

impl Response {
    /// Create a new response (200 OK, empty body)
    pub fn new() -> Self {
        Self {
            inner: SerializableResponse::new(200),
        }
    }

    /// Create a 200 OK response (empty body)
    pub fn ok() -> Self {
        Self::new()
    }

    /// Create a 201 Created response (empty body)
    pub fn created() -> Self {
        Self {
            inner: SerializableResponse::new(201),
        }
    }

    /// Create a 202 Accepted response (empty body)
    pub fn accepted() -> Self {
        Self {
            inner: SerializableResponse::new(202),
        }
    }

    /// Create a 204 No Content response
    pub fn no_content() -> Self {
        Self {
            inner: SerializableResponse::new(204),
        }
    }

    /// Create a 400 Bad Request response
    pub fn bad_request(message: impl Into<String>) -> Self {
        let body = SerializableValue::Object(vec![
            ("detail".to_string(), SerializableValue::String(message.into())),
        ]);
        Self {
            inner: SerializableResponse::json(body).status(400),
        }
    }

    /// Create a 404 Not Found response
    pub fn not_found(message: impl Into<String>) -> Self {
        let body = SerializableValue::Object(vec![
            ("detail".to_string(), SerializableValue::String(message.into())),
        ]);
        Self {
            inner: SerializableResponse::json(body).status(404),
        }
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_error(message: impl Into<String>) -> Self {
        let body = SerializableValue::Object(vec![
            ("detail".to_string(), SerializableValue::String(message.into())),
        ]);
        Self {
            inner: SerializableResponse::json(body).status(500),
        }
    }

    /// Create a JSON response (200 OK)
    pub fn json(body: SerializableValue) -> Self {
        Self {
            inner: SerializableResponse::json(body),
        }
    }

    /// Create a text response (200 OK)
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            inner: SerializableResponse::text(body),
        }
    }

    /// Create a bytes response (200 OK)
    pub fn bytes(body: Vec<u8>, content_type: impl Into<String>) -> Self {
        Self {
            inner: SerializableResponse::bytes(body, content_type),
        }
    }

    /// Create a response from an error
    pub fn error(err: &ApiError) -> Self {
        let status_code = err.status_code();
        let body = SerializableValue::Object(vec![
            ("detail".to_string(), SerializableValue::String(err.to_string())),
        ]);
        Self {
            inner: SerializableResponse::json(body).status(status_code),
        }
    }

    /// Set status code (builder pattern)
    pub fn status(mut self, code: u16) -> Self {
        self.inner.status_code = code;
        self
    }

    /// Add header (builder pattern)
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.header(name, value);
        self
    }

    /// Set JSON body (builder pattern)
    pub fn with_json(mut self, body: SerializableValue) -> Self {
        self.inner.headers.insert(
            "content-type".to_string(),
            "application/json".to_string(),
        );
        self.inner.body = ResponseBody::Json(body);
        self
    }

    /// Set text body (builder pattern)
    pub fn with_text(mut self, body: impl Into<String>) -> Self {
        self.inner.headers.insert(
            "content-type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );
        self.inner.body = ResponseBody::Text(body.into());
        self
    }

    /// Set bytes body (builder pattern)
    pub fn with_bytes(mut self, body: Vec<u8>, content_type: impl Into<String>) -> Self {
        self.inner.headers.insert("content-type".to_string(), content_type.into());
        self.inner.body = ResponseBody::Bytes(body);
        self
    }

    /// Convert to serializable response
    pub fn into_serializable(self) -> SerializableResponse {
        self.inner
    }

    /// Get reference to inner serializable response
    pub fn as_serializable(&self) -> &SerializableResponse {
        &self.inner
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SerializableResponse> for Response {
    fn from(inner: SerializableResponse) -> Self {
        Self { inner }
    }
}

impl From<Response> for SerializableResponse {
    fn from(response: Response) -> Self {
        response.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializable_response_json() {
        let value = SerializableValue::Object(vec![
            ("name".to_string(), SerializableValue::String("Alice".to_string())),
            ("age".to_string(), SerializableValue::Int(30)),
        ]);
        let resp = SerializableResponse::json(value);

        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.content_type(), Some("application/json"));
        assert!(resp.has_body());

        let bytes = resp.body_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["name"], "Alice");
        assert_eq!(json["age"], 30);
    }

    #[test]
    fn test_serializable_response_text() {
        let resp = SerializableResponse::text("Hello, World!");

        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.content_type(), Some("text/plain; charset=utf-8"));
        assert!(resp.has_body());

        let bytes = resp.body_bytes();
        assert_eq!(String::from_utf8(bytes).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_serializable_response_bytes() {
        let data = vec![1, 2, 3, 4, 5];
        let resp = SerializableResponse::bytes(data.clone(), "application/octet-stream");

        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.content_type(), Some("application/octet-stream"));
        assert!(resp.has_body());

        let bytes = resp.body_bytes();
        assert_eq!(bytes, data);
    }

    #[test]
    fn test_serializable_response_empty() {
        let resp = SerializableResponse::new(204);

        assert_eq!(resp.status_code, 204);
        assert!(!resp.has_body());
        assert_eq!(resp.content_length(), 0);

        let bytes = resp.body_bytes();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_serializable_response_builder() {
        let resp = SerializableResponse::new(201)
            .header("X-Custom-Header", "custom-value")
            .header("Content-Type", "application/json");

        assert_eq!(resp.status_code, 201);
        assert_eq!(resp.headers.get("x-custom-header"), Some(&"custom-value".to_string()));
        assert_eq!(resp.headers.get("content-type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_response_builder_ok() {
        let resp = Response::ok();
        assert_eq!(resp.inner.status_code, 200);
        assert!(!resp.inner.has_body());
    }

    #[test]
    fn test_response_builder_created() {
        let resp = Response::created();
        assert_eq!(resp.inner.status_code, 201);
    }

    #[test]
    fn test_response_builder_no_content() {
        let resp = Response::no_content();
        assert_eq!(resp.inner.status_code, 204);
    }

    #[test]
    fn test_response_builder_json() {
        let value = SerializableValue::Object(vec![
            ("status".to_string(), SerializableValue::String("success".to_string())),
        ]);
        let resp = Response::json(value);

        assert_eq!(resp.inner.status_code, 200);
        assert_eq!(resp.inner.content_type(), Some("application/json"));
    }

    #[test]
    fn test_response_builder_text() {
        let resp = Response::text("Hello");

        assert_eq!(resp.inner.status_code, 200);
        assert_eq!(resp.inner.content_type(), Some("text/plain; charset=utf-8"));
    }

    #[test]
    fn test_response_builder_error() {
        let error = ApiError::NotFound("User not found".to_string());
        let resp = Response::error(&error);

        assert_eq!(resp.inner.status_code, 404);

        let bytes = resp.inner.body_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["detail"].as_str().unwrap().contains("not found"));
    }

    #[test]
    fn test_response_builder_bad_request() {
        let resp = Response::bad_request("Invalid input");

        assert_eq!(resp.inner.status_code, 400);

        let bytes = resp.inner.body_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["detail"], "Invalid input");
    }

    #[test]
    fn test_response_builder_not_found() {
        let resp = Response::not_found("Resource not found");

        assert_eq!(resp.inner.status_code, 404);

        let bytes = resp.inner.body_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["detail"], "Resource not found");
    }

    #[test]
    fn test_response_builder_internal_error() {
        let resp = Response::internal_error("Something went wrong");

        assert_eq!(resp.inner.status_code, 500);

        let bytes = resp.inner.body_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["detail"], "Something went wrong");
    }

    #[test]
    fn test_response_builder_chain() {
        let value = SerializableValue::Object(vec![
            ("message".to_string(), SerializableValue::String("Success".to_string())),
        ]);

        let resp = Response::json(value)
            .status(201)
            .header("X-Request-ID", "12345")
            .header("X-Version", "1.0");

        assert_eq!(resp.inner.status_code, 201);
        assert_eq!(resp.inner.headers.get("x-request-id"), Some(&"12345".to_string()));
        assert_eq!(resp.inner.headers.get("x-version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_response_content_length() {
        let text_resp = SerializableResponse::text("Hello");
        assert_eq!(text_resp.content_length(), 5);

        let bytes_resp = SerializableResponse::bytes(vec![1, 2, 3], "application/octet-stream");
        assert_eq!(bytes_resp.content_length(), 3);

        let empty_resp = SerializableResponse::new(204);
        assert_eq!(empty_resp.content_length(), 0);
    }

    #[test]
    fn test_response_into_serializable() {
        let resp = Response::ok().status(201);
        let serializable = resp.into_serializable();

        assert_eq!(serializable.status_code, 201);
    }

    #[test]
    fn test_response_from_serializable() {
        let serializable = SerializableResponse::new(200);
        let resp = Response::from(serializable);

        assert_eq!(resp.inner.status_code, 200);
    }
}
