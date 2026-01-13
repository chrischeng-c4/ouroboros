//! WebSocket support for data-bridge-api
//!
//! This module provides WebSocket functionality for real-time bidirectional
//! communication between client and server. It follows the data-bridge
//! architecture principles:
//! - GIL-free message processing
//! - Async/await based on Tokio runtime
//! - Type-safe message handling
//! - Proper error handling with `ApiError`
//!
//! # Architecture
//!
//! ```text
//! HTTP Request → WebSocketUpgrade → WebSocket Connection
//!     ↓                                       ↓
//! Validate headers                    Send/Receive Messages
//!     ↓                                       ↓
//! Generate accept key                 Text/Binary/Ping/Pong/Close
//!     ↓
//! Upgrade to WebSocket
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use data_bridge_api::websocket::{WebSocket, is_websocket_upgrade};
//! use data_bridge_api::request::SerializableRequest;
//! use hyper::{Request, Body};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Check if request is a WebSocket upgrade
//! let req = SerializableRequest::new(
//!     data_bridge_api::request::HttpMethod::Get,
//!     "/ws"
//! )
//! .with_header("upgrade", "websocket")
//! .with_header("connection", "upgrade")
//! .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
//! .with_header("sec-websocket-version", "13");
//!
//! if is_websocket_upgrade(&req) {
//!     // Upgrade to WebSocket connection
//!     // (in real usage, you'd use the Hyper request)
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{ApiError, ApiResult};
use crate::request::SerializableRequest;
use futures_util::{SinkExt, StreamExt};
use hyper::body::Bytes;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as TungsteniteCloseFrame;
use tokio_tungstenite::tungstenite::protocol::Message as TungsteniteMessage;
use tokio_tungstenite::tungstenite::protocol::frame::Utf8Bytes;
use tokio_tungstenite::{accept_async, WebSocketStream};
use tracing::{debug, error, warn};

// ============================================================================
// Core Types
// ============================================================================

/// WebSocket message types
///
/// Represents the different types of messages that can be sent/received
/// over a WebSocket connection.
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketMessage {
    /// Text message (UTF-8 encoded)
    Text(String),
    /// Binary message (raw bytes)
    Binary(Vec<u8>),
    /// Ping message (with optional payload)
    Ping(Vec<u8>),
    /// Pong message (with optional payload)
    Pong(Vec<u8>),
    /// Close message (with optional close frame)
    Close(Option<CloseFrame>),
}

impl WebSocketMessage {
    /// Check if this is a text message
    pub fn is_text(&self) -> bool {
        matches!(self, WebSocketMessage::Text(_))
    }

    /// Check if this is a binary message
    pub fn is_binary(&self) -> bool {
        matches!(self, WebSocketMessage::Binary(_))
    }

    /// Check if this is a ping message
    pub fn is_ping(&self) -> bool {
        matches!(self, WebSocketMessage::Ping(_))
    }

    /// Check if this is a pong message
    pub fn is_pong(&self) -> bool {
        matches!(self, WebSocketMessage::Pong(_))
    }

    /// Check if this is a close message
    pub fn is_close(&self) -> bool {
        matches!(self, WebSocketMessage::Close(_))
    }

