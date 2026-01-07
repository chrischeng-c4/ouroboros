//! Application builder and runner
//!
//! The App struct is the main entry point for creating a data-bridge API application.

use crate::router::Router;

/// API application builder
pub struct App {
    router: Router,
}

impl App {
    /// Create a new API application
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    /// Get a reference to the router
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Get a mutable reference to the router
    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
