//! Argus Daemon - Long-running code analysis server

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, RwLock};

use crate::watch::{FileWatcher, WatchConfig};

use super::handler::RequestHandler;
use super::protocol::{Request, Response, RpcError};

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Root directory to analyze
    pub root: PathBuf,
    /// Unix socket path
    pub socket_path: PathBuf,
    /// Enable file watching
    pub watch: bool,
    /// Watch debounce duration
    pub debounce: Duration,
}

impl DaemonConfig {
    /// Create config with default socket path based on workspace hash
    pub fn new(root: PathBuf) -> Self {
        let socket_path = Self::default_socket_path(&root);
        Self {
            root,
            socket_path,
            watch: true,
            debounce: Duration::from_millis(300),
        }
    }

    /// Generate default socket path from workspace root
    pub fn default_socket_path(root: &PathBuf) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        root.hash(&mut hasher);
        let hash = hasher.finish();

        PathBuf::from(format!("/tmp/argus-{:x}.sock", hash))
    }

    /// Set custom socket path
    pub fn with_socket(mut self, path: PathBuf) -> Self {
        self.socket_path = path;
        self
    }

    /// Enable/disable file watching
    pub fn with_watch(mut self, enabled: bool) -> Self {
        self.watch = enabled;
        self
    }
}

/// Argus Daemon server
pub struct ArgusDaemon {
    config: DaemonConfig,
    handler: Arc<RequestHandler>,
    shutdown_tx: broadcast::Sender<()>,
    is_running: Arc<RwLock<bool>>,
}

impl ArgusDaemon {
    /// Create a new daemon
    pub fn new(config: DaemonConfig) -> Result<Self, String> {
        let handler = RequestHandler::new(config.root.clone())?;
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config,
            handler: Arc::new(handler),
            shutdown_tx,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Run the daemon
    pub async fn run(&self) -> Result<(), String> {
        // Remove existing socket if present
        if self.config.socket_path.exists() {
            std::fs::remove_file(&self.config.socket_path)
                .map_err(|e| format!("Failed to remove existing socket: {}", e))?;
        }

        // Create parent directory if needed
        if let Some(parent) = self.config.socket_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create socket directory: {}", e))?;
        }

        // Bind to socket
        let listener = UnixListener::bind(&self.config.socket_path)
            .map_err(|e| format!("Failed to bind socket: {}", e))?;

        println!("Argus daemon listening on {:?}", self.config.socket_path);

        // Mark as running
        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        // Start file watcher if enabled
        let _watcher_handle = if self.config.watch {
            Some(self.start_watcher().await?)
        } else {
            None
        };

        // Accept connections
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let handler = Arc::clone(&self.handler);
                            let mut shutdown_rx = self.shutdown_tx.subscribe();

                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, handler, &mut shutdown_rx).await {
                                    eprintln!("Connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    println!("Daemon shutting down...");
                    break;
                }
            }
        }

        // Cleanup
        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        // Remove socket file
        let _ = std::fs::remove_file(&self.config.socket_path);

        Ok(())
    }

    /// Start file watcher
    async fn start_watcher(&self) -> Result<tokio::task::JoinHandle<()>, String> {
        let watch_config = WatchConfig::new(self.config.root.clone())
            .with_debounce(self.config.debounce);

        let mut watcher = FileWatcher::new(watch_config)
            .map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher.start()
            .map_err(|e| format!("Failed to start watcher: {}", e))?;

        let handler = Arc::clone(&self.handler);

        // File watcher runs in its own thread via notify crate
        // For now, just spawn a placeholder task
        // TODO: Integrate with file watcher events properly
        let handle = tokio::spawn(async move {
            let _ = handler;
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });

        Ok(handle)
    }

    /// Request shutdown
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Check if daemon is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.config.socket_path
    }
}