    /// Get as text if this is a text message
    pub fn as_text(&self) -> Option<&str> {
        if let WebSocketMessage::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Get as binary if this is a binary message
    pub fn as_binary(&self) -> Option<&[u8]> {
        if let WebSocketMessage::Binary(b) = self {
            Some(b)
        } else {
            None
        }
    }

    /// Convert to tungstenite message
    fn to_tungstenite(&self) -> TungsteniteMessage {
        match self {
            WebSocketMessage::Text(s) => {
                TungsteniteMessage::Text(Utf8Bytes::from(s.clone()))
            }
            WebSocketMessage::Binary(b) => {
                TungsteniteMessage::Binary(Bytes::from(b.clone()))
            }
            WebSocketMessage::Ping(p) => {
                TungsteniteMessage::Ping(Bytes::from(p.clone()))
            }
            WebSocketMessage::Pong(p) => {
                TungsteniteMessage::Pong(Bytes::from(p.clone()))
            }
            WebSocketMessage::Close(Some(frame)) => {
                TungsteniteMessage::Close(Some(TungsteniteCloseFrame {
                    code: frame.code.into(),
                    reason: Utf8Bytes::from(frame.reason.clone()),
                }))
            }
            WebSocketMessage::Close(None) => TungsteniteMessage::Close(None),
        }
    }

    /// Create from tungstenite message
    fn from_tungstenite(msg: TungsteniteMessage) -> Option<Self> {
        match msg {
            TungsteniteMessage::Text(s) => Some(WebSocketMessage::Text(s.to_string())),
            TungsteniteMessage::Binary(b) => Some(WebSocketMessage::Binary(b.to_vec())),
            TungsteniteMessage::Ping(p) => Some(WebSocketMessage::Ping(p.to_vec())),
            TungsteniteMessage::Pong(p) => Some(WebSocketMessage::Pong(p.to_vec())),
            TungsteniteMessage::Close(frame) => {
                let close_frame = frame.map(|f| CloseFrame {
                    code: f.code.into(),
                    reason: f.reason.to_string(),
                });
                Some(WebSocketMessage::Close(close_frame))
            }
            TungsteniteMessage::Frame(_) => None, // Raw frames not exposed
        }
    }
}

/// WebSocket close frame
///
/// Contains the close code and optional reason when closing a WebSocket connection.
#[derive(Debug, Clone, PartialEq)]
pub struct CloseFrame {
    /// Close status code
    pub code: u16,
    /// Close reason (UTF-8 string, max 123 bytes)
    pub reason: String,
}

impl CloseFrame {
    /// Create a new close frame
    pub fn new(code: u16, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
        }
    }

    /// Normal closure (1000)
    pub fn normal() -> Self {
        Self {
            code: 1000,
            reason: String::new(),
        }
    }

    /// Going away (1001) - endpoint is going away
    pub fn going_away(reason: impl Into<String>) -> Self {
        Self {
            code: 1001,
            reason: reason.into(),
        }
    }

    /// Protocol error (1002)
    pub fn protocol_error(reason: impl Into<String>) -> Self {
        Self {
            code: 1002,
            reason: reason.into(),
        }
    }

    /// Unsupported data (1003)
    pub fn unsupported() -> Self {
        Self {
            code: 1003,
            reason: String::new(),
        }
    }

    /// Invalid frame payload data (1007)
    pub fn invalid_data(reason: impl Into<String>) -> Self {
        Self {
            code: 1007,
            reason: reason.into(),
        }
    }

    /// Policy violation (1008)
    pub fn policy_violation(reason: impl Into<String>) -> Self {
        Self {
            code: 1008,
            reason: reason.into(),
        }
    }

    /// Message too big (1009)
    pub fn too_big() -> Self {
        Self {
            code: 1009,
            reason: String::new(),
        }
    }

    /// Internal server error (1011)
    pub fn internal_error(reason: impl Into<String>) -> Self {
        Self {
            code: 1011,
            reason: reason.into(),
        }
    }
}

// ============================================================================
// WebSocket Connection
// ============================================================================

/// Active WebSocket connection
///
/// Represents an established WebSocket connection with methods for
/// sending and receiving messages. Uses Tokio for async I/O.
///
/// # Example
///
/// ```rust,no_run
/// use data_bridge_api::websocket::{WebSocket, WebSocketMessage};
///
/// # async fn example(ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
/// // Send a text message
/// ws.send_text("Hello, WebSocket!").await?;
///
/// // Receive a message
/// if let Some(msg) = ws.receive().await? {
///     match msg {
///         WebSocketMessage::Text(text) => println!("Received: {}", text),
///         WebSocketMessage::Binary(data) => println!("Received {} bytes", data.len()),
///         _ => {}
///     }
/// }
///
/// // Close the connection
/// ws.close(None).await?;
/// # Ok(())
/// # }
/// ```
pub struct WebSocket {
    stream: WebSocketStream<TcpStream>,
}

