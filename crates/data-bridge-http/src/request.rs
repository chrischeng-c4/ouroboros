//! HTTP request types and builders

use crate::error::{HttpError, HttpResult};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

// Re-export HttpMethod from data-bridge-common
pub use data_bridge_common::http::HttpMethod;

/// Convert HttpMethod to reqwest Method
fn to_reqwest_method(method: HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    }
}

/// Request body types
#[derive(Debug, Clone)]
pub enum RequestBody {
    /// No body
    None,
    /// JSON body
    Json(serde_json::Value),
    /// Form data (application/x-www-form-urlencoded)
    Form(HashMap<String, String>),
    /// Raw bytes
    Bytes(Vec<u8>),
    /// Raw text
    Text(String),
}

/// Authentication types
#[derive(Debug, Clone)]
pub enum Auth {
    /// No authentication
    None,
    /// Basic authentication (username, password)
    Basic { username: String, password: String },
    /// Bearer token
    Bearer(String),
}

/// Request builder for constructing HTTP requests
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    /// HTTP method
    pub method: HttpMethod,
    /// Request URL (relative path if base_url is set)
    pub url: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request body
    pub body: RequestBody,
    /// Authentication
    pub auth: Auth,
    /// Request timeout (overrides client timeout)
    pub timeout: Option<Duration>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: RequestBody::None,
            auth: Auth::None,
            timeout: None,
        }
    }

    /// Add a header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Add multiple headers
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Add a query parameter
    pub fn query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(name.into(), value.into());
        self
    }

    /// Add multiple query parameters
    pub fn query_params(mut self, params: HashMap<String, String>) -> Self {
        self.query_params.extend(params);
        self
    }

    /// Set JSON body
    pub fn json<T: Serialize>(mut self, body: &T) -> HttpResult<Self> {
        let json = serde_json::to_value(body)
            .map_err(|e| HttpError::Json(format!("Failed to serialize JSON: {}", e)))?;
        self.body = RequestBody::Json(json);
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Set JSON body from Value
    pub fn json_value(mut self, body: serde_json::Value) -> Self {
        self.body = RequestBody::Json(body);
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self
    }

    /// Set form body
    pub fn form(mut self, data: HashMap<String, String>) -> Self {
        self.body = RequestBody::Form(data);
        self.headers.insert(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        self
    }

    /// Set raw bytes body
    pub fn bytes(mut self, data: Vec<u8>) -> Self {
        self.body = RequestBody::Bytes(data);
        self
    }

    /// Set raw text body
    pub fn text(mut self, data: impl Into<String>) -> Self {
        self.body = RequestBody::Text(data.into());
        self.headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
        self
    }

    /// Set basic authentication
    pub fn basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Auth::Basic {
            username: username.into(),
            password: password.into(),
        };
        self
    }

    /// Set bearer token authentication
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Auth::Bearer(token.into());
        self
    }

    /// Set request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set timeout from seconds
    pub fn timeout_secs(mut self, secs: f64) -> Self {
        self.timeout = Some(Duration::from_secs_f64(secs));
        self
    }
}

/// Extracted request data (for GIL-free processing)
/// This is an intermediate representation extracted from Python with GIL,
/// then processed in Rust without GIL.
#[derive(Debug, Clone)]
pub struct ExtractedRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub query_params: Vec<(String, String)>,
    pub body: ExtractedBody,
    pub auth: ExtractedAuth,
    pub timeout_ms: Option<u64>,
}

/// Extracted request body
#[derive(Debug, Clone)]
pub enum ExtractedBody {
    None,
    Json(serde_json::Value),
    Form(Vec<(String, String)>),
    Bytes(Vec<u8>),
    Text(String),
}

/// Extracted authentication
#[derive(Debug, Clone)]
pub enum ExtractedAuth {
    None,
    Basic { username: String, password: String },
    Bearer(String),
}

impl ExtractedRequest {
    /// Build a reqwest RequestBuilder from extracted data
    pub fn build_reqwest(
        &self,
        client: &reqwest::Client,
        base_url: Option<&str>,
    ) -> HttpResult<reqwest::RequestBuilder> {
        // Construct full URL
        let full_url = if let Some(base) = base_url {
            if self.url.starts_with("http://") || self.url.starts_with("https://") {
                self.url.clone()
            } else {
                let base = base.trim_end_matches('/');
                let path = if self.url.starts_with('/') {
                    &self.url[..]
                } else {
                    &format!("/{}", self.url)
                };
                format!("{}{}", base, path)
            }
        } else {
            self.url.clone()
        };

        let mut builder = client.request(to_reqwest_method(self.method), &full_url);

        // Add headers
        for (name, value) in &self.headers {
            builder = builder.header(name, value);
        }

        // Add query params
        if !self.query_params.is_empty() {
            builder = builder.query(&self.query_params);
        }

        // Add body
        builder = match &self.body {
            ExtractedBody::None => builder,
            ExtractedBody::Json(value) => builder.json(value),
            ExtractedBody::Form(data) => {
                let form_data: Vec<(&str, &str)> = data
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                builder.form(&form_data)
            }
            ExtractedBody::Bytes(data) => builder.body(data.clone()),
            ExtractedBody::Text(data) => builder.body(data.clone()),
        };

        // Add auth
        builder = match &self.auth {
            ExtractedAuth::None => builder,
            ExtractedAuth::Basic { username, password } => {
                builder.basic_auth(username, Some(password))
            }
            ExtractedAuth::Bearer(token) => builder.bearer_auth(token),
        };

        // Add timeout
        if let Some(timeout_ms) = self.timeout_ms {
            builder = builder.timeout(Duration::from_millis(timeout_ms));
        }

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::parse_method("GET").unwrap(), HttpMethod::Get);
        assert_eq!(HttpMethod::parse_method("post").unwrap(), HttpMethod::Post);
        assert!(HttpMethod::parse_method("INVALID").is_err());
    }

    #[test]
    fn test_request_builder() {
        let request = RequestBuilder::new(HttpMethod::Post, "/api/users")
            .header("X-Custom", "value")
            .query("page", "1")
            .json(&serde_json::json!({"name": "Alice"}))
            .unwrap()
            .bearer_auth("token123");

        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, "/api/users");
        assert!(request.headers.contains_key("X-Custom"));
        assert!(request.headers.contains_key("Content-Type"));
        assert!(matches!(request.auth, Auth::Bearer(_)));
    }
}
