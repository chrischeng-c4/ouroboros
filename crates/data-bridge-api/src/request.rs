//! HTTP request representation with two-phase GIL pattern
//!
//! This module provides intermediate representation types for HTTP requests
//! that enable GIL-free processing, following the same pattern as
//! `crates/data-bridge/src/conversion.rs`.
//!
//! # Architecture
//!
//! ## Request Processing (Python → Rust)
//! 1. **Extract** (GIL held, <1ms): Python objects → `SerializableRequest`
//! 2. **Process** (GIL released): Validate, route, handle request
//!
//! # Example
//!
//! ```rust
//! use data_bridge_api::request::{HttpMethod, SerializableRequest, SerializableValue};
//!
//! // Phase 1: Extract request data (GIL held)
//! let serializable_req = SerializableRequest::new(HttpMethod::Post, "/api/users")
//!     .with_body(SerializableValue::Object(vec![
//!         ("name".to_string(), SerializableValue::String("Alice".to_string())),
//!     ]));
//!
//! // Phase 2: Process (GIL released)
//! // Handler logic runs without GIL
//! ```

use std::collections::HashMap;

// Re-export HttpMethod from data-bridge-common
pub use data_bridge_common::http::HttpMethod;

// ============================================================================
// Core Types
// ============================================================================

/// Intermediate representation for request values
///
/// All variants are `Send + Sync`, enabling GIL-free processing.
/// This is similar to `SerializablePyValue` but tailored for HTTP API values.
#[derive(Debug, Clone, PartialEq)]
pub enum SerializableValue {
    /// Null/None value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value (stored as i64)
    Int(i64),
    /// Floating point value
    Float(f64),
    /// String value (UTF-8)
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Array of values
    List(Vec<SerializableValue>),
    /// Object with key-value pairs (preserves insertion order)
    Object(Vec<(String, SerializableValue)>),
}

impl SerializableValue {
    /// Convert to JSON value (for body processing)
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(*b),
            Self::Int(i) => serde_json::Value::Number((*i).into()),
            Self::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            Self::String(s) => serde_json::Value::String(s.clone()),
            Self::Bytes(b) => {
                // Base64 encode bytes for JSON representation
                use base64::Engine;
                serde_json::Value::String(
                    base64::engine::general_purpose::STANDARD.encode(b)
                )
            }
            Self::List(items) => {
                serde_json::Value::Array(items.iter().map(|v| v.to_json()).collect())
            }
            Self::Object(pairs) => {
                let map: serde_json::Map<String, serde_json::Value> = pairs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_json()))
                    .collect();
                serde_json::Value::Object(map)
            }
        }
    }

    /// Create from JSON value
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Self::Float(f)
                } else {
                    Self::Null
                }
            }
            serde_json::Value::String(s) => Self::String(s.clone()),
            serde_json::Value::Array(arr) => {
                Self::List(arr.iter().map(Self::from_json).collect())
            }
            serde_json::Value::Object(obj) => {
                Self::Object(
                    obj.iter()
                        .map(|(k, v)| (k.clone(), Self::from_json(v)))
                        .collect(),
                )
            }
        }
    }
}

/// Serializable HTTP request (GIL-free processing)
///
/// Extracted with GIL held, validated and processed without GIL.
/// All fields are `Send + Sync` for cross-thread usage.
#[derive(Debug, Clone)]
pub struct SerializableRequest {
    /// HTTP method
    pub method: HttpMethod,
    /// Request path (without query string)
    pub path: String,
    /// Full URL
    pub url: String,
    /// Path parameters (from route matching)
    pub path_params: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, SerializableValue>,
    /// HTTP headers (lowercase keys)
    pub headers: HashMap<String, String>,
    /// Request body (if any)
    pub body: Option<SerializableValue>,
    /// Content-Type header value
    pub content_type: Option<String>,
}

impl SerializableRequest {
    /// Create a new request
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            url: String::new(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            content_type: None,
        }
    }

    /// Set the full URL
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Add a path parameter
    pub fn with_path_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.path_params.insert(name.into(), value.into());
        self
    }

    /// Add a query parameter
    pub fn with_query_param(mut self, name: impl Into<String>, value: SerializableValue) -> Self {
        self.query_params.insert(name.into(), value);
        self
    }

    /// Add a header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into().to_lowercase();
        let value = value.into();

        // Update content_type if this is a Content-Type header
        if name == "content-type" {
            self.content_type = Some(value.clone());
        }

        self.headers.insert(name, value);
        self
    }

    /// Set the request body
    pub fn with_body(mut self, body: SerializableValue) -> Self {
        self.body = Some(body);
        self
    }

    /// Get a path parameter as string
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name).map(|s| s.as_str())
    }

    /// Get a query parameter
    pub fn query_param(&self, name: &str) -> Option<&SerializableValue> {
        self.query_params.get(name)
    }

    /// Get a header value (case-insensitive)
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    /// Check if request has JSON content type
    pub fn is_json(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    /// Get body as JSON value
    pub fn body_json(&self) -> Option<serde_json::Value> {
        self.body.as_ref().map(|v| v.to_json())
    }
}

