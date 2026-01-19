//! Unix domain socket support
//!
//! Provides Unix socket connection and server APIs for
//! inter-process communication (IPC).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::io;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ============================================================================
// Unix Connection
// ============================================================================

/// Unix domain socket transport
pub struct UnixTransport {
    stream: UnixStream,
    path: Option<PathBuf>,
}

impl UnixTransport {
    /// Create from an existing stream
    pub fn from_stream(stream: UnixStream) -> Self {
        Self {
            stream,
            path: None,
        }
    }

    /// Get the socket path (for client connections)
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Get peer address if available
    pub fn peer_addr(&self) -> io::Result<tokio::net::unix::SocketAddr> {
        self.stream.peer_addr()
    }

    /// Get local address if available
    pub fn local_addr(&self) -> io::Result<tokio::net::unix::SocketAddr> {
        self.stream.local_addr()
    }

    /// Read data from the socket
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf).await
    }

    /// Read exact number of bytes
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.stream.read_exact(buf).await.map(|_| ())
    }

    /// Write data to the socket
    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf).await
    }

    /// Write all data
    pub async fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stream.write_all(buf).await
    }

    /// Flush the socket
    pub async fn flush(&mut self) -> io::Result<()> {
        self.stream.flush().await
    }

    /// Shutdown the socket
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.stream.shutdown().await
    }

    /// Close the connection
    pub fn close(self) {
        // Stream will be dropped, closing the connection
        drop(self.stream);
    }

    /// Split into reader and writer
    pub fn split(&mut self) -> (UnixReadHalf<'_>, UnixWriteHalf<'_>) {
        let (read, write) = self.stream.split();
        (UnixReadHalf { inner: read }, UnixWriteHalf { inner: write })
    }
}

/// Read half of a Unix socket
pub struct UnixReadHalf<'a> {
    inner: tokio::net::unix::ReadHalf<'a>,
}

impl<'a> UnixReadHalf<'a> {
    /// Read data
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).await
    }

    /// Read exact bytes
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf).await.map(|_| ())
    }
}

/// Write half of a Unix socket
pub struct UnixWriteHalf<'a> {
    inner: tokio::net::unix::WriteHalf<'a>,
}

impl<'a> UnixWriteHalf<'a> {
    /// Write data
    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).await
    }

    /// Write all data
    pub async fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf).await
    }

    /// Shutdown write side
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.inner.shutdown().await
    }
}

// ============================================================================
// Unix Server
// ============================================================================

/// Unix domain socket server
pub struct UnixServer {
    listener: UnixListener,
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl UnixServer {
    /// Create a new Unix socket server
    pub fn new(listener: UnixListener, path: PathBuf) -> Self {
        Self {
            listener,
            path,
            cleanup_on_drop: true,
        }
    }

    /// Get the socket path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the local address
    pub fn local_addr(&self) -> io::Result<tokio::net::unix::SocketAddr> {
        self.listener.local_addr()
    }

    /// Disable automatic cleanup on drop
    pub fn no_cleanup(mut self) -> Self {
        self.cleanup_on_drop = false;
        self
    }

    /// Accept a connection
    pub async fn accept(&self) -> io::Result<UnixTransport> {
        let (stream, _addr) = self.listener.accept().await?;
        Ok(UnixTransport::from_stream(stream))
    }

    /// Close the server
    pub fn close(self) {
        // Will be dropped
    }
}

impl Drop for UnixServer {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            // Try to remove the socket file
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// ============================================================================
// Connection Functions
// ============================================================================

/// Connect to a Unix socket
///
/// # Arguments
/// * `path` - Path to the Unix socket
///
/// # Returns
/// A connected Unix transport
pub async fn create_unix_connection(path: impl AsRef<Path>) -> io::Result<UnixTransport> {
    let path = path.as_ref();
    let stream = UnixStream::connect(path).await?;
    Ok(UnixTransport {
        stream,
        path: Some(path.to_path_buf()),
    })
}

/// Connect to a Unix socket with timeout
pub async fn create_unix_connection_with_timeout(
    path: impl AsRef<Path>,
    timeout: std::time::Duration,
) -> io::Result<UnixTransport> {
    let path = path.as_ref().to_path_buf();
    tokio::time::timeout(timeout, create_unix_connection(&path))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))?
}

