//! HTTP server implementation using Hyper 1.0
//!
//! This module provides a production-ready HTTP server that wraps the Router
//! with the following features:
//! - Async request processing via Tokio runtime
//! - GIL-free request/response handling (following two-phase pattern)
//! - Integration with existing validation layer
//! - Graceful shutdown via signals
//! - Request logging and error handling

use crate::error::{ApiError, ApiResult};
use crate::request::{HttpMethod, Request, SerializableRequest, SerializableValue};
use crate::response::{Response, ResponseBody};
use crate::router::Router;
use http::header::CONTENT_LENGTH;
use http::{HeaderMap, StatusCode};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info, warn};

/// HTTP server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind address (e.g., "127.0.0.1:8000")
    pub bind_addr: String,
    /// Maximum request body size in bytes (default: 10MB)
    pub max_body_size: usize,
    /// Enable request logging
    pub enable_logging: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8000".to_string(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            enable_logging: true,
        }
    }
}

impl ServerConfig {
    /// Create a new server config with bind address
    pub fn new(bind_addr: impl Into<String>) -> Self {
        Self {
            bind_addr: bind_addr.into(),
            ..Default::default()
        }
    }

    /// Set maximum request body size
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }

    /// Enable or disable request logging
    pub fn logging(mut self, enabled: bool) -> Self {
        self.enable_logging = enabled;
        self
    }
}

/// HTTP server wrapping the Router
pub struct Server {
    router: Arc<Router>,
    config: ServerConfig,
}

impl Server {
    /// Create a new server with the given router
    pub fn new(router: Router, config: ServerConfig) -> Self {
        Self {
            router: Arc::new(router),
            config,
        }
    }

    /// Create a new server with an Arc<Router>
    pub fn with_shared_router(router: Arc<Router>, config: ServerConfig) -> Self {
        Self {
            router,
            config,
        }
    }

    /// Create a new server with default configuration
    pub fn with_router(router: Router) -> Self {
        Self::new(router, ServerConfig::default())
    }

    /// Run the server until a shutdown signal is received
    ///
    /// This function binds to the configured address and starts accepting
    /// connections. It will run until Ctrl+C (SIGINT) or SIGTERM is received.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use data_bridge_api::{Router, server::{Server, ServerConfig}};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let router = Router::new();
    ///     let config = ServerConfig::new("127.0.0.1:8000");
    ///     let server = Server::new(router, config);
    ///     server.run().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr: SocketAddr = self.config.bind_addr.parse()?;
        let listener = TcpListener::bind(addr).await?;

        info!("Server listening on http://{}", addr);
        info!("Max body size: {} bytes", self.config.max_body_size);
        info!("Press Ctrl+C to shutdown");

        let router = self.router.clone();
        let config = self.config.clone();

        // Graceful shutdown signal handler
        let shutdown_signal = shutdown_signal();

        // Accept connections until shutdown
        tokio::select! {
            result = async {
                loop {
                    let (stream, remote_addr) = listener.accept().await?;
                    let io = TokioIo::new(stream);
                    let router = router.clone();
                    let config = config.clone();

                    // Spawn a task to handle this connection
                    tokio::spawn(async move {
                        let service = service_fn(move |req| {
                            handle_request(req, router.clone(), config.clone(), remote_addr)
                        });

                        if let Err(err) = http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            error!("Error serving connection: {:?}", err);
                        }
                    });
                }
            } => {
                result
            }
            _ = shutdown_signal => {
                info!("Shutdown signal received, stopping server");
                Ok(())
            }
        }
    }

    /// Get the configured bind address
    pub fn bind_addr(&self) -> &str {
        &self.config.bind_addr
    }

    /// Get the router
    pub fn router(&self) -> &Router {
        &self.router
    }
}

