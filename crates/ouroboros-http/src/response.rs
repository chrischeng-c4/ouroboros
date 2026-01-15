//! HTTP response types

use crate::error::{HttpError, HttpResult};
use std::collections::HashMap;
use std::time::Duration;
use ouroboros_common::http::HttpResponseLike;

/// HTTP response with built-in latency measurement
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code
    pub status_code: u16,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body as bytes
    pub body: Vec<u8>,

    /// Request latency in milliseconds
    pub latency_ms: u64,

    /// Final URL (may differ from request URL due to redirects)
    pub url: String,

    /// HTTP version
    pub version: String,
}

impl HttpResponse {
    /// Create a new response
    pub fn new(
        status_code: u16,
        headers: HashMap<String, String>,
        body: Vec<u8>,
        latency_ms: u64,
        url: String,
    ) -> Self {
        Self {
            status_code,
            headers,
            body,
            latency_ms,
            url,
            version: "HTTP/1.1".to_string(),
        }
    }

    /// Check if status is success (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Check if status is client error (4xx)
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    /// Check if status is server error (5xx)
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    /// Check if status is redirect (3xx)
    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status_code)
    }

    /// Get body as text (UTF-8)
    pub fn text(&self) -> HttpResult<String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| HttpError::ResponseError(format!("Invalid UTF-8 in response: {}", e)))
    }

    /// Get body as JSON
    pub fn json(&self) -> HttpResult<serde_json::Value> {
        serde_json::from_slice(&self.body)
            .map_err(|e| HttpError::Json(format!("Failed to parse JSON: {}", e)))
    }

    /// Get body as JSON and deserialize to type
    pub fn json_as<T: serde::de::DeserializeOwned>(&self) -> HttpResult<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| HttpError::Json(format!("Failed to deserialize JSON: {}", e)))
    }

    /// Get raw bytes
    pub fn bytes(&self) -> &[u8] {
        &self.body
    }

    /// Get content length
    pub fn content_length(&self) -> usize {
        self.body.len()
    }

    /// Get latency as Duration
    pub fn latency(&self) -> Duration {
        Duration::from_millis(self.latency_ms)
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        // Case-insensitive header lookup
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }

    /// Get content type
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Check if content type is JSON
    pub fn is_json(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }
}

impl HttpResponseLike for HttpResponse {
    fn status_code(&self) -> u16 {
        self.status_code
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    fn body_bytes(&self) -> &[u8] {
        &self.body
    }
}

/// Builder for creating HttpResponse (used internally)
#[derive(Debug)]
pub struct HttpResponseBuilder {
    status_code: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
    latency_ms: u64,
    url: String,
    version: String,
}

impl HttpResponseBuilder {
    pub fn new() -> Self {
        Self {
            status_code: 200,
            headers: HashMap::new(),
            body: Vec::new(),
            latency_ms: 0,
            url: String::new(),
            version: "HTTP/1.1".to_string(),
        }
    }

    pub fn status_code(mut self, code: u16) -> Self {
        self.status_code = code;
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn latency_ms(mut self, ms: u64) -> Self {
        self.latency_ms = ms;
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn build(self) -> HttpResponse {
        HttpResponse {
            status_code: self.status_code,
            headers: self.headers,
            body: self.body,
            latency_ms: self.latency_ms,
            url: self.url,
            version: self.version,
        }
    }
}

impl Default for HttpResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert reqwest Response to HttpResponse
pub async fn from_reqwest(
    response: reqwest::Response,
    latency_ms: u64,
) -> HttpResult<HttpResponse> {
    let status_code = response.status().as_u16();
    let url = response.url().to_string();
    let version = format!("{:?}", response.version());

    // Extract headers
    let mut headers = HashMap::new();
    for (name, value) in response.headers().iter() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.to_string(), v.to_string());
        }
    }

    // Read body
    let body = response.bytes().await?.to_vec();

    Ok(HttpResponse {
        status_code,
        headers,
        body,
        latency_ms,
        url,
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_status_checks() {
        let response = HttpResponseBuilder::new().status_code(200).build();
        assert!(response.is_success());
        assert!(!response.is_client_error());

        let response = HttpResponseBuilder::new().status_code(404).build();
        assert!(!response.is_success());
        assert!(response.is_client_error());

        let response = HttpResponseBuilder::new().status_code(500).build();
        assert!(response.is_server_error());
    }

    #[test]
    fn test_response_json() {
        let response = HttpResponseBuilder::new()
            .body(br#"{"name": "Alice", "age": 30}"#.to_vec())
            .build();

        let json = response.json().unwrap();
        assert_eq!(json["name"], "Alice");
        assert_eq!(json["age"], 30);
    }

    #[test]
    fn test_response_header_case_insensitive() {
        let response = HttpResponseBuilder::new()
            .header("Content-Type", "application/json")
            .build();

        assert_eq!(
            response.header("content-type"),
            Some("application/json")
        );
        assert_eq!(
            response.header("CONTENT-TYPE"),
            Some("application/json")
        );
    }

    #[test]
    fn test_response_is_json() {
        let response = HttpResponseBuilder::new()
            .header("Content-Type", "application/json; charset=utf-8")
            .build();
        assert!(response.is_json());

        let response = HttpResponseBuilder::new()
            .header("Content-Type", "text/html")
            .build();
        assert!(!response.is_json());
    }
}
