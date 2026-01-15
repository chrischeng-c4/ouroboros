//! HTTP client with connection pooling and async operations

use crate::config::HttpClientConfig;
use crate::error::HttpResult;
use crate::request::{ExtractedRequest, HttpMethod, RequestBuilder};
use crate::response::{from_reqwest, HttpResponse};
use std::sync::Arc;
use std::time::Instant;

/// High-performance async HTTP client with connection pooling
///
/// # Example
///
/// ```ignore
/// use ouroboros_http::{HttpClient, HttpClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = HttpClientConfig::new()
///         .base_url("https://api.example.com")
///         .timeout_secs(30.0);
///
///     let client = HttpClient::new(config)?;
///
///     let response = client.get("/users/1").await?;
///     println!("Status: {}, Latency: {}ms", response.status_code, response.latency_ms);
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct HttpClient {
    inner: Arc<HttpClientInner>,
}

struct HttpClientInner {
    client: reqwest::Client,
    config: HttpClientConfig,
}

impl HttpClient {
    /// Create a new HTTP client with the given configuration
    pub fn new(config: HttpClientConfig) -> HttpResult<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(config.pool_idle_timeout)
            .user_agent(&config.user_agent);

        // Configure redirects
        if config.follow_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::limited(config.max_redirects));
        } else {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }

        // Configure compression
        builder = builder.gzip(config.gzip).brotli(config.brotli);

        // Danger: Accept invalid certificates (testing only)
        if config.danger_accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(true);
        }
        if config.danger_accept_invalid_hostnames {
            builder = builder.danger_accept_invalid_hostnames(true);
        }

        let client = builder.build()?;

        Ok(Self {
            inner: Arc::new(HttpClientInner { client, config }),
        })
    }

    /// Create a client with default configuration
    pub fn default_client() -> HttpResult<Self> {
        Self::new(HttpClientConfig::default())
    }

    /// Get the base URL
    pub fn base_url(&self) -> Option<&str> {
        self.inner.config.base_url.as_deref()
    }

    /// Execute an extracted request (called from PyO3 bindings)
    pub async fn execute(&self, request: ExtractedRequest) -> HttpResult<HttpResponse> {
        let start = Instant::now();

        let reqwest_builder =
            request.build_reqwest(&self.inner.client, self.inner.config.base_url.as_deref())?;

        let response = reqwest_builder.send().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        from_reqwest(response, latency_ms).await
    }

    /// Execute a request builder
    pub async fn execute_builder(&self, builder: RequestBuilder) -> HttpResult<HttpResponse> {
        let extracted = ExtractedRequest {
            method: builder.method,
            url: builder.url,
            headers: builder.headers.into_iter().collect(),
            query_params: builder.query_params.into_iter().collect(),
            body: match builder.body {
                crate::request::RequestBody::None => crate::request::ExtractedBody::None,
                crate::request::RequestBody::Json(v) => crate::request::ExtractedBody::Json(v),
                crate::request::RequestBody::Form(m) => {
                    crate::request::ExtractedBody::Form(m.into_iter().collect())
                }
                crate::request::RequestBody::Bytes(b) => crate::request::ExtractedBody::Bytes(b),
                crate::request::RequestBody::Text(t) => crate::request::ExtractedBody::Text(t),
            },
            auth: match builder.auth {
                crate::request::Auth::None => crate::request::ExtractedAuth::None,
                crate::request::Auth::Basic { username, password } => {
                    crate::request::ExtractedAuth::Basic { username, password }
                }
                crate::request::Auth::Bearer(token) => crate::request::ExtractedAuth::Bearer(token),
            },
            timeout_ms: builder.timeout.map(|d| d.as_millis() as u64),
        };

        self.execute(extracted).await
    }

    // Convenience methods for common HTTP methods

    /// Send a GET request
    pub async fn get(&self, url: &str) -> HttpResult<HttpResponse> {
        self.execute_builder(RequestBuilder::new(HttpMethod::Get, url))
            .await
    }

    /// Send a POST request with JSON body
    pub async fn post(&self, url: &str, body: serde_json::Value) -> HttpResult<HttpResponse> {
        self.execute_builder(RequestBuilder::new(HttpMethod::Post, url).json_value(body))
            .await
    }

    /// Send a PUT request with JSON body
    pub async fn put(&self, url: &str, body: serde_json::Value) -> HttpResult<HttpResponse> {
        self.execute_builder(RequestBuilder::new(HttpMethod::Put, url).json_value(body))
            .await
    }

    /// Send a PATCH request with JSON body
    pub async fn patch(&self, url: &str, body: serde_json::Value) -> HttpResult<HttpResponse> {
        self.execute_builder(RequestBuilder::new(HttpMethod::Patch, url).json_value(body))
            .await
    }

    /// Send a DELETE request
    pub async fn delete(&self, url: &str) -> HttpResult<HttpResponse> {
        self.execute_builder(RequestBuilder::new(HttpMethod::Delete, url))
            .await
    }

    /// Send a HEAD request
    pub async fn head(&self, url: &str) -> HttpResult<HttpResponse> {
        self.execute_builder(RequestBuilder::new(HttpMethod::Head, url))
            .await
    }

    /// Send an OPTIONS request
    pub async fn options(&self, url: &str) -> HttpResult<HttpResponse> {
        self.execute_builder(RequestBuilder::new(HttpMethod::Options, url))
            .await
    }

    /// Create a request builder for more complex requests
    pub fn request(&self, method: HttpMethod, url: &str) -> RequestBuilder {
        RequestBuilder::new(method, url)
    }
}

impl std::fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("base_url", &self.inner.config.base_url)
            .field("timeout", &self.inner.config.timeout)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = HttpClientConfig::new()
            .base_url("https://api.example.com")
            .timeout_secs(30.0);

        let client = HttpClient::new(config).unwrap();
        assert_eq!(client.base_url(), Some("https://api.example.com"));
    }

    #[test]
    fn test_default_client() {
        let client = HttpClient::default_client().unwrap();
        assert!(client.base_url().is_none());
    }

    #[tokio::test]
    async fn test_request_builder() {
        let config = HttpClientConfig::new().base_url("https://httpbin.org");

        let client = HttpClient::new(config).unwrap();

        // This would make a real HTTP call, so we just test the builder
        let builder = client
            .request(HttpMethod::Post, "/post")
            .header("X-Custom", "value")
            .query("foo", "bar")
            .json_value(serde_json::json!({"key": "value"}));

        assert_eq!(builder.method, HttpMethod::Post);
        assert_eq!(builder.url, "/post");
        assert!(builder.headers.contains_key("X-Custom"));
    }
}