/// Handle a single HTTP request
///
/// This function implements the two-phase GIL pattern:
/// 1. Extract request data (fast, would hold GIL in PyO3 context)
/// 2. Process request (slow, GIL-free)
async fn handle_request(
    hyper_req: HyperRequest<Incoming>,
    router: Arc<Router>,
    config: ServerConfig,
    remote_addr: SocketAddr,
) -> Result<HyperResponse<http_body_util::Full<Bytes>>, Infallible> {
    // Phase 1: Extract request data (fast)
    let (parts, body) = hyper_req.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri.clone();
    let path = uri.path().to_string();

    if config.enable_logging {
        info!("{} {} - from {}", method, path, remote_addr);
    }

    // Parse HTTP method
    let http_method = match HttpMethod::from_str(method.as_str()) {
        Ok(m) => m,
        Err(_) => {
            warn!("Invalid HTTP method: {}", method);
            return Ok(error_response(
                ApiError::MethodNotAllowed(format!("Invalid method: {}", method)),
            ));
        }
    };

    // Collect body bytes with size limit
    let body_result = collect_body(body, config.max_body_size).await;
    let body_bytes = match body_result {
        Ok(bytes) => bytes,
        Err(err) => {
            error!("Failed to read request body: {}", err);
            return Ok(error_response(ApiError::BadRequest(err.to_string())));
        }
    };

    // Convert Hyper request to SerializableRequest
    let serializable_req = match convert_hyper_request(
        http_method,
        path.clone(),
        uri.to_string(),
        parts.headers,
        body_bytes,
    )
    .await
    {
        Ok(req) => req,
        Err(err) => {
            error!("Failed to convert request: {}", err);
            return Ok(error_response(err));
        }
    };

    // Phase 2: Process request (GIL-free in PyO3 context)
    let response = process_request(serializable_req, router).await;

    // Convert response to Hyper format
    Ok(convert_response_to_hyper(response))
}

/// Convert Hyper request to SerializableRequest
///
/// Extracts all request data into a GIL-free representation
async fn convert_hyper_request(
    method: HttpMethod,
    path: String,
    url: String,
    headers: HeaderMap,
    body_bytes: Vec<u8>,
) -> ApiResult<SerializableRequest> {
    let mut req = SerializableRequest::new(method, path).with_url(url);

    // Extract headers
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            req = req.with_header(name.as_str(), value_str);
        }
    }

    // Parse query parameters from URL
    // Note: In a full implementation, we'd parse the query string here
    // For now, this is a simplified version

    // Parse body based on content type
    if !body_bytes.is_empty() {
        let content_type = req.content_type.as_deref().unwrap_or("");

        if content_type.contains("application/json") {
            // Parse JSON body
            match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                Ok(json) => {
                    req = req.with_body(SerializableValue::from_json(&json));
                }
                Err(err) => {
                    return Err(ApiError::BadRequest(format!(
                        "Invalid JSON body: {}",
                        err
                    )));
                }
            }
        } else if content_type.contains("application/x-www-form-urlencoded") {
            // Parse form data
            let fields = crate::request::parse_urlencoded(&body_bytes)
                .map_err(|e| ApiError::BadRequest(format!("Invalid form data: {}", e)))?;

            let form_data = crate::request::SerializableFormData {
                fields,
                files: Vec::new(),
            };
            req.form_data = Some(form_data);
        } else if content_type.contains("multipart/form-data") {
            // Extract boundary from Content-Type header
            let boundary = extract_boundary(content_type)
                .ok_or_else(|| ApiError::BadRequest("Missing multipart boundary".to_string()))?;

            // Parse multipart form data
            let form_data = crate::request::parse_multipart(boundary, body_bytes)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Invalid multipart data: {}", e)))?;

            req.form_data = Some(form_data);
        } else {
            // Store as raw bytes
            req = req.with_body(SerializableValue::Bytes(body_bytes));
        }
    }

    Ok(req)
}

/// Extract boundary from multipart Content-Type header
fn extract_boundary(content_type: &str) -> Option<String> {
    content_type
        .split(';')
        .find_map(|part| {
            let part = part.trim();
            if part.starts_with("boundary=") {
                Some(part.trim_start_matches("boundary=").trim().to_string())
            } else {
                None
            }
        })
}

