//! Router for HTTP request routing
//!
//! Uses a radix tree (matchit) for efficient route matching with path parameters.

use crate::error::{ApiError, ApiResult};
use crate::handler::HandlerMeta;
use crate::request::{HttpMethod, Request};
use crate::response::Response;
use crate::validation::{RequestValidator, ValidatedRequest};
use std::collections::HashMap;
use std::sync::Arc;

/// Route handler function type
pub type HandlerFn = Arc<
    dyn Fn(Request, ValidatedRequest) -> BoxFuture<'static, ApiResult<Response>> + Send + Sync,
>;

/// Boxed future for async handlers
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Registered route with metadata
pub struct Route {
    /// Handler function
    pub handler: HandlerFn,
    /// Request validator (pre-compiled)
    pub validator: RequestValidator,
    /// Handler metadata for OpenAPI
    pub metadata: HandlerMeta,
}

/// Route match result
pub struct RouteMatch<'a> {
    /// Matched route
    pub route: &'a Route,
    /// Extracted path parameters
    pub params: HashMap<String, String>,
}

/// HTTP Router using radix tree (matchit)
pub struct Router {
    /// Route trees per HTTP method
    trees: HashMap<HttpMethod, matchit::Router<String>>,
    /// Route storage by ID
    routes: HashMap<String, Route>,
    /// Route counter for unique IDs
    route_counter: usize,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            trees: HashMap::new(),
            routes: HashMap::new(),
            route_counter: 0,
        }
    }

    /// Register a route
    pub fn route(
        &mut self,
        method: HttpMethod,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        let route_id = format!("route_{}", self.route_counter);
        self.route_counter += 1;

        // Store route
        self.routes.insert(
            route_id.clone(),
            Route {
                handler,
                validator,
                metadata,
            },
        );

        // Add to method tree
        let tree = self.trees.entry(method).or_default();
        tree.insert(path, route_id).map_err(|e| {
            ApiError::Internal(format!("Route registration failed: {}", e))
        })?;

        Ok(())
    }

    /// Match a request to a route
    pub fn match_route(&self, method: HttpMethod, path: &str) -> Option<RouteMatch<'_>> {
        let tree = self.trees.get(&method)?;
        let matched = tree.at(path).ok()?;
        let route_id = matched.value;
        let route = self.routes.get(route_id)?;

        let params: HashMap<String, String> = matched
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Some(RouteMatch { route, params })
    }

    /// Get all routes (for OpenAPI generation)
    pub fn routes(&self) -> impl Iterator<Item = (HttpMethod, &str, &Route)> {
        // This requires storing paths with routes
        // For now, return empty - will implement properly
        std::iter::empty()
    }

    // Convenience methods

    /// Register a GET route
    pub fn get(
        &mut self,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.route(HttpMethod::Get, path, handler, validator, metadata)
    }

    /// Register a POST route
    pub fn post(
        &mut self,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.route(HttpMethod::Post, path, handler, validator, metadata)
    }

    /// Register a PUT route
    pub fn put(
        &mut self,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.route(HttpMethod::Put, path, handler, validator, metadata)
    }

    /// Register a PATCH route
    pub fn patch(
        &mut self,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.route(HttpMethod::Patch, path, handler, validator, metadata)
    }

    /// Register a DELETE route
    pub fn delete(
        &mut self,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.route(HttpMethod::Delete, path, handler, validator, metadata)
    }

    /// Register a WebSocket route
    ///
    /// WebSocket routes are GET requests that include the WebSocket upgrade headers.
    /// The actual upgrade happens in the server layer, but we register the route
    /// here so the path can be matched and validated.
    ///
    /// # Arguments
    /// * `path` - The WebSocket endpoint path (e.g., "/ws")
    /// * `handler` - Handler function that will process WebSocket messages
    /// * `validator` - Request validator for the initial HTTP upgrade request
    /// * `metadata` - Handler metadata for documentation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use data_bridge_api::{Router, Request, Response};
    /// use data_bridge_api::validation::{RequestValidator, ValidatedRequest};
    /// use data_bridge_api::handler::HandlerMeta;
    /// use data_bridge_api::error::ApiResult;
    /// use std::sync::Arc;
    ///
    /// let mut router = Router::new();
    /// router.websocket_route(
    ///     "/ws",
    ///     Arc::new(|req: Request, validated: ValidatedRequest| {
    ///         Box::pin(async move {
    ///             // WebSocket handler logic here
    ///             Ok(Response::ok("WebSocket connected"))
    ///         })
    ///     }),
    ///     RequestValidator::new(),
    ///     HandlerMeta::new("websocket_handler".to_string()),
    /// ).unwrap();
    /// ```
    pub fn websocket_route(
        &mut self,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        // WebSocket upgrade uses GET method
        self.route(HttpMethod::Get, path, handler, validator, metadata)
    }

    // Python handler registration methods

    /// Register a Python handler for GET requests
    ///
    /// This is a convenience method that wraps a PythonHandler and registers it
    /// with the appropriate method.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use data_bridge_api::{Router, PythonHandler};
    /// use data_bridge_api::validation::RequestValidator;
    /// use data_bridge_api::handler::HandlerMeta;
    /// use data_bridge_pyloop::PyLoop;
    /// use pyo3::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn main() -> PyResult<()> {
    /// Python::with_gil(|py| {
    ///     let mut router = Router::new();
    ///     let pyloop = Arc::new(PyLoop::new()?);
    ///
    ///     let handler_fn = py.eval("lambda req: {'status': 'ok'}", None, None)?;
    ///     let python_handler = PythonHandler::new(handler_fn.into(), pyloop);
    ///
    ///     router.get_python(
    ///         "/api/status",
    ///         python_handler,
    ///         RequestValidator::new(),
    ///         HandlerMeta::new("get_status".to_string()),
    ///     )?;
    ///     Ok(())
    /// })
    /// # }
    /// ```
    pub fn get_python(
        &mut self,
        path: &str,
        handler: crate::python_handler::PythonHandler,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.get(path, handler.into_handler_fn(), validator, metadata)
    }

    /// Register a Python handler for POST requests
    pub fn post_python(
        &mut self,
        path: &str,
        handler: crate::python_handler::PythonHandler,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.post(path, handler.into_handler_fn(), validator, metadata)
    }

    /// Register a Python handler for PUT requests
    pub fn put_python(
        &mut self,
        path: &str,
        handler: crate::python_handler::PythonHandler,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.put(path, handler.into_handler_fn(), validator, metadata)
    }

    /// Register a Python handler for PATCH requests
    pub fn patch_python(
        &mut self,
        path: &str,
        handler: crate::python_handler::PythonHandler,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.patch(path, handler.into_handler_fn(), validator, metadata)
    }

    /// Register a Python handler for DELETE requests
    pub fn delete_python(
        &mut self,
        path: &str,
        handler: crate::python_handler::PythonHandler,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.delete(path, handler.into_handler_fn(), validator, metadata)
    }

    /// Register a Python handler for any HTTP method
    pub fn route_python(
        &mut self,
        method: HttpMethod,
        path: &str,
        handler: crate::python_handler::PythonHandler,
        validator: RequestValidator,
        metadata: HandlerMeta,
    ) -> ApiResult<()> {
        self.route(method, path, handler.into_handler_fn(), validator, metadata)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}


