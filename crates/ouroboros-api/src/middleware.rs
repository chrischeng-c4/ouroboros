//! Middleware system
//!
//! Middleware can intercept and modify requests/responses.

use crate::request::Request;
use crate::response::Response;
use crate::error::ApiResult;
use async_trait::async_trait;

/// Middleware trait
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Process a request before it reaches the handler
    async fn before_request(&self, _req: &mut Request) -> ApiResult<()> {
        Ok(())
    }

    /// Process a response before it's sent to the client
    async fn after_response(&self, _req: &Request, _res: &mut Response) -> ApiResult<()> {
        Ok(())
    }
}

/// Middleware chain
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new middleware chain
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the chain
    pub fn add(&mut self, middleware: Box<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    /// Process request through all middlewares
    pub async fn process_request(&self, req: &mut Request) -> ApiResult<()> {
        for middleware in &self.middlewares {
            middleware.before_request(req).await?;
        }
        Ok(())
    }

    /// Process response through all middlewares (in reverse order)
    pub async fn process_response(&self, req: &Request, res: &mut Response) -> ApiResult<()> {
        for middleware in self.middlewares.iter().rev() {
            middleware.after_response(req, res).await?;
        }
        Ok(())
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CORS Middleware
// ============================================================================

/// CORS (Cross-Origin Resource Sharing) configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins (use "*" for any)
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Exposed headers (readable by client)
    pub exposed_headers: Vec<String>,
    /// Allow credentials (cookies, authorization headers)
    pub allow_credentials: bool,
    /// Max age for preflight cache (seconds)
    pub max_age: Option<u32>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["*".to_string()],
            exposed_headers: Vec::new(),
            allow_credentials: false,
            max_age: Some(86400), // 24 hours
        }
    }
}

impl CorsConfig {
    /// Create a new CORS configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow any origin
    pub fn allow_any_origin(mut self) -> Self {
        self.allowed_origins = vec!["*".to_string()];
        self
    }

    /// Set allowed origins
    pub fn origins<I, S>(mut self, origins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_origins = origins.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set allowed methods
    pub fn methods<I, S>(mut self, methods: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_methods = methods.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set allowed headers
    pub fn headers<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_headers = headers.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set exposed headers
    pub fn expose<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.exposed_headers = headers.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Allow credentials
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }

    /// Set max age for preflight cache
    pub fn max_age(mut self, seconds: u32) -> Self {
        self.max_age = Some(seconds);
        self
    }

    /// Check if origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.iter().any(|o| o == "*" || o == origin)
    }
}

/// CORS middleware
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    /// Create a new CORS middleware with the given configuration
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }

    /// Create a permissive CORS middleware (allows everything)
    pub fn permissive() -> Self {
        Self::new(CorsConfig::default())
    }

    /// Create a restrictive CORS middleware
    pub fn restrictive(origins: Vec<String>) -> Self {
        Self::new(CorsConfig::new().origins(origins).allow_credentials(true))
    }
}

#[async_trait]
impl Middleware for CorsMiddleware {
    async fn after_response(&self, req: &Request, res: &mut Response) -> ApiResult<()> {
        // Get origin from request
        let origin = req.header("origin").unwrap_or("*");

        // Check if origin is allowed
        let allowed_origin = if self.config.is_origin_allowed(origin) {
            origin.to_string()
        } else if self.config.allowed_origins.contains(&"*".to_string()) {
            "*".to_string()
        } else {
            // Origin not allowed - don't add CORS headers
            return Ok(());
        };

        // Add CORS headers
        res.set_header("Access-Control-Allow-Origin", &allowed_origin);

        if self.config.allow_credentials {
            res.set_header("Access-Control-Allow-Credentials", "true");
        }

        // For preflight requests (OPTIONS), add additional headers
        if req.method_str() == "OPTIONS" {
            res.set_header(
                "Access-Control-Allow-Methods",
                &self.config.allowed_methods.join(", "),
            );

            let headers = if self.config.allowed_headers.contains(&"*".to_string()) {
                // Echo back the requested headers
                req.header("access-control-request-headers")
                    .unwrap_or("*")
                    .to_string()
            } else {
                self.config.allowed_headers.join(", ")
            };
            res.set_header("Access-Control-Allow-Headers", &headers);

            if let Some(max_age) = self.config.max_age {
                res.set_header("Access-Control-Max-Age", &max_age.to_string());
            }
        }

        // Add exposed headers
        if !self.config.exposed_headers.is_empty() {
            res.set_header(
                "Access-Control-Expose-Headers",
                &self.config.exposed_headers.join(", "),
            );
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert!(config.allowed_origins.contains(&"*".to_string()));
        assert!(config.allowed_methods.contains(&"GET".to_string()));
        assert!(!config.allow_credentials);
    }

    #[test]
    fn test_cors_config_builder() {
        let config = CorsConfig::new()
            .origins(["https://example.com"])
            .methods(["GET", "POST"])
            .headers(["Content-Type", "Authorization"])
            .allow_credentials(true)
            .max_age(3600);

        assert!(config.allowed_origins.contains(&"https://example.com".to_string()));
        assert_eq!(config.allowed_methods.len(), 2);
        assert_eq!(config.allowed_headers.len(), 2);
        assert!(config.allow_credentials);
        assert_eq!(config.max_age, Some(3600));
    }

    #[test]
    fn test_is_origin_allowed() {
        let config = CorsConfig::new().origins(["https://example.com", "https://api.example.com"]);

        assert!(config.is_origin_allowed("https://example.com"));
        assert!(config.is_origin_allowed("https://api.example.com"));
        assert!(!config.is_origin_allowed("https://other.com"));
    }

    #[test]
    fn test_is_origin_allowed_wildcard() {
        let config = CorsConfig::new().allow_any_origin();

        assert!(config.is_origin_allowed("https://anything.com"));
        assert!(config.is_origin_allowed("http://localhost:3000"));
    }
}