/// Handle a single client connection
async fn handle_connection(
    stream: UnixStream,
    handler: Arc<RequestHandler>,
    shutdown_rx: &mut broadcast::Receiver<()>,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        tokio::select! {
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => {
                        // Connection closed
                        break;
                    }
                    Ok(_) => {
                        let response = process_request(&line, &handler).await;
                        let response_json = serde_json::to_string(&response)
                            .map_err(|e| format!("Failed to serialize response: {}", e))?;

                        writer.write_all(response_json.as_bytes()).await
                            .map_err(|e| format!("Failed to write response: {}", e))?;
                        writer.write_all(b"\n").await
                            .map_err(|e| format!("Failed to write newline: {}", e))?;
                        writer.flush().await
                            .map_err(|e| format!("Failed to flush: {}", e))?;

                        // Check for shutdown request
                        if line.contains("\"method\":\"shutdown\"") {
                            break;
                        }
                    }
                    Err(e) => {
                        return Err(format!("Read error: {}", e));
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }

    Ok(())
}

/// Process a single request
async fn process_request(line: &str, handler: &RequestHandler) -> Response {
    // Parse request
    let request: Request = match serde_json::from_str(line.trim()) {
        Ok(r) => r,
        Err(e) => {
            return Response::error(
                super::protocol::RequestId::Number(0),
                RpcError::parse_error(format!("Invalid JSON: {}", e)),
            );
        }
    };

    // Handle request
    handler.handle(request).await
}

/// Client for connecting to daemon
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    /// Create a client for the given socket
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Create a client for the default socket of a workspace
    pub fn for_workspace(root: &PathBuf) -> Self {
        let socket_path = DaemonConfig::default_socket_path(root);
        Self::new(socket_path)
    }

    /// Check if daemon is running
    pub async fn is_daemon_running(&self) -> bool {
        UnixStream::connect(&self.socket_path).await.is_ok()
    }

    /// Send a request and get response
    pub async fn request(&self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let stream = UnixStream::connect(&self.socket_path).await
            .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

        let (reader, mut writer) = stream.into_split();

        // Build request
        let request = Request::new(1i64, method, params);
        let request_json = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        // Send request
        writer.write_all(request_json.as_bytes()).await
            .map_err(|e| format!("Failed to send request: {}", e))?;
        writer.write_all(b"\n").await
            .map_err(|e| format!("Failed to send newline: {}", e))?;
        writer.flush().await
            .map_err(|e| format!("Failed to flush: {}", e))?;

        // Read response
        let mut reader = BufReader::new(reader);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // Parse response
        let response: Response = serde_json::from_str(response_line.trim())
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = response.error {
            Err(format!("RPC error {}: {}", error.code, error.message))
        } else {
            Ok(response.result.unwrap_or(serde_json::Value::Null))
        }
    }

    /// Convenience methods
    pub async fn check(&self, path: &str) -> Result<serde_json::Value, String> {
        self.request("check", Some(serde_json::json!({ "path": path }))).await
    }

    pub async fn type_at(&self, file: &str, line: u32, column: u32) -> Result<serde_json::Value, String> {
        self.request("type_at", Some(serde_json::json!({
            "file": file,
            "line": line,
            "column": column
        }))).await
    }

    pub async fn symbols(&self, file: &str) -> Result<serde_json::Value, String> {
        self.request("symbols", Some(serde_json::json!({ "file": file }))).await
    }

    pub async fn diagnostics(&self, file: Option<&str>) -> Result<serde_json::Value, String> {
        let params = file.map(|f| serde_json::json!({ "file": f }));
        self.request("diagnostics", params).await
    }

    pub async fn index_status(&self) -> Result<serde_json::Value, String> {
        self.request("index_status", None).await
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        self.request("shutdown", None).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_generation() {
        let root1 = PathBuf::from("/home/user/project1");
        let root2 = PathBuf::from("/home/user/project2");

        let path1 = DaemonConfig::default_socket_path(&root1);
        let path2 = DaemonConfig::default_socket_path(&root2);

        // Different roots should produce different socket paths
        assert_ne!(path1, path2);

        // Same root should produce same path
        let path1_again = DaemonConfig::default_socket_path(&root1);
        assert_eq!(path1, path1_again);
    }

    #[test]
    fn test_daemon_config() {
        let root = PathBuf::from("/test/project");
        let config = DaemonConfig::new(root.clone());

        assert_eq!(config.root, root);
        assert!(config.watch);
        assert_eq!(config.debounce, Duration::from_millis(300));
    }
}