/// High-level request wrapper with app state
///
/// This wraps `SerializableRequest` and adds application-level features
/// like shared state management.
pub struct Request {
    /// Serializable request data
    pub inner: SerializableRequest,
    /// Application state (Arc shared)
    state: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}

impl Request {
    /// Create a new request from serializable data
    pub fn new(inner: SerializableRequest) -> Self {
        Self { inner, state: None }
    }

    /// Set application state
    pub fn with_state(mut self, state: std::sync::Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.state = Some(state);
        self
    }

    /// Get application state (typed)
    pub fn state<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.state.as_ref().and_then(|s| s.downcast_ref::<T>())
    }

    // Delegate to inner

    /// Get HTTP method
    pub fn method(&self) -> HttpMethod {
        self.inner.method
    }

    /// Get request path
    pub fn path(&self) -> &str {
        &self.inner.path
    }

    /// Get full URL
    pub fn url(&self) -> &str {
        &self.inner.url
    }

    /// Get a path parameter
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.inner.path_param(name)
    }

    /// Get a query parameter
    pub fn query_param(&self, name: &str) -> Option<&SerializableValue> {
        self.inner.query_param(name)
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.inner.header(name)
    }

    /// Get request body
    pub fn body(&self) -> Option<&SerializableValue> {
        self.inner.body.as_ref()
    }

    /// Check if request has JSON content type
    pub fn is_json(&self) -> bool {
        self.inner.is_json()
    }

    /// Get body as JSON value
    pub fn body_json(&self) -> Option<serde_json::Value> {
        self.inner.body_json()
    }
}

impl From<SerializableRequest> for Request {
    fn from(inner: SerializableRequest) -> Self {
        Self::new(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_from_str() {
        use std::str::FromStr;
        assert_eq!(HttpMethod::from_str("GET"), Ok(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("post"), Ok(HttpMethod::Post));
        assert_eq!(HttpMethod::from_str("PATCH"), Ok(HttpMethod::Patch));
        assert!(HttpMethod::from_str("INVALID").is_err());
    }

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn test_serializable_value_to_json() {
        let value = SerializableValue::Object(vec![
            ("name".to_string(), SerializableValue::String("Alice".to_string())),
            ("age".to_string(), SerializableValue::Int(30)),
            ("active".to_string(), SerializableValue::Bool(true)),
        ]);

        let json = value.to_json();
        assert_eq!(json["name"], "Alice");
        assert_eq!(json["age"], 30);
        assert_eq!(json["active"], true);
    }

    #[test]
    fn test_serializable_value_from_json() {
        let json = serde_json::json!({
            "name": "Alice",
            "age": 30,
            "active": true
        });

        let value = SerializableValue::from_json(&json);

        if let SerializableValue::Object(pairs) = value {
            assert_eq!(pairs.len(), 3);

            // Check all fields are present (order may vary)
            let name_pair = pairs.iter().find(|(k, _)| k == "name").unwrap();
            assert_eq!(name_pair.1, SerializableValue::String("Alice".to_string()));

            let age_pair = pairs.iter().find(|(k, _)| k == "age").unwrap();
            assert_eq!(age_pair.1, SerializableValue::Int(30));

            let active_pair = pairs.iter().find(|(k, _)| k == "active").unwrap();
            assert_eq!(active_pair.1, SerializableValue::Bool(true));
        } else {
            panic!("Expected Object variant");
        }
    }

    #[test]
    fn test_serializable_request_builder() {
        let req = SerializableRequest::new(HttpMethod::Post, "/api/users")
            .with_url("https://example.com/api/users")
            .with_path_param("id", "123")
            .with_query_param("limit", SerializableValue::Int(10))
            .with_header("Content-Type", "application/json")
            .with_body(SerializableValue::Object(vec![
                ("name".to_string(), SerializableValue::String("Alice".to_string())),
            ]));

        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/users");
        assert_eq!(req.url, "https://example.com/api/users");
        assert_eq!(req.path_param("id"), Some("123"));
        assert_eq!(req.query_param("limit"), Some(&SerializableValue::Int(10)));
        assert_eq!(req.header("content-type"), Some("application/json"));
        assert!(req.is_json());
        assert!(req.body.is_some());
    }

    #[test]
    fn test_request_wrapper() {
        let serializable = SerializableRequest::new(HttpMethod::Get, "/api/test");
        let req = Request::new(serializable);

        assert_eq!(req.method(), HttpMethod::Get);
        assert_eq!(req.path(), "/api/test");
        assert!(req.body().is_none());
    }

    #[test]
    fn test_request_with_state() {
        #[derive(Debug)]
        struct AppState {
            counter: u32,
        }

        let state = std::sync::Arc::new(AppState { counter: 42 });
        let serializable = SerializableRequest::new(HttpMethod::Get, "/api/test");
        let req = Request::new(serializable).with_state(state.clone());

        let retrieved_state = req.state::<AppState>().unwrap();
        assert_eq!(retrieved_state.counter, 42);
    }

    #[test]
    fn test_serializable_value_bytes_to_json() {
        let bytes = vec![1, 2, 3, 4, 5];
        let value = SerializableValue::Bytes(bytes.clone());
        let json = value.to_json();

        // Should be base64 encoded
        assert!(json.is_string());
        let encoded = json.as_str().unwrap();

        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        assert_eq!(decoded, bytes);
    }
}