/// Router builder for fluent API
pub struct RouterBuilder {
    prefix: String,
    tags: Vec<String>,
    routes: Vec<PendingRoute>,
}

struct PendingRoute {
    method: HttpMethod,
    path: String,
    handler: HandlerFn,
    validator: RequestValidator,
    metadata: HandlerMeta,
}

impl RouterBuilder {
    /// Create a new router builder
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            tags: Vec::new(),
            routes: Vec::new(),
        }
    }

    /// Set path prefix for all routes
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Add a tag to all routes
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Register a route
    pub fn route(
        mut self,
        method: HttpMethod,
        path: &str,
        handler: HandlerFn,
        validator: RequestValidator,
        mut metadata: HandlerMeta,
    ) -> Self {
        // Apply tags
        metadata.tags.extend(self.tags.clone());

        // Apply prefix
        let full_path = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}{}", self.prefix, path)
        };

        self.routes.push(PendingRoute {
            method,
            path: full_path,
            handler,
            validator,
            metadata,
        });

        self
    }

    /// Build into a Router
    pub fn build(self) -> ApiResult<Router> {
        let mut router = Router::new();
        for route in self.routes {
            router.route(
                route.method,
                &route.path,
                route.handler,
                route.validator,
                route.metadata,
            )?;
        }
        Ok(router)
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handler() -> HandlerFn {
        Arc::new(|_req, _validated| {
            Box::pin(async move { Ok(Response::ok()) })
        })
    }

    #[test]
    fn test_basic_routing() {
        let mut router = Router::new();
        router
            .get(
                "/users",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("get_users".to_string()),
            )
            .unwrap();
        router
            .get(
                "/users/:id",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("get_user".to_string()),
            )
            .unwrap();
        router
            .post(
                "/users",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("create_user".to_string()),
            )
            .unwrap();

        // Test match
        let m = router.match_route(HttpMethod::Get, "/users").unwrap();
        assert!(m.params.is_empty());

        let m = router.match_route(HttpMethod::Get, "/users/123").unwrap();
        assert_eq!(m.params.get("id"), Some(&"123".to_string()));

        // Test no match
        assert!(router.match_route(HttpMethod::Delete, "/users").is_none());
    }

    #[test]
    fn test_router_builder() {
        let router = RouterBuilder::new()
            .prefix("/api/v1")
            .tag("users")
            .route(
                HttpMethod::Get,
                "/users",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("get_users".to_string()),
            )
            .route(
                HttpMethod::Post,
                "/users",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("create_user".to_string()),
            )
            .build()
            .unwrap();

        let m = router
            .match_route(HttpMethod::Get, "/api/v1/users")
            .unwrap();
        assert!(m.params.is_empty());
    }

    #[test]
    fn test_path_params() {
        let mut router = Router::new();
        router
            .get(
                "/users/:user_id/posts/:post_id",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("get_post".to_string()),
            )
            .unwrap();

        let m = router
            .match_route(HttpMethod::Get, "/users/42/posts/99")
            .unwrap();
        assert_eq!(m.params.get("user_id"), Some(&"42".to_string()));
        assert_eq!(m.params.get("post_id"), Some(&"99".to_string()));
    }

    #[test]
    fn test_wildcard() {
        let mut router = Router::new();
        router
            .get(
                "/files/*path",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("get_file".to_string()),
            )
            .unwrap();

        let m = router
            .match_route(HttpMethod::Get, "/files/a/b/c.txt")
            .unwrap();
        assert_eq!(m.params.get("path"), Some(&"a/b/c.txt".to_string()));
    }

    #[test]
    fn test_method_not_allowed() {
        let mut router = Router::new();
        router
            .get(
                "/users",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("get_users".to_string()),
            )
            .unwrap();

        // GET exists
        assert!(router.match_route(HttpMethod::Get, "/users").is_some());

        // POST does not exist
        assert!(router.match_route(HttpMethod::Post, "/users").is_none());
    }

    #[test]
    fn test_route_not_found() {
        let router = Router::new();

        // No routes registered
        assert!(router.match_route(HttpMethod::Get, "/users").is_none());
    }

    #[test]
    fn test_multiple_methods_same_path() {
        let mut router = Router::new();
        router
            .get(
                "/users",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("get_users".to_string()),
            )
            .unwrap();
        router
            .post(
                "/users",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("create_user".to_string()),
            )
            .unwrap();

        // Both methods should work
        assert!(router.match_route(HttpMethod::Get, "/users").is_some());
        assert!(router.match_route(HttpMethod::Post, "/users").is_some());
    }

    #[test]
    fn test_router_builder_tags() {
        let router = RouterBuilder::new()
            .tag("api")
            .tag("v1")
            .route(
                HttpMethod::Get,
                "/test",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("test".to_string()),
            )
            .build()
            .unwrap();

        let m = router.match_route(HttpMethod::Get, "/test").unwrap();
        assert_eq!(m.route.metadata.tags.len(), 2);
        assert!(m.route.metadata.tags.contains(&"api".to_string()));
        assert!(m.route.metadata.tags.contains(&"v1".to_string()));
    }

    #[test]
    fn test_convenience_methods() {
        let mut router = Router::new();

        // Test all convenience methods
        router
            .get(
                "/test1",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("test1".to_string()),
            )
            .unwrap();
        router
            .post(
                "/test2",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("test2".to_string()),
            )
            .unwrap();
        router
            .put(
                "/test3",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("test3".to_string()),
            )
            .unwrap();
        router
            .patch(
                "/test4",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("test4".to_string()),
            )
            .unwrap();
        router
            .delete(
                "/test5",
                dummy_handler(),
                RequestValidator::new(),
                HandlerMeta::new("test5".to_string()),
            )
            .unwrap();

        assert!(router.match_route(HttpMethod::Get, "/test1").is_some());
        assert!(router.match_route(HttpMethod::Post, "/test2").is_some());
        assert!(router.match_route(HttpMethod::Put, "/test3").is_some());
        assert!(router.match_route(HttpMethod::Patch, "/test4").is_some());
        assert!(router.match_route(HttpMethod::Delete, "/test5").is_some());
    }
}
