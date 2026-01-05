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

/// File uploaded in multipart form
///
/// All fields are `Send + Sync` for GIL-free processing.
#[derive(Debug, Clone)]
pub struct SerializableFile {
    /// Field name in the form
    pub field_name: String,
    /// Original filename (if provided)
    pub filename: String,
    /// Content-Type (MIME type)
    pub content_type: String,
    /// File data (binary)
    pub data: Vec<u8>,
}

/// Form data (multipart or url-encoded)
///
/// All fields are `Send + Sync` for GIL-free processing.
#[derive(Debug, Clone)]
pub struct SerializableFormData {
    /// Simple text fields
    pub fields: HashMap<String, String>,
    /// File uploads
    pub files: Vec<SerializableFile>,
}

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
    /// Form data (multipart or url-encoded)
    pub form_data: Option<SerializableFormData>,
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
            form_data: None,
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

    /// Get form data
    pub fn form_data(&self) -> Option<&SerializableFormData> {
        self.inner.form_data.as_ref()
    }
}

impl From<SerializableRequest> for Request {
    fn from(inner: SerializableRequest) -> Self {
        Self::new(inner)
    }
}

// ============================================================================
// Form Data Parsing
// ============================================================================

/// Parse multipart form data
///
/// This function processes multipart/form-data requests, extracting both
/// text fields and file uploads. Designed for GIL-free processing.
///
/// # Arguments
/// * `boundary` - Multipart boundary string (from Content-Type header)
/// * `body_bytes` - Raw request body bytes
///
/// # Returns
/// `SerializableFormData` with text fields and files
///
/// # Example
/// ```rust,no_run
/// use data_bridge_api::request::parse_multipart;
///
/// # async fn example() -> Result<(), String> {
/// let boundary = "----WebKitFormBoundary".to_string();
/// let body = b"------WebKitFormBoundary\r\n...".to_vec();
/// let form_data = parse_multipart(boundary, body).await?;
/// # Ok(())
/// # }
/// ```
pub async fn parse_multipart(
    boundary: String,
    body_bytes: Vec<u8>,
) -> Result<SerializableFormData, String> {
    let stream = futures_util::stream::once(async move {
        Ok::<_, std::io::Error>(body_bytes)
    });

    let mut multipart = multer::Multipart::new(stream, boundary);

    let mut fields = HashMap::new();
    let mut files = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let name = field.name()
            .ok_or("Missing field name")?
            .to_string();

        if let Some(filename) = field.file_name() {
            // File field - clone filename before consuming field
            let filename = filename.to_string();
            let content_type = field.content_type()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let data = field.bytes().await.map_err(|e| e.to_string())?;

            files.push(SerializableFile {
                field_name: name,
                filename,
                content_type,
                data: data.to_vec(),
            });
        } else {
            // Text field
            let value = field.text().await.map_err(|e| e.to_string())?;
            fields.insert(name, value);
        }
    }

    Ok(SerializableFormData { fields, files })
}

