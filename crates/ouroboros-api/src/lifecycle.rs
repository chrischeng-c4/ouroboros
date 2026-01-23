//! Application lifecycle hooks for ouroboros-api
//!
//! Provides startup and shutdown event handlers for resource initialization
//! and cleanup, similar to FastAPI's lifespan events.
//!
//! # Example
//!
//! ```rust,no_run
//! use ouroboros_api::lifecycle::LifecycleManager;
//!
//! # async fn example() {
//! let mut lifecycle = LifecycleManager::new();
//!
//! lifecycle.on_startup(|| async {
//!     println!("Starting up...");
//!     Ok(())
//! });
//!
//! lifecycle.on_shutdown(|| async {
//!     println!("Shutting down...");
//! });
//!
//! // Run startup hooks
//! lifecycle.startup().await.unwrap();
//!
//! // ... application runs ...
//!
//! // Run shutdown hooks
//! lifecycle.shutdown().await;
//! # }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Types
// ============================================================================

/// Boxed async startup hook that returns Result
pub type StartupHook = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync,
>;

/// Boxed async shutdown hook (no result - best effort)
pub type ShutdownHook = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
>;

/// Error during startup
#[derive(Debug, Clone)]
pub struct StartupError {
    /// Name of the hook that failed
    pub hook_name: Option<String>,
    /// Error message
    pub message: String,
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref name) = self.hook_name {
            write!(f, "Startup hook '{}' failed: {}", name, self.message)
        } else {
            write!(f, "Startup hook failed: {}", self.message)
        }
    }
}

impl std::error::Error for StartupError {}

// ============================================================================
// LifecycleManager
// ============================================================================

/// Manages application lifecycle hooks
///
/// Provides FastAPI-like startup and shutdown event handling.
#[derive(Default)]
pub struct LifecycleManager {
    /// Startup hooks (run in order)
    startup_hooks: Vec<(Option<String>, StartupHook)>,
    /// Shutdown hooks (run in reverse order)
    shutdown_hooks: Vec<(Option<String>, ShutdownHook)>,
    /// Whether startup has been run
    started: bool,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a startup hook
    ///
    /// Startup hooks are run in the order they are registered.
    /// If any hook fails, startup is aborted and an error is returned.
    pub fn on_startup<F, Fut>(&mut self, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.startup_hooks.push((
            None,
            Box::new(move || Box::pin(hook())),
        ));
    }

    /// Register a named startup hook
    pub fn on_startup_named<F, Fut>(&mut self, name: impl Into<String>, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.startup_hooks.push((
            Some(name.into()),
            Box::new(move || Box::pin(hook())),
        ));
    }

    /// Register a shutdown hook
    ///
    /// Shutdown hooks are run in reverse order of registration.
    /// Errors are logged but don't stop other hooks from running.
    pub fn on_shutdown<F, Fut>(&mut self, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.shutdown_hooks.push((
            None,
            Box::new(move || Box::pin(hook())),
        ));
    }

    /// Register a named shutdown hook
    pub fn on_shutdown_named<F, Fut>(&mut self, name: impl Into<String>, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.shutdown_hooks.push((
            Some(name.into()),
            Box::new(move || Box::pin(hook())),
        ));
    }

    /// Run all startup hooks
    ///
    /// Returns an error if any hook fails.
    pub async fn startup(&mut self) -> Result<(), StartupError> {
        for (name, hook) in &self.startup_hooks {
            tracing::info!(hook_name = ?name, "Running startup hook");

            match hook().await {
                Ok(()) => {
                    tracing::debug!(hook_name = ?name, "Startup hook completed");
                }
                Err(e) => {
                    tracing::error!(hook_name = ?name, error = %e, "Startup hook failed");
                    return Err(StartupError {
                        hook_name: name.clone(),
                        message: e,
                    });
                }
            }
        }

        self.started = true;
        tracing::info!("All startup hooks completed");
        Ok(())
    }

    /// Run all shutdown hooks
    ///
    /// Hooks are run in reverse order. Errors are logged but don't stop
    /// other hooks from running.
    pub async fn shutdown(&mut self) {
        tracing::info!("Running shutdown hooks");

        // Run in reverse order
        for (name, hook) in self.shutdown_hooks.iter().rev() {
            tracing::info!(hook_name = ?name, "Running shutdown hook");
            hook().await;
            tracing::debug!(hook_name = ?name, "Shutdown hook completed");
        }

        tracing::info!("All shutdown hooks completed");
    }