/// Collect request body with size limit
async fn collect_body(
    body: Incoming,
    max_size: usize,
) -> Result<Vec<u8>, String> {
    use http_body_util::BodyExt;

    let collected = body.collect().await.map_err(|e| e.to_string())?;
    let bytes = collected.to_bytes();

    if bytes.len() > max_size {
        return Err(format!(
            "Request body too large: {} bytes (max: {})",
            bytes.len(),
            max_size
        ));
    }

    Ok(bytes.to_vec())
}

/// Process the request through the router
///
/// This is the core request handling logic:
/// 1. Match route
/// 2. Validate request
/// 3. Execute handler
/// 4. Return response
async fn process_request(mut serializable_req: SerializableRequest, router: Arc<Router>) -> Response {
    // Match route
    let route_match = match router.match_route(serializable_req.method, &serializable_req.path) {
        Some(m) => m,
        None => {
            return Response::not_found(format!(
                "Route not found: {} {}",
                serializable_req.method.as_str(),
                serializable_req.path
            ));
        }
    };

    // Add path parameters to request
    serializable_req.path_params = route_match.params;

    // Validate request (before creating Request wrapper)
    let validated = match route_match.route.validator.validate(
        &serializable_req.path_params,
        &serializable_req.query_params,
        &serializable_req.headers,
        serializable_req.body.as_ref(),
    ) {
        Ok(v) => v,
        Err(err) => {
            return Response::error(&ApiError::Validation(err));
        }
    };

    // Create Request wrapper
    let request = Request::new(serializable_req);

    // Execute handler
    match (route_match.route.handler)(request, validated).await {
        Ok(response) => response,
        Err(err) => {
            error!("Handler error: {}", err);
            Response::error(&err)
        }
    }
}

/// Convert Response to Hyper response
fn convert_response_to_hyper(
    response: Response,
) -> HyperResponse<http_body_util::Full<Bytes>> {
    let serializable = response.into_serializable();
    let status = StatusCode::from_u16(serializable.status_code)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = serializable.body_bytes();
    let body = http_body_util::Full::new(Bytes::from(body_bytes));

    let mut hyper_response = HyperResponse::builder().status(status);

    // Add headers
    for (name, value) in serializable.headers.iter() {
        hyper_response = hyper_response.header(name, value);
    }

    // Add Content-Length if not already present
    if !serializable.headers.contains_key("content-length") {
        let content_length = match &serializable.body {
            ResponseBody::Empty => 0,
            ResponseBody::Json(_) | ResponseBody::Text(_) | ResponseBody::Bytes(_) => {
                serializable.content_length()
            }
        };
        hyper_response = hyper_response.header(CONTENT_LENGTH, content_length);
    }

    hyper_response.body(body).unwrap_or_else(|err| {
        error!("Failed to build response: {}", err);
        HyperResponse::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(http_body_util::Full::new(Bytes::from("Internal Server Error")))
            .unwrap()
    })
}