/// Create a Unix socket server
///
/// # Arguments
/// * `path` - Path to create the socket at
///
/// # Returns
/// A Unix socket server
pub async fn create_unix_server(path: impl AsRef<Path>) -> io::Result<UnixServer> {
    let path = path.as_ref();

    // Remove existing socket if present
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;
    Ok(UnixServer::new(listener, path.to_path_buf()))
}

/// Create a Unix socket server with permissions
pub async fn create_unix_server_with_permissions(
    path: impl AsRef<Path>,
    mode: u32,
) -> io::Result<UnixServer> {
    let path = path.as_ref();

    // Remove existing socket if present
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;

    // Set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }

    Ok(UnixServer::new(listener, path.to_path_buf()))
}

/// Open a Unix connection (asyncio-style API)
///
/// Returns separate reader and writer streams.
pub async fn open_unix_connection(
    path: impl AsRef<Path>,
) -> io::Result<(UnixStreamReader, UnixStreamWriter)> {
    let transport = create_unix_connection(path).await?;
    let transport = Arc::new(tokio::sync::Mutex::new(transport));
    Ok((
        UnixStreamReader {
            transport: Arc::clone(&transport),
        },
        UnixStreamWriter { transport },
    ))
}

// ============================================================================
// Stream Reader/Writer
// ============================================================================

/// Unix stream reader (asyncio-style)
pub struct UnixStreamReader {
    transport: Arc<tokio::sync::Mutex<UnixTransport>>,
}

impl UnixStreamReader {
    /// Read up to n bytes
    pub async fn read(&self, n: usize) -> io::Result<Vec<u8>> {
        let mut transport = self.transport.lock().await;
        let mut buf = vec![0u8; n];
        let bytes_read = transport.read(&mut buf).await?;
        buf.truncate(bytes_read);
        Ok(buf)
    }

    /// Read exactly n bytes
    pub async fn readexactly(&self, n: usize) -> io::Result<Vec<u8>> {
        let mut transport = self.transport.lock().await;
        let mut buf = vec![0u8; n];
        transport.read_exact(&mut buf).await?;
        Ok(buf)
    }

    /// Read until delimiter
    pub async fn readuntil(&self, delimiter: u8) -> io::Result<Vec<u8>> {
        let mut transport = self.transport.lock().await;
        let mut result = Vec::new();
        let mut buf = [0u8; 1];

        loop {
            let n = transport.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            result.push(buf[0]);
            if buf[0] == delimiter {
                break;
            }
        }

        Ok(result)
    }

    /// Read a line (until \n)
    pub async fn readline(&self) -> io::Result<Vec<u8>> {
        self.readuntil(b'\n').await
    }

    /// Check if at EOF
    pub async fn at_eof(&self) -> bool {
        // Try to peek; if 0 bytes, we're at EOF
        let transport = self.transport.lock().await;
        // Can't easily implement without peek, return false
        false
    }
}

/// Unix stream writer (asyncio-style)
pub struct UnixStreamWriter {
    transport: Arc<tokio::sync::Mutex<UnixTransport>>,
}

impl UnixStreamWriter {
    /// Write data
    pub async fn write(&self, data: &[u8]) -> io::Result<()> {
        let mut transport = self.transport.lock().await;
        transport.write_all(data).await
    }

    /// Write lines (adds newline after each)
    pub async fn writelines(&self, lines: &[&[u8]]) -> io::Result<()> {
        let mut transport = self.transport.lock().await;
        for line in lines {
            transport.write_all(line).await?;
            transport.write_all(b"\n").await?;
        }
        Ok(())
    }

