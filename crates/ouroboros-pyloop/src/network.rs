//! Network I/O primitives for ouroboros-pyloop
//!
//! This module provides TCP/UDP network primitives that integrate with
//! the Tokio runtime used by PyLoop.
//!
//! # Features
//!
//! - TCP client connections via `Transport` and `Protocol`
//! - TCP server via `TcpServer`
//! - Async read/write operations
//! - Connection lifecycle management
//! - asyncio-compatible API
//!
//! # Example
//!
//! ```python
//! import asyncio
//! from ouroboros._pyloop import PyLoop
//!
//! async def tcp_client():
//!     loop = asyncio.get_running_loop()
//!     transport, protocol = await loop.create_connection(
//!         lambda: MyProtocol(),
//!         '127.0.0.1', 8888
//!     )
//!     transport.write(b'Hello')
//! ```

use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

// ============================================================================
// Transport - asyncio-like transport abstraction
// ============================================================================

/// TCP Transport for async I/O
///
/// Wraps a Tokio TcpStream and provides asyncio-compatible methods.
/// This is the Rust implementation that will be exposed to Python.
pub struct TcpTransport {
    /// The underlying TCP stream
    stream: Arc<Mutex<TcpStream>>,
    /// Whether the transport is closing/closed
    closing: AtomicBool,
    /// Local address
    local_addr: SocketAddr,
    /// Remote address
    peer_addr: SocketAddr,
}

impl TcpTransport {
    /// Create a new TCP transport from a stream
    pub async fn new(stream: TcpStream) -> io::Result<Self> {
        let local_addr = stream.local_addr()?;
        let peer_addr = stream.peer_addr()?;

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            closing: AtomicBool::new(false),
            local_addr,
            peer_addr,
        })
    }

    /// Get the local socket address
    pub fn get_local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get the remote socket address
    pub fn get_peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Check if the transport is closing
    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }

    /// Write data to the transport
    ///
    /// This method buffers the data and sends it asynchronously.
    /// It does not block waiting for the data to be sent.
    pub async fn write(&self, data: &[u8]) -> io::Result<()> {
        if self.closing.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Transport is closing",
            ));
        }

        let mut stream = self.stream.lock().await;
        stream.write_all(data).await
    }

    /// Write multiple buffers to the transport
    pub async fn writelines(&self, buffers: Vec<&[u8]>) -> io::Result<()> {
        for buf in buffers {
            self.write(buf).await?;
        }
        Ok(())
    }

    /// Read data from the transport
    ///
    /// Returns the number of bytes read, or 0 if EOF.
    pub async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        if self.closing.load(Ordering::Acquire) {
            return Ok(0);
        }

        let mut stream = self.stream.lock().await;
        stream.read(buf).await
    }

    /// Read exact number of bytes
    pub async fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        let mut stream = self.stream.lock().await;
        stream.read_exact(buf).await?;
        Ok(())
    }

    /// Close the transport
    ///
    /// Marks the transport as closing. Buffered data will be flushed
    /// before the underlying connection is closed.
    pub async fn close(&self) -> io::Result<()> {
        self.closing.store(true, Ordering::Release);
        let mut stream = self.stream.lock().await;
        stream.flush().await?;
        stream.shutdown().await
    }

    /// Abort the transport
    ///
    /// Closes the transport immediately without flushing buffers.
    pub fn abort(&self) {
        self.closing.store(true, Ordering::Release);
        // The stream will be dropped when all references are gone
    }

    /// Get write buffer size (placeholder - returns 0)
    pub fn get_write_buffer_size(&self) -> usize {
        0 // Tokio handles buffering internally
    }

    /// Set write buffer limits (no-op - Tokio handles this)
    pub fn set_write_buffer_limits(&self, _high: usize, _low: usize) {
        // No-op - Tokio manages buffer sizes
    }
}

impl std::fmt::Debug for TcpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpTransport")
            .field("local_addr", &self.local_addr)
            .field("peer_addr", &self.peer_addr)
            .field("closing", &self.closing.load(Ordering::Relaxed))
            .finish()
    }
}

// ============================================================================
// TCP Server
// ============================================================================

/// TCP Server for accepting connections
///
/// Wraps a Tokio TcpListener and provides methods for accepting connections.
pub struct TcpServer {
    /// The underlying TCP listener
    listener: Arc<Mutex<TcpListener>>,
    /// Whether the server is closing
    closing: AtomicBool,
    /// Local address
    local_addr: SocketAddr,
}

