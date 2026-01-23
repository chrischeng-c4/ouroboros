use anyhow::Result;
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

pub mod hmr;
pub mod watcher;

use hmr::{HmrManager, HmrMessage};
use watcher::FileWatcher;

/// Development server with HMR support
pub struct DevServer {
    /// Bundler instance
    bundler: Arc<ouroboros_talos_bundler::Bundler>,

    /// File watcher
    watcher: Arc<FileWatcher>,

    /// HMR manager
    hmr_manager: Arc<HmrManager>,

    /// Server configuration
    config: ServerConfig,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,

    /// Port to bind to
    pub port: u16,

    /// Project root directory
    pub root_dir: PathBuf,

    /// Public directory for static assets
    pub public_dir: Option<PathBuf>,
}

/// Server state shared across handlers
#[derive(Clone)]
struct ServerState {
    bundler: Arc<ouroboros_talos_bundler::Bundler>,
    hmr_manager: Arc<HmrManager>,
}

impl DevServer {
    /// Create a new development server
    pub fn new(
        bundler: ouroboros_talos_bundler::Bundler,
        config: ServerConfig,
    ) -> Result<Self> {
        let bundler = Arc::new(bundler);
        let hmr_manager = Arc::new(HmrManager::new());

        // Create file watcher
        let watcher = Arc::new(FileWatcher::new(config.root_dir.clone())?);

        Ok(Self {
            bundler,
            watcher,
            hmr_manager,
            config,
        })
    }

    /// Start the development server
    pub async fn start(self: Arc<Self>) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port)
            .parse::<SocketAddr>()?;

        tracing::info!("Starting dev server on http://{}", addr);

        // Create router
        let app = self.create_router();

        // Start file watcher
        self.start_file_watcher().await?;

        // Start server
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("✓ Dev server running on http://{}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Create Axum router
    fn create_router(self: &Arc<Self>) -> Router {
        let state = ServerState {
            bundler: self.bundler.clone(),
            hmr_manager: self.hmr_manager.clone(),
        };

        Router::new()
            .route("/__talos_hmr", get(hmr_websocket_handler))
            .route("/*path", get(serve_handler))
            .with_state(state)
    }

    /// Start file watcher for HMR
    async fn start_file_watcher(self: &Arc<Self>) -> Result<()> {
        let watcher = self.watcher.clone();
        let hmr_manager = self.hmr_manager.clone();
        let _bundler = self.bundler.clone();

        tokio::spawn(async move {
            let mut rx = watcher.subscribe();

            while let Ok(path) = rx.recv().await {
                tracing::info!("File changed: {:?}", path);

                // TODO: Trigger incremental rebuild
                // let result = bundler.rebuild(&path).await;

                // Send HMR update
                let message = HmrMessage::Update {
                    path: path.to_string_lossy().to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };

                hmr_manager.broadcast(message).await;
            }
        });

        Ok(())
    }
}

/// WebSocket handler for HMR
async fn hmr_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(|socket| hmr_websocket(socket, state))
}

/// Handle HMR WebSocket connection
async fn hmr_websocket(socket: WebSocket, state: ServerState) {
    tracing::info!("New HMR client connected");

    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.hmr_manager.subscribe();

    // Spawn task to forward HMR messages to client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender
                .send(axum::extract::ws::Message::Text(json.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Handle incoming messages from client (ping/pong, etc.)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                axum::extract::ws::Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    tracing::info!("HMR client disconnected");
}

/// Serve files handler
async fn serve_handler(
    State(state): State<ServerState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    tracing::debug!("Serving: {}", path);

    // Handle bundle request
    if path == "bundle.js" || path == "main.js" {
        return serve_bundle(state).await;
    }

    // Handle index.html (inject HMR client)
    if path.is_empty() || path == "/" || path == "index.html" {
        return serve_index_html().await;
    }

    // Try to serve static file
    if let Some(content) = serve_static_file(&path).await {
        return content;
    }

    // Fall back to index.html for SPA routing
    serve_index_html().await
}

/// Serve the JavaScript bundle
async fn serve_bundle(state: ServerState) -> Response {
    // TODO: Get entry point from config
    let entry = PathBuf::from("src/index.js");

    match state.bundler.bundle(entry).await {
        Ok(output) => {
            let mut code = output.code;

            // Inject HMR client code
            code.push_str("\n\n");
            code.push_str(&generate_hmr_client());

            (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/javascript; charset=utf-8",
                )],
                code,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Bundle error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Bundle error: {}", e),
            )
                .into_response()
        }
    }
}

/// Serve index.html
async fn serve_index_html() -> Response {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Talos Dev Server</title>
</head>
<body>
    <div id="root"></div>
    <script src="/bundle.js"></script>
</body>
</html>"#;

    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

/// Serve static file
async fn serve_static_file(path: &str) -> Option<Response> {
    let file_path = PathBuf::from("public").join(path);

    if !file_path.exists() {
        return None;
    }

    let content = std::fs::read(&file_path).ok()?;
    let content_type = guess_content_type(&file_path);

    Some(
        (
            [(axum::http::header::CONTENT_TYPE, content_type.as_str())],
            content,
        )
            .into_response(),
    )
}

/// Guess content type from file extension
fn guess_content_type(path: &PathBuf) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8".to_string(),
        Some("css") => "text/css; charset=utf-8".to_string(),
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8".to_string(),
        Some("json") => "application/json; charset=utf-8".to_string(),
        Some("png") => "image/png".to_string(),
        Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
        Some("gif") => "image/gif".to_string(),
        Some("svg") => "image/svg+xml".to_string(),
        Some("woff") => "font/woff".to_string(),
        Some("woff2") => "font/woff2".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

/// Generate HMR client code
fn generate_hmr_client() -> String {
    r#"// Talos HMR Client
(function() {
  if (typeof window === 'undefined') return;

  console.log('[Talos] Connecting to HMR server...');

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = window.location.host;
  const ws = new WebSocket(`${protocol}//${host}/__talos_hmr`);

  ws.onopen = () => {
    console.log('[Talos] HMR connected');
  };

  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    console.log('[Talos] HMR message:', message);

    switch (message.type) {
      case 'update':
        console.log('[Talos] Module updated:', message.path);
        // For now, do a full reload
        window.location.reload();
        break;

      case 'full-reload':
        console.log('[Talos] Full reload:', message.reason);
        window.location.reload();
        break;

      case 'error':
        console.error('[Talos] Error:', message.message);
        break;
    }
  };

  ws.onerror = (error) => {
    console.error('[Talos] HMR connection error:', error);
  };

  ws.onclose = () => {
    console.log('[Talos] HMR disconnected. Retrying in 1s...');
    setTimeout(() => window.location.reload(), 1000);
  };
})();
"#
    .to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            root_dir: PathBuf::from("."),
            public_dir: Some(PathBuf::from("public")),
        }
    }
}