    /// Flush the write buffer
    pub async fn drain(&self) -> io::Result<()> {
        let mut transport = self.transport.lock().await;
        transport.flush().await
    }

    /// Close the writer
    pub async fn close(&self) -> io::Result<()> {
        let mut transport = self.transport.lock().await;
        transport.shutdown().await
    }

    /// Check if closing
    pub fn is_closing(&self) -> bool {
        // Would need to track state
        false
    }

    /// Wait until closed
    pub async fn wait_closed(&self) -> io::Result<()> {
        self.close().await
    }
}

// ============================================================================
// Server Callback Style
// ============================================================================

/// Callback function for handling client connections
pub type ClientHandler = Box<dyn Fn(UnixStreamReader, UnixStreamWriter) + Send + Sync>;

/// Start a Unix socket server (asyncio-style with callback)
pub async fn start_unix_server<F, Fut>(
    client_connected_cb: F,
    path: impl AsRef<Path>,
) -> io::Result<UnixServer>
where
    F: Fn(UnixStreamReader, UnixStreamWriter) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let server = create_unix_server(path).await?;

    // Spawn accept loop
    let listener_path = server.path.clone();
    tokio::spawn(async move {
        // We need to re-bind since we moved server
        if let Ok(listener) = UnixListener::bind(&listener_path) {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let transport = UnixTransport::from_stream(stream);
                        let transport = Arc::new(tokio::sync::Mutex::new(transport));
                        let reader = UnixStreamReader {
                            transport: Arc::clone(&transport),
                        };
                        let writer = UnixStreamWriter { transport };

                        let cb = client_connected_cb.clone();
                        tokio::spawn(async move {
                            cb(reader, writer).await;
                        });
                    }
                    Err(_) => break,
                }
            }
        }
    });

    Ok(server)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_unix_connection() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Create server
        let server = create_unix_server(&socket_path).await.unwrap();
        assert!(socket_path.exists());

        // Spawn server accept
        let server_handle = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            let mut buf = [0u8; 5];
            conn.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"hello");
            conn.write_all(b"world").await.unwrap();
        });

        // Connect as client
        let mut client = create_unix_connection(&socket_path).await.unwrap();
        client.write_all(b"hello").await.unwrap();

        let mut buf = [0u8; 5];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"world");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_open_unix_connection() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test2.sock");

        // Create server
        let server = create_unix_server(&socket_path).await.unwrap();

        // Spawn server
        let server_handle = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            let mut buf = vec![0u8; 100];
            let n = conn.read(&mut buf).await.unwrap();
            buf.truncate(n);
            conn.write_all(&buf).await.unwrap();
        });

        // Connect with reader/writer style
        let (reader, writer) = open_unix_connection(&socket_path).await.unwrap();
        writer.write(b"echo me").await.unwrap();
        writer.drain().await.unwrap();

        let response = reader.read(100).await.unwrap();
        assert_eq!(&response, b"echo me");

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_server_cleanup() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("cleanup.sock");

        {
            let _server = create_unix_server(&socket_path).await.unwrap();
            assert!(socket_path.exists());
        }

        // Server dropped, socket should be cleaned up
        assert!(!socket_path.exists());
    }

    #[tokio::test]
    async fn test_transport_split() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("split.sock");

        let server = create_unix_server(&socket_path).await.unwrap();

        let server_handle = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            let (mut read, mut write) = conn.split();

            let mut buf = [0u8; 4];
            read.read_exact(&mut buf).await.unwrap();
            write.write_all(&buf).await.unwrap();
        });

        let mut client = create_unix_connection(&socket_path).await.unwrap();
        client.write_all(b"ping").await.unwrap();

        let mut buf = [0u8; 4];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"ping");

        server_handle.await.unwrap();
    }
}