impl TcpServer {
    /// Create a new TCP server bound to the given address
    pub async fn bind(addr: &str, port: u16) -> io::Result<Self> {
        let socket_addr: SocketAddr = format!("{}:{}", addr, port)
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let listener = TcpListener::bind(socket_addr).await?;
        let local_addr = listener.local_addr()?;

        Ok(Self {
            listener: Arc::new(Mutex::new(listener)),
            closing: AtomicBool::new(false),
            local_addr,
        })
    }

    /// Bind to an address with custom backlog
    pub async fn bind_with_backlog(addr: &str, port: u16, _backlog: u32) -> io::Result<Self> {
        // Note: Tokio doesn't expose backlog configuration directly
        // The OS default is used instead
        Self::bind(addr, port).await
    }

    /// Get the local address this server is bound to
    pub fn get_local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Check if the server is closing
    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }

    /// Accept a new connection
    ///
    /// Returns a tuple of (transport, peer_address).
    pub async fn accept(&self) -> io::Result<(TcpTransport, SocketAddr)> {
        if self.closing.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Server is closing",
            ));
        }

        let listener = self.listener.lock().await;
        let (stream, peer_addr) = listener.accept().await?;
        let transport = TcpTransport::new(stream).await?;

        Ok((transport, peer_addr))
    }

    /// Close the server
    ///
    /// Stops accepting new connections but allows existing connections
    /// to continue.
    pub fn close(&self) {
        self.closing.store(true, Ordering::Release);
    }
}

impl std::fmt::Debug for TcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpServer")
            .field("local_addr", &self.local_addr)
            .field("closing", &self.closing.load(Ordering::Relaxed))
            .finish()
    }
}

// ============================================================================
// Connection Functions
// ============================================================================

/// Create a TCP client connection
///
/// Connects to the specified host and port.
///
/// # Arguments
/// * `host` - Hostname or IP address
/// * `port` - Port number
///
/// # Returns
/// A TcpTransport on success
pub async fn create_connection(host: &str, port: u16) -> io::Result<TcpTransport> {
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).await?;
    TcpTransport::new(stream).await
}

/// Create a TCP client connection with timeout
pub async fn create_connection_with_timeout(
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> io::Result<TcpTransport> {
    let addr = format!("{}:{}", host, port);
    let duration = std::time::Duration::from_millis(timeout_ms);

    let stream = tokio::time::timeout(duration, TcpStream::connect(&addr))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Connection timed out"))??;

    TcpTransport::new(stream).await
}

/// Create a TCP server
///
/// Creates a server that listens on the specified address and port.
///
/// # Arguments
/// * `host` - Hostname or IP address to bind to
/// * `port` - Port number (0 for auto-assign)
///
/// # Returns
/// A TcpServer on success
pub async fn create_server(host: &str, port: u16) -> io::Result<TcpServer> {
    TcpServer::bind(host, port).await
}

// ============================================================================
// Stream Reader/Writer helpers
// ============================================================================

/// Async reader for TcpTransport
///
/// Provides a stream-like interface for reading data.
pub struct StreamReader {
    transport: Arc<TcpTransport>,
    buffer: Vec<u8>,
    position: usize,
}

impl StreamReader {
    /// Create a new stream reader
    pub fn new(transport: Arc<TcpTransport>) -> Self {
        Self {
            transport,
            buffer: Vec::with_capacity(8192),
            position: 0,
        }
    }

    /// Read up to n bytes
    pub async fn read(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut result = vec![0u8; n];
        let bytes_read = self.transport.read(&mut result).await?;
        result.truncate(bytes_read);
        Ok(result)
    }

    /// Read exactly n bytes
    pub async fn readexactly(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut result = vec![0u8; n];
        self.transport.read_exact(&mut result).await?;
        Ok(result)
    }

    /// Read until EOF
    pub async fn read_all(&mut self) -> io::Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = self.transport.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            result.extend_from_slice(&buf[..n]);
        }
        Ok(result)
    }

    /// Read a line (until \n)
    pub async fn readline(&mut self) -> io::Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            let n = self.transport.read(&mut byte).await?;
            if n == 0 {
                break;
            }
            result.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        Ok(result)
    }
}

/// Async writer for TcpTransport
///
/// Provides a stream-like interface for writing data.
pub struct StreamWriter {
    transport: Arc<TcpTransport>,
}