    /// Check if startup has been run
    pub fn is_started(&self) -> bool {
        self.started
    }

    /// Get the number of registered startup hooks
    pub fn startup_hook_count(&self) -> usize {
        self.startup_hooks.len()
    }

    /// Get the number of registered shutdown hooks
    pub fn shutdown_hook_count(&self) -> usize {
        self.shutdown_hooks.len()
    }
}

impl std::fmt::Debug for LifecycleManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LifecycleManager")
            .field("startup_hooks", &self.startup_hooks.len())
            .field("shutdown_hooks", &self.shutdown_hooks.len())
            .field("started", &self.started)
            .finish()
    }
}

// ============================================================================
// SharedLifecycleManager
// ============================================================================

/// Thread-safe lifecycle manager
///
/// Use this when lifecycle needs to be shared across threads.
#[derive(Clone, Default)]
pub struct SharedLifecycleManager {
    inner: Arc<Mutex<LifecycleManager>>,
}

impl SharedLifecycleManager {
    /// Create a new shared lifecycle manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a startup hook
    pub async fn on_startup<F, Fut>(&self, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.inner.lock().await.on_startup(hook);
    }

    /// Register a shutdown hook
    pub async fn on_shutdown<F, Fut>(&self, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.inner.lock().await.on_shutdown(hook);
    }

    /// Run all startup hooks
    pub async fn startup(&self) -> Result<(), StartupError> {
        self.inner.lock().await.startup().await
    }

    /// Run all shutdown hooks
    pub async fn shutdown(&self) {
        self.inner.lock().await.shutdown().await;
    }

    /// Check if started
    pub async fn is_started(&self) -> bool {
        self.inner.lock().await.is_started()
    }
}

