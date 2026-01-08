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
