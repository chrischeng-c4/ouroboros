//! Development server with hot reload support.
//!
//! Provides a development server that watches for file changes and automatically
//! restarts the server, similar to uvicorn's --reload flag.
//!
//! # Example
//!
//! ```rust,no_run
//! use ouroboros_api::dev_server::{DevServer, DevServerConfig};
//! use std::path::PathBuf;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = DevServerConfig {
//!         bind_addr: "127.0.0.1:8000".to_string(),
//!         watch_dirs: vec![PathBuf::from("./python")],
//!         watch_extensions: vec!["py".to_string(), "rs".to_string()],
//!         debounce: Duration::from_millis(500),
//!         exclude_patterns: vec!["__pycache__".to_string()],
//!         hot_reload: true,
//!     };
//!
//!     let dev_server = DevServer::new(config);
//!     dev_server.run().await.unwrap();
//! }
//! ```

use crate::error::ApiError;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn, debug};

/// Configuration for the development server.
#[derive(Debug, Clone)]
pub struct DevServerConfig {
    /// Bind address (e.g., "127.0.0.1:8000")
    pub bind_addr: String,
    /// Directories to watch for changes
    pub watch_dirs: Vec<PathBuf>,
    /// File extensions to watch (e.g., ["py", "rs"])
    pub watch_extensions: Vec<String>,
    /// Debounce duration to prevent rapid restarts
    pub debounce: Duration,
    /// Patterns to exclude from watching
    pub exclude_patterns: Vec<String>,
    /// Whether hot reload is enabled
    pub hot_reload: bool,
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8000".to_string(),
            watch_dirs: vec![PathBuf::from("./python")],
            watch_extensions: vec!["py".to_string()],
            debounce: Duration::from_millis(500),
            exclude_patterns: vec![
                "__pycache__".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".pytest_cache".to_string(),
            ],
            hot_reload: true,
        }
    }
}

/// Development server with file watching and hot reload.
pub struct DevServer {
    config: DevServerConfig,
}

impl DevServer {
    /// Create a new development server.
    pub fn new(config: DevServerConfig) -> Self {
        Self { config }
    }

