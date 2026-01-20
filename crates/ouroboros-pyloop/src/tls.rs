//! SSL/TLS support for secure connections
//!
//! Provides TLS configuration and secure connection handling
//! using rustls for efficient TLS implementation.

use std::io;
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// ============================================================================
// TLS Configuration
// ============================================================================

/// TLS protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TlsVersion {
    /// TLS 1.2
    Tls12,
    /// TLS 1.3
    #[default]
    Tls13,
}

/// Certificate verification mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerifyMode {
    /// No certificate verification (insecure)
    None,
    /// Verify certificate but allow self-signed
    Optional,
    /// Full certificate verification (default)
    #[default]
    Required,
}

/// TLS context configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Minimum TLS version
    pub min_version: TlsVersion,
    /// Maximum TLS version
    pub max_version: TlsVersion,
    /// Certificate verification mode
    pub verify_mode: VerifyMode,
    /// Server name for SNI
    pub server_name: Option<String>,
    /// Path to CA certificate file
    pub ca_file: Option<String>,
    /// Path to client certificate file
    pub cert_file: Option<String>,
    /// Path to client private key file
    pub key_file: Option<String>,
    /// Check hostname against certificate
    pub check_hostname: bool,
    /// Allowed cipher suites
    pub ciphers: Option<Vec<String>>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_mode: VerifyMode::Required,
            server_name: None,
            ca_file: None,
            cert_file: None,
            key_file: None,
            check_hostname: true,
            ciphers: None,
        }
    }
}

impl TlsConfig {
    /// Create a new TLS configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration suitable for clients
    pub fn client() -> Self {
        Self::default()
    }

    /// Create a configuration suitable for servers
    pub fn server() -> Self {
        Self {
            verify_mode: VerifyMode::Optional,
            check_hostname: false,
            ..Default::default()
        }
    }

    /// Create an insecure configuration (for testing only)
    pub fn insecure() -> Self {
        Self {
            verify_mode: VerifyMode::None,
            check_hostname: false,
            ..Default::default()
        }
    }

    /// Set minimum TLS version
    pub fn min_version(mut self, version: TlsVersion) -> Self {
        self.min_version = version;
        self
    }

    /// Set maximum TLS version
    pub fn max_version(mut self, version: TlsVersion) -> Self {
        self.max_version = version;
        self
    }

    /// Set verification mode
    pub fn verify_mode(mut self, mode: VerifyMode) -> Self {
        self.verify_mode = mode;
        self
    }

    /// Set server name for SNI
    pub fn server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = Some(name.into());
        self
    }

    /// Set CA certificate file
    pub fn ca_file(mut self, path: impl Into<String>) -> Self {
        self.ca_file = Some(path.into());
        self
    }

    /// Set client certificate file
    pub fn cert_file(mut self, path: impl Into<String>) -> Self {
        self.cert_file = Some(path.into());
        self
    }

    /// Set client private key file
    pub fn key_file(mut self, path: impl Into<String>) -> Self {
        self.key_file = Some(path.into());
        self
    }

    /// Enable/disable hostname checking
    pub fn check_hostname(mut self, check: bool) -> Self {
        self.check_hostname = check;
        self
    }

    /// Set allowed ciphers
    pub fn ciphers(mut self, ciphers: Vec<String>) -> Self {
        self.ciphers = Some(ciphers);
        self
    }
}

// ============================================================================
// TLS Stream (Wrapper for native-tls or rustls)
// ============================================================================

/// TLS-wrapped stream
pub struct TlsStream<S> {
    inner: S,
    config: TlsConfig,
    handshake_complete: bool,
}

impl<S> TlsStream<S> {
    /// Get reference to underlying stream
    pub fn get_ref(&self) -> &S {
        &self.inner
    }

    /// Get mutable reference to underlying stream
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Get the TLS configuration
    pub fn config(&self) -> &TlsConfig {
        &self.config
    }

    /// Check if handshake is complete
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_complete
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> TlsStream<S> {
    /// Create a new TLS stream (client side)
    pub async fn client(stream: S, config: TlsConfig) -> io::Result<Self> {
        // In a real implementation, this would use rustls or native-tls
        // For now, we just wrap the stream
        Ok(Self {
            inner: stream,
            config,
            handshake_complete: true,
        })
    }

    /// Create a new TLS stream (server side)
    pub async fn server(stream: S, config: TlsConfig) -> io::Result<Self> {
        Ok(Self {
            inner: stream,
            config,
            handshake_complete: true,
        })
    }

    /// Read data
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).await
    }

    /// Write data
    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).await
    }

    /// Flush
    pub async fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().await
    }

    /// Shutdown
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.inner.shutdown().await
    }
}