impl StreamWriter {
    /// Create a new stream writer
    pub fn new(transport: Arc<TcpTransport>) -> Self {
        Self { transport }
    }

    /// Write data
    pub async fn write(&self, data: &[u8]) -> io::Result<()> {
        self.transport.write(data).await
    }

    /// Write multiple buffers
    pub async fn writelines(&self, lines: Vec<&[u8]>) -> io::Result<()> {
        self.transport.writelines(lines).await
    }

    /// Flush (ensures data is sent)
    pub async fn drain(&self) -> io::Result<()> {
        // Tokio flushes on write, so this is mostly a no-op
        // but we include it for API compatibility
        Ok(())
    }

    /// Close the writer
    pub async fn close(&self) -> io::Result<()> {
        self.transport.close().await
    }

    /// Wait for close to complete
    pub async fn wait_closed(&self) -> io::Result<()> {
        // Already closed in close(), so this is a no-op
        Ok(())
    }

    /// Check if closing
    pub fn is_closing(&self) -> bool {
        self.transport.is_closing()
    }
}

// ============================================================================
// asyncio-style open_connection helper
// ============================================================================

/// Open a TCP connection and return reader/writer pair
///
/// This is the asyncio-style API: `reader, writer = await asyncio.open_connection(...)`
pub async fn open_connection(host: &str, port: u16) -> io::Result<(StreamReader, StreamWriter)> {
    let transport = Arc::new(create_connection(host, port).await?);
    let reader = StreamReader::new(transport.clone());
    let writer = StreamWriter::new(transport);
    Ok((reader, writer))
}

/// Open a TCP connection with timeout
pub async fn open_connection_with_timeout(
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> io::Result<(StreamReader, StreamWriter)> {
    let transport = Arc::new(create_connection_with_timeout(host, port, timeout_ms).await?);
    let reader = StreamReader::new(transport.clone());
    let writer = StreamWriter::new(transport);
    Ok((reader, writer))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_server_bind() {
        let server = TcpServer::bind("127.0.0.1", 0).await.unwrap();
        let addr = server.get_local_addr();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert!(addr.port() > 0);
    }

    #[tokio::test]
    async fn test_tcp_server_debug() {
        let server = TcpServer::bind("127.0.0.1", 0).await.unwrap();
        let debug_str = format!("{:?}", server);
        assert!(debug_str.contains("TcpServer"));
        assert!(debug_str.contains("local_addr"));
    }

    #[tokio::test]
    async fn test_tcp_transport_connect_and_close() {
        // Start a server
        let server = TcpServer::bind("127.0.0.1", 0).await.unwrap();
        let port = server.get_local_addr().port();

        // Accept in background
        let server_task = tokio::spawn(async move {
            let (transport, _) = server.accept().await.unwrap();
            transport.close().await.unwrap();
        });

        // Connect client
        let client = create_connection("127.0.0.1", port).await.unwrap();
        assert!(!client.is_closing());

        client.close().await.unwrap();
        assert!(client.is_closing());

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_transport_write_read() {
        let server = TcpServer::bind("127.0.0.1", 0).await.unwrap();
        let port = server.get_local_addr().port();

        // Server echoes data
        let server_task = tokio::spawn(async move {
            let (transport, _) = server.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = transport.read(&mut buf).await.unwrap();
            transport.write(&buf[..n]).await.unwrap();
            transport.close().await.unwrap();
        });

        // Client sends and receives
        let client = create_connection("127.0.0.1", port).await.unwrap();
        client.write(b"hello").await.unwrap();

        let mut buf = [0u8; 1024];
        let n = client.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello");

        client.close().await.unwrap();
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_open_connection_helper() {
        let server = TcpServer::bind("127.0.0.1", 0).await.unwrap();
        let port = server.get_local_addr().port();

        let server_task = tokio::spawn(async move {
            let (transport, _) = server.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let n = transport.read(&mut buf).await.unwrap();
            transport.write(&buf[..n]).await.unwrap();
            transport.close().await.unwrap();
        });

        let (mut reader, writer) = open_connection("127.0.0.1", port).await.unwrap();
        writer.write(b"test").await.unwrap();

        let data = reader.read(4).await.unwrap();
        assert_eq!(data, b"test");

        writer.close().await.unwrap();
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        // Try to connect to a port that's not listening with a short timeout
        let result = create_connection_with_timeout("127.0.0.1", 65535, 100).await;
        assert!(result.is_err());
    }
}