    /// Run the development server.
    ///
    /// If hot reload is enabled, this will watch for file changes and
    /// signal the server to restart when changes are detected.
    pub async fn run(self) -> Result<(), ApiError> {
        if !self.config.hot_reload {
            // Run without hot reload - just start the server
            info!(
                "Starting server on http://{} (hot reload disabled)",
                self.config.bind_addr
            );
            return self.run_server_once().await;
        }

        // Set up file watcher
        let (tx, mut rx) = mpsc::channel::<()>(1);
        let should_restart = Arc::new(AtomicBool::new(false));
        let should_restart_clone = should_restart.clone();

        // Create file watcher
        let watcher_result = self.create_watcher(tx);
        let _watcher = match watcher_result {
            Ok(w) => w,
            Err(e) => {
                warn!("Failed to create file watcher: {}. Running without hot reload.", e);
                return self.run_server_once().await;
            }
        };

        info!(
            "Starting dev server on http://{} with hot reload",
            self.config.bind_addr
        );
        info!("Watching directories: {:?}", self.config.watch_dirs);
        info!("Watching extensions: {:?}", self.config.watch_extensions);

        // Main server loop
        loop {
            should_restart.store(false, Ordering::SeqCst);

            // Spawn server task
            let bind_addr = self.config.bind_addr.clone();
            let restart_flag = should_restart_clone.clone();

            let mut server_handle = tokio::spawn(async move {
                Self::run_server_with_shutdown(bind_addr, restart_flag).await
            });

            // Wait for either:
            // 1. File change notification
            // 2. Server to exit (error or ctrl-c)
            enum SelectResult {
                FileChange,
                ServerResult(Result<Result<(), ApiError>, tokio::task::JoinError>),
            }

            let select_result = tokio::select! {
                _ = rx.recv() => SelectResult::FileChange,
                result = &mut server_handle => SelectResult::ServerResult(result),
            };

            match select_result {
                SelectResult::FileChange => {
                    info!("File change detected, restarting server...");
                    should_restart.store(true, Ordering::SeqCst);

                    // Give the server a moment to notice the restart flag
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    // Abort the server task if it hasn't stopped
                    server_handle.abort();

                    // Wait a bit for cleanup
                    tokio::time::sleep(self.config.debounce).await;
                }
                SelectResult::ServerResult(result) => {
                    match result {
                        Ok(Ok(())) => {
                            // Server exited normally (probably ctrl-c)
                            info!("Server stopped");
                            break;
                        }
                        Ok(Err(e)) => {
                            warn!("Server error: {}", e);
                            // Wait before restart on error
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                        Err(e) => {
                            if e.is_cancelled() {
                                // Task was cancelled for restart, continue loop
                                continue;
                            }
                            warn!("Server task error: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Run server once without hot reload.
    async fn run_server_once(&self) -> Result<(), ApiError> {
        use crate::{Router, Server, ServerConfig};

        let router = Router::new();
        let server_config = ServerConfig::new(&self.config.bind_addr);
        let server = Server::new(router, server_config);

        server.run().await
            .map_err(|e| ApiError::Internal(format!("Server error: {}", e)))
    }

    /// Run server with shutdown flag.
    async fn run_server_with_shutdown(
        bind_addr: String,
        should_restart: Arc<AtomicBool>,
    ) -> Result<(), ApiError> {
        use crate::{Router, Server, ServerConfig};
        use tokio::signal;

        let router = Router::new();
        let server_config = ServerConfig::new(&bind_addr);
        let server = Server::new(router, server_config);

        // Create a future that completes when we should restart
        let restart_check = async {
            loop {
                if should_restart.load(Ordering::SeqCst) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        };

        // Run server with graceful shutdown
        tokio::select! {
            result = server.run() => {
                result.map_err(|e| ApiError::Internal(format!("Server error: {}", e)))
            }
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
                Ok(())
            }
            _ = restart_check => {
                info!("Restart requested, stopping server...");
                Ok(())
            }
        }
    }

    /// Create file watcher.
    fn create_watcher(&self, tx: mpsc::Sender<()>) -> Result<RecommendedWatcher, ApiError> {
        let extensions = self.config.watch_extensions.clone();
        let exclude_patterns = self.config.exclude_patterns.clone();
        let debounce = self.config.debounce;

        // Track last event time for debouncing
        let last_event = Arc::new(std::sync::Mutex::new(std::time::Instant::now() - debounce));

        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        // Check if any path matches our criteria
                        let should_trigger = event.paths.iter().any(|path| {
                            // Check extension
                            let ext_match = path.extension()
                                .and_then(|e| e.to_str())
                                .map(|e| extensions.iter().any(|ext| ext == e))
                                .unwrap_or(false);

                            if !ext_match {
                                return false;
                            }

                            // Check exclusions
                            let path_str = path.to_string_lossy();
                            let excluded = exclude_patterns.iter()
                                .any(|pattern| path_str.contains(pattern));

                            !excluded
                        });

                        if should_trigger {
                            // Debounce check
                            let mut last = last_event.lock().unwrap();
                            let now = std::time::Instant::now();

                            if now.duration_since(*last) >= debounce {
                                *last = now;
                                debug!("File change detected: {:?}", event.paths);
                                let _ = tx.try_send(());
                            }
                        }
                    }
                    Err(e) => {
                        warn!("File watcher error: {}", e);
                    }
                }
            },
            Config::default(),
        ).map_err(|e| ApiError::Internal(format!("Failed to create watcher: {}", e)))?;

        // Watch all configured directories
        for dir in &self.config.watch_dirs {
            if dir.exists() {
                watcher.watch(dir, RecursiveMode::Recursive)
                    .map_err(|e| ApiError::Internal(format!("Failed to watch {}: {}", dir.display(), e)))?;
                debug!("Watching directory: {}", dir.display());
            } else {
                warn!("Watch directory does not exist: {}", dir.display());
            }
        }

        Ok(watcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DevServerConfig::default();
        assert!(config.hot_reload);
        assert!(!config.watch_extensions.is_empty());
        assert!(!config.exclude_patterns.is_empty());
    }

    #[test]
    fn test_dev_server_creation() {
        let config = DevServerConfig {
            bind_addr: "127.0.0.1:9000".to_string(),
            watch_dirs: vec![PathBuf::from("./test")],
            watch_extensions: vec!["py".to_string()],
            debounce: Duration::from_millis(100),
            exclude_patterns: vec![],
            hot_reload: false,
        };

        let _server = DevServer::new(config);
    }
}
