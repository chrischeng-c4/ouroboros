//! Custom exception handling for the event loop
//!
//! Provides configurable exception handling with context information,
//! similar to Python asyncio's exception handler.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

// ============================================================================
// Exception Context
// ============================================================================

/// Context information for an exception
#[derive(Debug, Clone)]
pub struct ExceptionContext {
    /// The exception message
    pub message: String,
    /// The exception type/name
    pub exception_type: Option<String>,
    /// The exception itself (as string representation)
    pub exception: Option<String>,
    /// Future that raised the exception
    pub future: Option<String>,
    /// Handle that raised the exception
    pub handle: Option<String>,
    /// Protocol involved
    pub protocol: Option<String>,
    /// Transport involved
    pub transport: Option<String>,
    /// Socket involved
    pub socket: Option<String>,
    /// Source traceback
    pub source_traceback: Option<String>,
    /// Additional context data
    pub extra: HashMap<String, String>,
}

impl ExceptionContext {
    /// Create a new exception context
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exception_type: None,
            exception: None,
            future: None,
            handle: None,
            protocol: None,
            transport: None,
            socket: None,
            source_traceback: None,
            extra: HashMap::new(),
        }
    }

    /// Set exception type
    pub fn exception_type(mut self, typ: impl Into<String>) -> Self {
        self.exception_type = Some(typ.into());
        self
    }

    /// Set exception
    pub fn exception(mut self, exc: impl Into<String>) -> Self {
        self.exception = Some(exc.into());
        self
    }

    /// Set future
    pub fn future(mut self, future: impl Into<String>) -> Self {
        self.future = Some(future.into());
        self
    }

    /// Set handle
    pub fn handle(mut self, handle: impl Into<String>) -> Self {
        self.handle = Some(handle.into());
        self
    }

    /// Set protocol
    pub fn protocol(mut self, protocol: impl Into<String>) -> Self {
        self.protocol = Some(protocol.into());
        self
    }

    /// Set transport
    pub fn transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = Some(transport.into());
        self
    }

    /// Set socket
    pub fn socket(mut self, socket: impl Into<String>) -> Self {
        self.socket = Some(socket.into());
        self
    }

    /// Set source traceback
    pub fn source_traceback(mut self, traceback: impl Into<String>) -> Self {
        self.source_traceback = Some(traceback.into());
        self
    }

    /// Add extra context
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// Get a context value by key
    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "message" => Some(&self.message),
            "exception_type" => self.exception_type.as_deref(),
            "exception" => self.exception.as_deref(),
            "future" => self.future.as_deref(),
            "handle" => self.handle.as_deref(),
            "protocol" => self.protocol.as_deref(),
            "transport" => self.transport.as_deref(),
            "socket" => self.socket.as_deref(),
            "source_traceback" => self.source_traceback.as_deref(),
            _ => self.extra.get(key).map(|s| s.as_str()),
        }
    }

    /// Format context for logging
    pub fn format(&self) -> String {
        let mut parts = vec![format!("Message: {}", self.message)];

        if let Some(ref exc_type) = self.exception_type {
            parts.push(format!("Exception type: {}", exc_type));
        }
        if let Some(ref exc) = self.exception {
            parts.push(format!("Exception: {}", exc));
        }
        if let Some(ref future) = self.future {
            parts.push(format!("Future: {}", future));
        }
        if let Some(ref handle) = self.handle {
            parts.push(format!("Handle: {}", handle));
        }
        if let Some(ref protocol) = self.protocol {
            parts.push(format!("Protocol: {}", protocol));
        }
        if let Some(ref transport) = self.transport {
            parts.push(format!("Transport: {}", transport));
        }
        if let Some(ref traceback) = self.source_traceback {
            parts.push(format!("Source traceback:\n{}", traceback));
        }

        for (key, value) in &self.extra {
            parts.push(format!("{}: {}", key, value));
        }

        parts.join("\n")
    }
}

// ============================================================================
// Exception Handler
// ============================================================================

/// Exception handler function type
pub type ExceptionHandlerFn = Box<dyn Fn(&ExceptionContext) + Send + Sync>;

/// Exception handler manager
pub struct ExceptionHandlerManager {
    /// Custom handler
    custom_handler: RwLock<Option<ExceptionHandlerFn>>,
    /// Whether to log unhandled exceptions
    log_unhandled: RwLock<bool>,
    /// Exception history
    history: RwLock<Vec<ExceptionContext>>,
    /// Maximum history size
    max_history: usize,
}

impl ExceptionHandlerManager {
    /// Create a new exception handler manager
    pub fn new() -> Self {
        Self {
            custom_handler: RwLock::new(None),
            log_unhandled: RwLock::new(true),
            history: RwLock::new(Vec::new()),
            max_history: 100,
        }
    }