/// Parse application/x-www-form-urlencoded data
///
/// This function decodes URL-encoded form data into key-value pairs.
/// Designed for GIL-free processing.
///
/// # Arguments
/// * `body` - Raw request body bytes
///
/// # Returns
/// HashMap with decoded field names and values
///
/// # Example
/// ```rust
/// use data_bridge_api::request::parse_urlencoded;
///
/// # fn example() -> Result<(), String> {
/// let body = b"name=Alice&age=30&city=New%20York";
/// let fields = parse_urlencoded(body)?;
/// assert_eq!(fields.get("name"), Some(&"Alice".to_string()));
/// assert_eq!(fields.get("city"), Some(&"New York".to_string()));
/// # Ok(())
/// # }
/// ```
pub fn parse_urlencoded(body: &[u8]) -> Result<HashMap<String, String>, String> {
    let text = std::str::from_utf8(body).map_err(|e| e.to_string())?;
    let mut fields = HashMap::new();

    for pair in text.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = urlencoding::decode(key).map_err(|e| e.to_string())?.to_string();
            let value = urlencoding::decode(value).map_err(|e| e.to_string())?.to_string();
            fields.insert(key, value);
        }
    }

    Ok(fields)
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

    #[test]
    fn test_parse_urlencoded_basic() {
        let body = b"name=Alice&age=30&city=New%20York";
        let fields = parse_urlencoded(body).unwrap();

        assert_eq!(fields.get("name"), Some(&"Alice".to_string()));
        assert_eq!(fields.get("age"), Some(&"30".to_string()));
        assert_eq!(fields.get("city"), Some(&"New York".to_string()));
    }

    #[test]
    fn test_parse_urlencoded_empty() {
        let body = b"";
        let fields = parse_urlencoded(body).unwrap();
        assert!(fields.is_empty());
    }

    #[test]
    fn test_parse_urlencoded_special_chars() {
        let body = b"email=test%40example.com&message=Hello%21%20World";
        let fields = parse_urlencoded(body).unwrap();

        assert_eq!(fields.get("email"), Some(&"test@example.com".to_string()));
        assert_eq!(fields.get("message"), Some(&"Hello! World".to_string()));
    }

    #[test]
    fn test_parse_urlencoded_invalid_utf8() {
        let body = &[0xFF, 0xFE, 0xFD];
        let result = parse_urlencoded(body);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_multipart_text_fields() {
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let body = b"\
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"name\"\r
\r
Alice\r
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"age\"\r
\r
30\r
------WebKitFormBoundary7MA4YWxkTrZu0gW--\r
";

        let form_data = parse_multipart(boundary.to_string(), body.to_vec())
            .await
            .unwrap();

        assert_eq!(form_data.fields.get("name"), Some(&"Alice".to_string()));
        assert_eq!(form_data.fields.get("age"), Some(&"30".to_string()));
        assert!(form_data.files.is_empty());
    }

    #[tokio::test]
    async fn test_parse_multipart_file_upload() {
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let body = b"\
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"document\"; filename=\"test.txt\"\r
Content-Type: text/plain\r
\r
Hello, World!\r
------WebKitFormBoundary7MA4YWxkTrZu0gW--\r
";

        let form_data = parse_multipart(boundary.to_string(), body.to_vec())
            .await
            .unwrap();

        assert_eq!(form_data.files.len(), 1);
        assert!(form_data.fields.is_empty());

        let file = &form_data.files[0];
        assert_eq!(file.field_name, "document");
        assert_eq!(file.filename, "test.txt");
        assert_eq!(file.content_type, "text/plain");
        assert_eq!(file.data, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_parse_multipart_mixed_fields_and_files() {
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let body = b"\
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"title\"\r
\r
My Document\r
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"file\"; filename=\"data.bin\"\r
Content-Type: application/octet-stream\r
\r
\x00\x01\x02\x03\r
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"description\"\r
\r
Important file\r
------WebKitFormBoundary7MA4YWxkTrZu0gW--\r
";

        let form_data = parse_multipart(boundary.to_string(), body.to_vec())
            .await
            .unwrap();

        // Check text fields
        assert_eq!(form_data.fields.get("title"), Some(&"My Document".to_string()));
        assert_eq!(form_data.fields.get("description"), Some(&"Important file".to_string()));

        // Check file
        assert_eq!(form_data.files.len(), 1);
        let file = &form_data.files[0];
        assert_eq!(file.field_name, "file");
        assert_eq!(file.filename, "data.bin");
        assert_eq!(file.content_type, "application/octet-stream");
        assert_eq!(file.data, vec![0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn test_parse_multipart_multiple_files() {
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let body = b"\
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"file1\"; filename=\"a.txt\"\r
Content-Type: text/plain\r
\r
Content A\r
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"file2\"; filename=\"b.txt\"\r
Content-Type: text/plain\r
\r
Content B\r
------WebKitFormBoundary7MA4YWxkTrZu0gW--\r
";

        let form_data = parse_multipart(boundary.to_string(), body.to_vec())
            .await
            .unwrap();

        assert_eq!(form_data.files.len(), 2);
        assert!(form_data.fields.is_empty());

        // Files should be in order
        assert_eq!(form_data.files[0].filename, "a.txt");
        assert_eq!(form_data.files[0].data, b"Content A");
        assert_eq!(form_data.files[1].filename, "b.txt");
        assert_eq!(form_data.files[1].data, b"Content B");
    }

    #[tokio::test]
    async fn test_parse_multipart_file_without_content_type() {
        let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let body = b"\
------WebKitFormBoundary7MA4YWxkTrZu0gW\r
Content-Disposition: form-data; name=\"upload\"; filename=\"file.dat\"\r
\r
Binary data\r
------WebKitFormBoundary7MA4YWxkTrZu0gW--\r
";

        let form_data = parse_multipart(boundary.to_string(), body.to_vec())
            .await
            .unwrap();

        assert_eq!(form_data.files.len(), 1);
        let file = &form_data.files[0];
        // Should default to application/octet-stream
        assert_eq!(file.content_type, "application/octet-stream");
    }

    #[test]
    fn test_serializable_file_is_send_sync() {
        // Compile-time check that SerializableFile is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SerializableFile>();
    }

    #[test]
    fn test_serializable_form_data_is_send_sync() {
        // Compile-time check that SerializableFormData is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SerializableFormData>();
    }

    #[test]
    fn test_request_with_form_data() {
        let form_data = SerializableFormData {
            fields: {
                let mut map = HashMap::new();
                map.insert("name".to_string(), "Alice".to_string());
                map
            },
            files: vec![],
        };

        let mut req = SerializableRequest::new(HttpMethod::Post, "/api/upload");
        req.form_data = Some(form_data);

        let request = Request::new(req);
        let retrieved_form = request.form_data().unwrap();
        assert_eq!(retrieved_form.fields.get("name"), Some(&"Alice".to_string()));
    }
}