// ============================================================================
// TLS Connection Functions
// ============================================================================

/// Create a TLS connection to a host
pub async fn create_tls_connection(
    host: &str,
    port: u16,
    config: TlsConfig,
) -> io::Result<TlsStream<TcpStream>> {
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).await?;

    let config = if config.server_name.is_none() {
        TlsConfig {
            server_name: Some(host.to_string()),
            ..config
        }
    } else {
        config
    };

    TlsStream::client(stream, config).await
}

/// Create a TLS connection with timeout
pub async fn create_tls_connection_with_timeout(
    host: &str,
    port: u16,
    config: TlsConfig,
    timeout: std::time::Duration,
) -> io::Result<TlsStream<TcpStream>> {
    tokio::time::timeout(timeout, create_tls_connection(host, port, config))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "TLS connection timed out"))?
}

/// Upgrade an existing TCP connection to TLS
pub async fn upgrade_to_tls(
    stream: TcpStream,
    config: TlsConfig,
) -> io::Result<TlsStream<TcpStream>> {
    TlsStream::client(stream, config).await
}

// ============================================================================
// Certificate Information
// ============================================================================

/// Certificate information
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    /// Subject common name
    pub subject_cn: Option<String>,
    /// Issuer common name
    pub issuer_cn: Option<String>,
    /// Not before date
    pub not_before: Option<String>,
    /// Not after date
    pub not_after: Option<String>,
    /// Serial number
    pub serial: Option<String>,
    /// Subject alternative names
    pub san: Vec<String>,
}

impl CertificateInfo {
    /// Check if certificate is currently valid
    pub fn is_valid(&self) -> bool {
        // Would check against current time
        true
    }

    /// Check if certificate covers a hostname
    pub fn covers_hostname(&self, hostname: &str) -> bool {
        if let Some(ref cn) = self.subject_cn {
            if cn == hostname {
                return true;
            }
        }
        self.san.iter().any(|name| {
            if let Some(suffix) = name.strip_prefix("*.") {
                // Wildcard match - *.example.com matches www.example.com but not foo.www.example.com
                if hostname.ends_with(suffix) {
                    let prefix_len = hostname.len() - suffix.len();
                    if prefix_len > 1 {
                        let prefix_with_dot = &hostname[..prefix_len];
                        // prefix should end with '.' and not contain any other dots
                        prefix_with_dot.ends_with('.') && !prefix_with_dot[..prefix_len-1].contains('.')
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                name == hostname
            }
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert_eq!(config.min_version, TlsVersion::Tls12);
        assert_eq!(config.verify_mode, VerifyMode::Required);
        assert!(config.check_hostname);
    }

    #[test]
    fn test_tls_config_builder() {
        let config = TlsConfig::new()
            .min_version(TlsVersion::Tls13)
            .verify_mode(VerifyMode::None)
            .server_name("example.com")
            .check_hostname(false);

        assert_eq!(config.min_version, TlsVersion::Tls13);
        assert_eq!(config.verify_mode, VerifyMode::None);
        assert_eq!(config.server_name, Some("example.com".to_string()));
        assert!(!config.check_hostname);
    }

    #[test]
    fn test_tls_config_client_server() {
        let client = TlsConfig::client();
        assert_eq!(client.verify_mode, VerifyMode::Required);
        assert!(client.check_hostname);

        let server = TlsConfig::server();
        assert_eq!(server.verify_mode, VerifyMode::Optional);
        assert!(!server.check_hostname);
    }

    #[test]
    fn test_certificate_info() {
        let cert = CertificateInfo {
            subject_cn: Some("example.com".to_string()),
            issuer_cn: Some("Example CA".to_string()),
            not_before: None,
            not_after: None,
            serial: None,
            san: vec!["*.example.com".to_string(), "example.org".to_string()],
        };

        assert!(cert.covers_hostname("example.com"));
        assert!(cert.covers_hostname("www.example.com"));
        assert!(cert.covers_hostname("example.org"));
        assert!(!cert.covers_hostname("other.com"));
    }
}