    /// Set custom exception handler
    pub fn set_exception_handler<F>(&self, handler: F)
    where
        F: Fn(&ExceptionContext) + Send + Sync + 'static,
    {
        *self.custom_handler.write() = Some(Box::new(handler));
    }

    /// Get current exception handler (returns None if using default)
    pub fn get_exception_handler(&self) -> bool {
        self.custom_handler.read().is_some()
    }

    /// Clear custom exception handler (revert to default)
    pub fn clear_exception_handler(&self) {
        *self.custom_handler.write() = None;
    }

    /// Set whether to log unhandled exceptions
    pub fn set_log_unhandled(&self, log: bool) {
        *self.log_unhandled.write() = log;
    }

    /// Call the exception handler
    pub fn call_exception_handler(&self, context: ExceptionContext) {
        // Add to history
        {
            let mut history = self.history.write();
            history.push(context.clone());
            while history.len() > self.max_history {
                history.remove(0);
            }
        }

        // Try custom handler first
        let handler = self.custom_handler.read();
        if let Some(ref h) = *handler {
            h(&context);
            return;
        }
        drop(handler);

        // Default handler - log the exception
        if *self.log_unhandled.read() {
            self.default_exception_handler(&context);
        }
    }

    /// Default exception handler
    fn default_exception_handler(&self, context: &ExceptionContext) {
        eprintln!("Unhandled exception in event loop:");
        eprintln!("{}", context.format());
    }

    /// Get exception history
    pub fn get_history(&self) -> Vec<ExceptionContext> {
        self.history.read().clone()
    }

    /// Clear exception history
    pub fn clear_history(&self) {
        self.history.write().clear();
    }

    /// Get recent exceptions count
    pub fn recent_exception_count(&self) -> usize {
        self.history.read().len()
    }
}

impl Default for ExceptionHandlerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared exception handler manager
pub type SharedExceptionHandler = Arc<ExceptionHandlerManager>;

/// Create a shared exception handler
pub fn shared_exception_handler() -> SharedExceptionHandler {
    Arc::new(ExceptionHandlerManager::new())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create context from an error
pub fn context_from_error(error: &dyn std::error::Error) -> ExceptionContext {
    let mut context = ExceptionContext::new(error.to_string());

    // Try to get the error type name
    let type_name = std::any::type_name_of_val(error);
    context.exception_type = Some(type_name.to_string());

    // Include source if available
    if let Some(source) = error.source() {
        context.extra.insert("source".to_string(), source.to_string());
    }

    context
}

/// Create context from IO error
pub fn context_from_io_error(error: &std::io::Error) -> ExceptionContext {
    ExceptionContext::new(error.to_string())
        .exception_type(format!("IOError::{:?}", error.kind()))
        .exception(error.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_exception_context() {
        let ctx = ExceptionContext::new("Test error")
            .exception_type("ValueError")
            .exception("invalid value")
            .future("Future<Task>")
            .extra("custom_key", "custom_value");

        assert_eq!(ctx.message, "Test error");
        assert_eq!(ctx.exception_type, Some("ValueError".to_string()));
        assert_eq!(ctx.get("custom_key"), Some("custom_value"));
    }

    #[test]
    fn test_context_format() {
        let ctx = ExceptionContext::new("Test error")
            .exception_type("Error")
            .exception("details");

        let formatted = ctx.format();
        assert!(formatted.contains("Test error"));
        assert!(formatted.contains("Error"));
    }

    #[test]
    fn test_exception_handler_manager() {
        let manager = ExceptionHandlerManager::new();

        let ctx = ExceptionContext::new("Test exception");
        manager.call_exception_handler(ctx);

        assert_eq!(manager.recent_exception_count(), 1);
    }

    #[test]
    fn test_custom_exception_handler() {
        let manager = ExceptionHandlerManager::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        manager.set_exception_handler(move |_ctx| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        let ctx = ExceptionContext::new("Test");
        manager.call_exception_handler(ctx);

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_clear_exception_handler() {
        let manager = ExceptionHandlerManager::new();

        manager.set_exception_handler(|_| {});
        assert!(manager.get_exception_handler());

        manager.clear_exception_handler();
        assert!(!manager.get_exception_handler());
    }

    #[test]
    fn test_exception_history() {
        let manager = ExceptionHandlerManager::new();
        manager.set_log_unhandled(false);

        manager.call_exception_handler(ExceptionContext::new("Error 1"));
        manager.call_exception_handler(ExceptionContext::new("Error 2"));

        let history = manager.get_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].message, "Error 1");
        assert_eq!(history[1].message, "Error 2");

        manager.clear_history();
        assert_eq!(manager.recent_exception_count(), 0);
    }
}