impl std::fmt::Debug for SharedLifecycleManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedLifecycleManager").finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_startup_hooks() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut lifecycle = LifecycleManager::new();

        let c = counter.clone();
        lifecycle.on_startup(move || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let c = counter.clone();
        lifecycle.on_startup(move || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        lifecycle.startup().await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert!(lifecycle.is_started());
    }

    #[tokio::test]
    async fn test_shutdown_hooks_reverse_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut lifecycle = LifecycleManager::new();

        let o = order.clone();
        lifecycle.on_shutdown(move || {
            let o = o.clone();
            async move {
                o.lock().await.push(1);
            }
        });

        let o = order.clone();
        lifecycle.on_shutdown(move || {
            let o = o.clone();
            async move {
                o.lock().await.push(2);
            }
        });

        lifecycle.shutdown().await;
        let result = order.lock().await;
        assert_eq!(*result, vec![2, 1]); // Reverse order
    }

    #[tokio::test]
    async fn test_startup_error() {
        let mut lifecycle = LifecycleManager::new();

        lifecycle.on_startup_named("failing_hook", || async {
            Err("Something went wrong".to_string())
        });

        let result = lifecycle.startup().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.hook_name, Some("failing_hook".to_string()));
    }

    #[tokio::test]
    async fn test_named_hooks() {
        let mut lifecycle = LifecycleManager::new();

        lifecycle.on_startup_named("database", || async { Ok(()) });
        lifecycle.on_shutdown_named("database", || async {});

        assert_eq!(lifecycle.startup_hook_count(), 1);
        assert_eq!(lifecycle.shutdown_hook_count(), 1);
    }

    #[tokio::test]
    async fn test_shared_lifecycle_manager() {
        let lifecycle = SharedLifecycleManager::new();
        let counter = Arc::new(AtomicU32::new(0));

        let c = counter.clone();
        lifecycle.on_startup(move || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }).await;

        lifecycle.startup().await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(lifecycle.is_started().await);
    }

    // ========================================================================
    // Additional Tests for Edge Cases
    // ========================================================================

    #[tokio::test]
    async fn test_startup_execution_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut lifecycle = LifecycleManager::new();

        // Register hooks in specific order
        let o = order.clone();
        lifecycle.on_startup_named("first", move || {
            let o = o.clone();
            async move {
                o.lock().await.push("first");
                Ok(())
            }
        });

        let o = order.clone();
        lifecycle.on_startup_named("second", move || {
            let o = o.clone();
            async move {
                o.lock().await.push("second");
                Ok(())
            }
        });

        let o = order.clone();
        lifecycle.on_startup_named("third", move || {
            let o = o.clone();
            async move {
                o.lock().await.push("third");
                Ok(())
            }
        });

        lifecycle.startup().await.unwrap();

        let result = order.lock().await;
        assert_eq!(*result, vec!["first", "second", "third"]);
    }

    #[tokio::test]
    async fn test_startup_stops_on_first_error() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut lifecycle = LifecycleManager::new();

        let o = order.clone();
        lifecycle.on_startup_named("succeeds", move || {
            let o = o.clone();
            async move {
                o.lock().await.push("succeeds");
                Ok(())
            }
        });

        let o = order.clone();
        lifecycle.on_startup_named("fails", move || {
            let o = o.clone();
            async move {
                o.lock().await.push("fails");
                Err("Error".to_string())
            }
        });

        let o = order.clone();
        lifecycle.on_startup_named("never_runs", move || {
            let o = o.clone();
            async move {
                o.lock().await.push("never_runs");
                Ok(())
            }
        });

        let result = lifecycle.startup().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().hook_name, Some("fails".to_string()));

        let executed = order.lock().await;
        assert_eq!(*executed, vec!["succeeds", "fails"]);
        assert!(!executed.contains(&"never_runs"));
    }

    #[tokio::test]
    async fn test_empty_lifecycle_manager() {
        let mut lifecycle = LifecycleManager::new();

        // Empty startup should succeed
        let result = lifecycle.startup().await;
        assert!(result.is_ok());
        assert!(lifecycle.is_started());

        // Empty shutdown should not panic
        lifecycle.shutdown().await;
    }

    #[tokio::test]
    async fn test_hook_counts() {
        let mut lifecycle = LifecycleManager::new();

        assert_eq!(lifecycle.startup_hook_count(), 0);
        assert_eq!(lifecycle.shutdown_hook_count(), 0);

        lifecycle.on_startup(|| async { Ok(()) });
        lifecycle.on_startup(|| async { Ok(()) });
        lifecycle.on_shutdown(|| async {});

        assert_eq!(lifecycle.startup_hook_count(), 2);
        assert_eq!(lifecycle.shutdown_hook_count(), 1);
    }

    #[tokio::test]
    async fn test_startup_error_display() {
        let err = StartupError {
            hook_name: Some("database".to_string()),
            message: "Connection refused".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("database"));
        assert!(display.contains("Connection refused"));
    }

    #[tokio::test]
    async fn test_startup_error_without_name() {
        let err = StartupError {
            hook_name: None,
            message: "Unknown error".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("Unknown error"));
        assert!(!display.contains("''")); // Should not show empty quotes
    }

    #[tokio::test]
    async fn test_lifecycle_debug_format() {
        let mut lifecycle = LifecycleManager::new();
        lifecycle.on_startup(|| async { Ok(()) });
        lifecycle.on_shutdown(|| async {});

        let debug_str = format!("{:?}", lifecycle);
        assert!(debug_str.contains("LifecycleManager"));
        assert!(debug_str.contains("startup_hooks"));
        assert!(debug_str.contains("1")); // 1 startup hook
    }

    #[tokio::test]
    async fn test_shared_lifecycle_debug_format() {
        let lifecycle = SharedLifecycleManager::new();
        let debug_str = format!("{:?}", lifecycle);
        assert!(debug_str.contains("SharedLifecycleManager"));
    }

    #[tokio::test]
    async fn test_all_shutdown_hooks_run_even_with_slow_hook() {
        use std::time::Duration;

        let order = Arc::new(Mutex::new(Vec::new()));
        let mut lifecycle = LifecycleManager::new();

        let o = order.clone();
        lifecycle.on_shutdown_named("slow", move || {
            let o = o.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                o.lock().await.push("slow");
            }
        });

        let o = order.clone();
        lifecycle.on_shutdown_named("fast", move || {
            let o = o.clone();
            async move {
                o.lock().await.push("fast");
            }
        });

        lifecycle.shutdown().await;

        let result = order.lock().await;
        // Both should run (reverse order: fast then slow)
        assert_eq!(result.len(), 2);
        assert_eq!(*result, vec!["fast", "slow"]);
    }
}