/// Create an error response
fn error_response(error: ApiError) -> HyperResponse<http_body_util::Full<Bytes>> {
    let response = Response::error(&error);
    convert_response_to_hyper(response)
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::HandlerMeta;
    use crate::validation::RequestValidator;
    use http::header::CONTENT_TYPE;

    fn dummy_handler() -> crate::router::HandlerFn {
        Arc::new(|_req, _validated| {
            Box::pin(async move { Ok(Response::ok()) })
        })
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr, "127.0.0.1:8000");
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
        assert!(config.enable_logging);
    }

    #[test]
    fn test_server_config_builder() {
        let config = ServerConfig::new("0.0.0.0:3000")
            .max_body_size(5 * 1024 * 1024)
            .logging(false);

        assert_eq!(config.bind_addr, "0.0.0.0:3000");
        assert_eq!(config.max_body_size, 5 * 1024 * 1024);
        assert!(!config.enable_logging);
    }

    #[test]
    fn test_server_creation() {
        let router = Router::new();
        let config = ServerConfig::new("127.0.0.1:8080");
        let server = Server::new(router, config);

        assert_eq!(server.bind_addr(), "127.0.0.1:8080");
    }

    #[test]
    fn test_server_with_router() {
        let router = Router::new();
        let server = Server::with_router(router);

        assert_eq!(server.bind_addr(), "127.0.0.1:8000");
    }

    #[tokio::test]
    async fn test_convert_hyper_request_json() {
        let json_body = r#"{"name":"Alice","age":30}"#;
        let headers = {
            let mut h = HeaderMap::new();
            h.insert(CONTENT_TYPE, "application/json".parse().unwrap());
            h
        };

        let req = convert_hyper_request(
            HttpMethod::Post,
            "/api/users".to_string(),
            "http://localhost/api/users".to_string(),
            headers,
            json_body.as_bytes().to_vec(),
        )
        .await
        .unwrap();

        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/users");
        assert!(req.is_json());

        let body_json = req.body_json().unwrap();
        assert_eq!(body_json["name"], "Alice");
        assert_eq!(body_json["age"], 30);
    }

    #[tokio::test]
    async fn test_convert_hyper_request_urlencoded() {
        let form_body = b"name=Alice&age=30&city=New%20York";
        let headers = {
            let mut h = HeaderMap::new();
            h.insert(
                CONTENT_TYPE,
                "application/x-www-form-urlencoded".parse().unwrap(),
            );
            h
        };

        let req = convert_hyper_request(
            HttpMethod::Post,
            "/api/users".to_string(),
            "http://localhost/api/users".to_string(),
            headers,
            form_body.to_vec(),
        )
        .await
        .unwrap();

        let form_data = req.form_data.unwrap();
        assert_eq!(form_data.fields.get("name"), Some(&"Alice".to_string()));
        assert_eq!(form_data.fields.get("age"), Some(&"30".to_string()));
        assert_eq!(
            form_data.fields.get("city"),
            Some(&"New York".to_string())
        );
    }

    #[test]
    fn test_extract_boundary() {
        let content_type = "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW";
        let boundary = extract_boundary(content_type).unwrap();
        assert_eq!(boundary, "----WebKitFormBoundary7MA4YWxkTrZu0gW");
    }

    #[test]
    fn test_extract_boundary_missing() {
        let content_type = "multipart/form-data";
        assert!(extract_boundary(content_type).is_none());
    }

    #[tokio::test]
    async fn test_process_request_not_found() {
        let router = Arc::new(Router::new());
        let req = SerializableRequest::new(HttpMethod::Get, "/api/notfound");

        let response = process_request(req, router).await;
        let serializable = response.into_serializable();

        assert_eq!(serializable.status_code, 404);
    }

    #[tokio::test]
    async fn test_process_request_success() {
        let mut router = Router::new();
        router
            .get(
                "/api/test",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("test".to_string()),
            )
            .unwrap();

        let router = Arc::new(router);
        let req = SerializableRequest::new(HttpMethod::Get, "/api/test");

        let response = process_request(req, router).await;
        let serializable = response.into_serializable();

        assert_eq!(serializable.status_code, 200);
    }

    #[test]
    fn test_convert_response_to_hyper_json() {
        let response = Response::json(SerializableValue::Object(vec![(
            "status".to_string(),
            SerializableValue::String("ok".to_string()),
        )]));

        let hyper_response = convert_response_to_hyper(response);

        assert_eq!(hyper_response.status(), StatusCode::OK);
        assert_eq!(
            hyper_response.headers().get(CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_convert_response_to_hyper_text() {
        let response = Response::text("Hello, World!");

        let hyper_response = convert_response_to_hyper(response);

        assert_eq!(hyper_response.status(), StatusCode::OK);
        assert_eq!(
            hyper_response.headers().get(CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8"
        );
    }

    #[test]
    fn test_error_response() {
        let error = ApiError::NotFound("Resource not found".to_string());
        let hyper_response = error_response(error);

        assert_eq!(hyper_response.status(), StatusCode::NOT_FOUND);
    }
}