impl WebSocket {
    /// Accept a WebSocket connection from a TCP stream
    ///
    /// Performs the WebSocket handshake and returns a WebSocket instance.
    ///
    /// # Arguments
    /// * `stream` - The TCP stream from an accepted connection
    ///
    /// # Returns
    /// A WebSocket instance ready for communication
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use data_bridge_api::websocket::WebSocket;
    /// use tokio::net::TcpListener;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let listener = TcpListener::bind("127.0.0.1:8080").await?;
    /// let (stream, _) = listener.accept().await?;
    /// let ws = WebSocket::accept(stream).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn accept(stream: TcpStream) -> ApiResult<Self> {
        debug!("Accepting WebSocket connection");
        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| ApiError::Internal(format!("WebSocket handshake failed: {}", e)))?;

        debug!("WebSocket connection established");
        Ok(Self { stream: ws_stream })
    }

    /// Send a text message
    ///
    /// Sends a UTF-8 encoded text message over the WebSocket.
    ///
    /// # Arguments
    /// * `text` - The text message to send
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_bridge_api::websocket::WebSocket;
    /// # async fn example(ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    /// ws.send_text("Hello, World!").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_text(&mut self, text: impl Into<String>) -> ApiResult<()> {
        let text_string = text.into();
        let msg = TungsteniteMessage::Text(Utf8Bytes::from(text_string));
        self.stream
            .send(msg)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to send text message: {}", e)))
    }

    /// Receive a text message
    ///
    /// Waits for and receives a text message. Returns None if the connection
    /// is closed or if a non-text message is received.
    ///
    /// # Returns
    /// The text message, or None if connection closed or non-text message
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_bridge_api::websocket::WebSocket;
    /// # async fn example(mut ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(text) = ws.receive_text().await? {
    ///     println!("Received: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn receive_text(&mut self) -> ApiResult<Option<String>> {
        match self.receive().await? {
            Some(WebSocketMessage::Text(text)) => Ok(Some(text)),
            Some(_) => Ok(None), // Non-text message
            None => Ok(None),    // Connection closed
        }
    }

    /// Send a binary message
    ///
    /// Sends raw binary data over the WebSocket.
    ///
    /// # Arguments
    /// * `data` - The binary data to send
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_bridge_api::websocket::WebSocket;
    /// # async fn example(mut ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    /// let data = vec![1, 2, 3, 4, 5];
    /// ws.send_binary(data).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_binary(&mut self, data: Vec<u8>) -> ApiResult<()> {
        let msg = TungsteniteMessage::Binary(Bytes::from(data));
        self.stream
            .send(msg)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to send binary message: {}", e)))
    }

    /// Receive a binary message
    ///
    /// Waits for and receives a binary message. Returns None if the connection
    /// is closed or if a non-binary message is received.
    ///
    /// # Returns
    /// The binary data, or None if connection closed or non-binary message
    pub async fn receive_binary(&mut self) -> ApiResult<Option<Vec<u8>>> {
        match self.receive().await? {
            Some(WebSocketMessage::Binary(data)) => Ok(Some(data)),
            Some(_) => Ok(None), // Non-binary message
            None => Ok(None),    // Connection closed
        }
    }

    /// Send a JSON message
    ///
    /// Serializes a value to JSON and sends it as a text message.
    /// Uses sonic-rs for 3-7x faster JSON serialization.
    ///
    /// # Arguments
    /// * `value` - The value to serialize and send (must implement Serialize)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_bridge_api::websocket::WebSocket;
    /// # use serde::Serialize;
    /// # #[derive(Serialize)]
    /// # struct Message { text: String }
    /// # async fn example(mut ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    /// let msg = Message { text: "Hello".to_string() };
    /// ws.send_json(&msg).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_json<T: serde::Serialize>(&mut self, value: &T) -> ApiResult<()> {
        let json = sonic_rs::to_string(value)
            .map_err(|e| ApiError::Serialization(format!("JSON serialization failed: {}", e)))?;
        self.send_text(json).await
    }

    /// Receive and parse a JSON message
    ///
    /// Receives a text message and parses it as JSON.
    /// Uses sonic-rs for 3-7x faster JSON parsing.
    ///
    /// # Returns
    /// The deserialized value, or None if connection closed or parse error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_bridge_api::websocket::WebSocket;
    /// # use serde::Deserialize;
    /// # #[derive(Deserialize)]
    /// # struct Message { text: String }
    /// # async fn example(mut ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(msg) = ws.receive_json::<Message>().await? {
    ///     println!("Received: {}", msg.text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn receive_json<T: serde::de::DeserializeOwned>(&mut self) -> ApiResult<Option<T>> {
        match self.receive_text().await? {
            Some(text) => {
                let value = sonic_rs::from_str(&text).map_err(|e| {
                    ApiError::Serialization(format!("JSON deserialization failed: {}", e))
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Send a WebSocket message
    ///
    /// Sends any type of WebSocket message (text, binary, ping, pong, close).
    ///
    /// # Arguments
    /// * `msg` - The message to send
    pub async fn send(&mut self, msg: WebSocketMessage) -> ApiResult<()> {
        let tungstenite_msg = msg.to_tungstenite();
        self.stream
            .send(tungstenite_msg)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to send message: {}", e)))
    }

    /// Receive a WebSocket message
    ///
    /// Waits for and receives the next WebSocket message. Returns None if
    /// the connection is closed.
    ///
    /// # Returns
    /// The received message, or None if connection closed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_bridge_api::websocket::{WebSocket, WebSocketMessage};
    /// # async fn example(mut ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    /// while let Some(msg) = ws.receive().await? {
    ///     match msg {
    ///         WebSocketMessage::Text(text) => println!("Text: {}", text),
    ///         WebSocketMessage::Binary(data) => println!("Binary: {} bytes", data.len()),
    ///         WebSocketMessage::Close(_) => break,
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn receive(&mut self) -> ApiResult<Option<WebSocketMessage>> {
        loop {
            match self.stream.next().await {
                Some(Ok(msg)) => {
                    if let Some(ws_msg) = WebSocketMessage::from_tungstenite(msg) {
                        if ws_msg.is_close() {
                            debug!("WebSocket close message received");
                        }
                        return Ok(Some(ws_msg));
                    }
                    // Raw frame, skip it and continue loop
                }
                Some(Err(e)) => {
                    error!("WebSocket receive error: {}", e);
                    return Err(ApiError::Internal(format!(
                        "Failed to receive message: {}",
                        e
                    )));
                }
                None => {
                    debug!("WebSocket connection closed");
                    return Ok(None);
                }
            }
        }
    }

    /// Send a ping message
    ///
    /// Sends a ping frame with optional payload. The remote endpoint should
    /// respond with a pong frame.
    ///
    /// # Arguments
    /// * `payload` - Optional ping payload (max 125 bytes)
    pub async fn ping(&mut self, payload: Vec<u8>) -> ApiResult<()> {
        self.send(WebSocketMessage::Ping(payload)).await
    }

    /// Send a pong message
    ///
    /// Sends a pong frame, typically in response to a ping.
    ///
    /// # Arguments
    /// * `payload` - Optional pong payload (should match ping payload)
    pub async fn pong(&mut self, payload: Vec<u8>) -> ApiResult<()> {
        self.send(WebSocketMessage::Pong(payload)).await
    }

    /// Close the WebSocket connection
    ///
    /// Sends a close frame and gracefully closes the connection.
    ///
    /// # Arguments
    /// * `frame` - Optional close frame with code and reason
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use data_bridge_api::websocket::{WebSocket, CloseFrame};
    /// # async fn example(mut ws: WebSocket) -> Result<(), Box<dyn std::error::Error>> {
    /// // Normal closure
    /// ws.close(None).await?;
    ///
    /// // Close with reason
    /// ws.close(Some(CloseFrame::going_away("Server shutting down"))).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(&mut self, frame: Option<CloseFrame>) -> ApiResult<()> {
        debug!("Closing WebSocket connection");
        let close_msg = WebSocketMessage::Close(frame);
        self.send(close_msg).await?;

        // Close the underlying stream
        self.stream
            .close(None)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to close WebSocket: {}", e)))?;

        debug!("WebSocket connection closed");
        Ok(())
    }
}

// ============================================================================
// WebSocket Upgrade
// ============================================================================

/// Check if a request is a WebSocket upgrade request
///
/// Validates that the request has the required headers for a WebSocket upgrade:
/// - HTTP method is GET
/// - Upgrade header is "websocket" (case-insensitive)
/// - Connection header contains "upgrade" (case-insensitive)
/// - Sec-WebSocket-Key header is present
/// - Sec-WebSocket-Version is "13"
///
/// # Arguments
/// * `req` - The HTTP request to check
///
/// # Returns
/// `true` if the request is a valid WebSocket upgrade request
///
/// # Example
///
/// ```rust
/// use data_bridge_api::request::{SerializableRequest, HttpMethod};
/// use data_bridge_api::websocket::is_websocket_upgrade;
///
/// let req = SerializableRequest::new(HttpMethod::Get, "/ws")
///     .with_header("upgrade", "websocket")
///     .with_header("connection", "upgrade")
///     .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
///     .with_header("sec-websocket-version", "13");
///
/// assert!(is_websocket_upgrade(&req));
/// ```
pub fn is_websocket_upgrade(req: &SerializableRequest) -> bool {
    use crate::request::HttpMethod;

    // Must be GET request
    if req.method != HttpMethod::Get {
        debug!("Not a WebSocket upgrade: method is not GET");
        return false;
    }

    // Check Upgrade header
    let upgrade = req.header("upgrade");
    if upgrade.map(|v| v.to_lowercase()) != Some("websocket".to_string()) {
        debug!("Not a WebSocket upgrade: missing or invalid Upgrade header");
        return false;
    }

    // Check Connection header contains "upgrade"
    let connection = req.header("connection");
    if !connection
        .map(|v| v.to_lowercase().contains("upgrade"))
        .unwrap_or(false)
    {
        debug!("Not a WebSocket upgrade: missing or invalid Connection header");
        return false;
    }

    // Check Sec-WebSocket-Key
    if req.header("sec-websocket-key").is_none() {
        debug!("Not a WebSocket upgrade: missing Sec-WebSocket-Key header");
        return false;
    }

    // Check Sec-WebSocket-Version
    if req.header("sec-websocket-version") != Some("13") {
        warn!(
            "WebSocket version mismatch: expected 13, got {:?}",
            req.header("sec-websocket-version")
        );
        return false;
    }

    true
}

/// Generate the Sec-WebSocket-Accept header value
///
/// This is used during the WebSocket handshake to prove that the server
/// understands the WebSocket protocol. It's computed as:
/// `base64(sha1(Sec-WebSocket-Key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))`
///
/// # Arguments
/// * `key` - The Sec-WebSocket-Key from the client request
///
/// # Returns
/// The computed Sec-WebSocket-Accept value
///
/// # Example
///
/// ```rust
/// use data_bridge_api::websocket::generate_accept_key;
///
/// let client_key = "dGhlIHNhbXBsZSBub25jZQ==";
/// let accept_key = generate_accept_key(client_key);
/// assert_eq!(accept_key, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
/// ```
pub fn generate_accept_key(key: &str) -> String {
    use base64::Engine;
    use sha1::{Digest, Sha1};

    const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    let hash = hasher.finalize();

    base64::engine::general_purpose::STANDARD.encode(hash)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::HttpMethod;

    #[test]
    fn test_websocket_message_is_text() {
        let msg = WebSocketMessage::Text("hello".to_string());
        assert!(msg.is_text());
        assert!(!msg.is_binary());
        assert_eq!(msg.as_text(), Some("hello"));
    }

    #[test]
    fn test_websocket_message_is_binary() {
        let msg = WebSocketMessage::Binary(vec![1, 2, 3]);
        assert!(msg.is_binary());
        assert!(!msg.is_text());
        assert_eq!(msg.as_binary(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn test_websocket_message_is_ping() {
        let msg = WebSocketMessage::Ping(vec![]);
        assert!(msg.is_ping());
        assert!(!msg.is_pong());
    }

    #[test]
    fn test_websocket_message_is_pong() {
        let msg = WebSocketMessage::Pong(vec![]);
        assert!(msg.is_pong());
        assert!(!msg.is_ping());
    }

    #[test]
    fn test_websocket_message_is_close() {
        let msg = WebSocketMessage::Close(None);
        assert!(msg.is_close());
        assert!(!msg.is_text());
    }

    #[test]
    fn test_close_frame_normal() {
        let frame = CloseFrame::normal();
        assert_eq!(frame.code, 1000);
        assert_eq!(frame.reason, "");
    }

    #[test]
    fn test_close_frame_going_away() {
        let frame = CloseFrame::going_away("Server restart");
        assert_eq!(frame.code, 1001);
        assert_eq!(frame.reason, "Server restart");
    }

    #[test]
    fn test_close_frame_protocol_error() {
        let frame = CloseFrame::protocol_error("Invalid frame");
        assert_eq!(frame.code, 1002);
        assert_eq!(frame.reason, "Invalid frame");
    }

    #[test]
    fn test_close_frame_internal_error() {
        let frame = CloseFrame::internal_error("Database error");
        assert_eq!(frame.code, 1011);
        assert_eq!(frame.reason, "Database error");
    }

    #[test]
    fn test_is_websocket_upgrade_valid() {
        let req = SerializableRequest::new(HttpMethod::Get, "/ws")
            .with_header("upgrade", "websocket")
            .with_header("connection", "upgrade")
            .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .with_header("sec-websocket-version", "13");

        assert!(is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_case_insensitive() {
        let req = SerializableRequest::new(HttpMethod::Get, "/ws")
            .with_header("Upgrade", "WebSocket")
            .with_header("Connection", "Upgrade")
            .with_header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .with_header("Sec-WebSocket-Version", "13");

        assert!(is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_wrong_method() {
        let req = SerializableRequest::new(HttpMethod::Post, "/ws")
            .with_header("upgrade", "websocket")
            .with_header("connection", "upgrade")
            .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .with_header("sec-websocket-version", "13");

        assert!(!is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_missing_upgrade_header() {
        let req = SerializableRequest::new(HttpMethod::Get, "/ws")
            .with_header("connection", "upgrade")
            .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .with_header("sec-websocket-version", "13");

        assert!(!is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_missing_connection_header() {
        let req = SerializableRequest::new(HttpMethod::Get, "/ws")
            .with_header("upgrade", "websocket")
            .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .with_header("sec-websocket-version", "13");

        assert!(!is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_missing_key() {
        let req = SerializableRequest::new(HttpMethod::Get, "/ws")
            .with_header("upgrade", "websocket")
            .with_header("connection", "upgrade")
            .with_header("sec-websocket-version", "13");

        assert!(!is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_wrong_version() {
        let req = SerializableRequest::new(HttpMethod::Get, "/ws")
            .with_header("upgrade", "websocket")
            .with_header("connection", "upgrade")
            .with_header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .with_header("sec-websocket-version", "12");

        assert!(!is_websocket_upgrade(&req));
    }

    #[test]
    fn test_generate_accept_key() {
        // Test vector from RFC 6455
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let expected = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
        assert_eq!(generate_accept_key(key), expected);
    }

    #[test]
    fn test_generate_accept_key_different_input() {
        let key1 = "x3JJHMbDL1EzLkh9GBhXDw==";
        let key2 = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept1 = generate_accept_key(key1);
        let accept2 = generate_accept_key(key2);
        assert_ne!(accept1, accept2);
    }

    #[test]
    fn test_websocket_message_conversion() {
        // Test Text conversion
        let text_msg = WebSocketMessage::Text("hello".to_string());
        let tungstenite_msg = text_msg.to_tungstenite();
        let converted = WebSocketMessage::from_tungstenite(tungstenite_msg).unwrap();
        assert_eq!(converted, text_msg);

        // Test Binary conversion
        let binary_msg = WebSocketMessage::Binary(vec![1, 2, 3]);
        let tungstenite_msg = binary_msg.to_tungstenite();
        let converted = WebSocketMessage::from_tungstenite(tungstenite_msg).unwrap();
        assert_eq!(converted, binary_msg);

        // Test Close conversion
        let close_msg = WebSocketMessage::Close(Some(CloseFrame::normal()));
        let tungstenite_msg = close_msg.to_tungstenite();
        let converted = WebSocketMessage::from_tungstenite(tungstenite_msg).unwrap();
        assert!(converted.is_close());
    }

    #[test]
    fn test_close_frame_equality() {
        let frame1 = CloseFrame::new(1000, "Normal");
        let frame2 = CloseFrame::new(1000, "Normal");
        let frame3 = CloseFrame::new(1001, "Going away");

        assert_eq!(frame1, frame2);
        assert_ne!(frame1, frame3);
    }
}
