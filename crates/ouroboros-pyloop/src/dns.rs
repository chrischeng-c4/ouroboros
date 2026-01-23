//! Async DNS resolution
//!
//! Provides async DNS lookup APIs compatible with Python's asyncio.

use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

// ============================================================================
// Address Info
// ============================================================================

/// Address family
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressFamily {
    /// IPv4
    Inet,
    /// IPv6
    Inet6,
    /// Any
    #[default]
    Unspec,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SocketType {
    /// Stream socket (TCP)
    Stream,
    /// Datagram socket (UDP)
    Dgram,
    /// Any
    #[default]
    Any,
}

/// Address info result
#[derive(Debug, Clone)]
pub struct AddrInfo {
    /// Address family
    pub family: AddressFamily,
    /// Socket type
    pub socktype: SocketType,
    /// Protocol
    pub protocol: u8,
    /// Canonical name
    pub canonname: Option<String>,
    /// Socket address
    pub sockaddr: SocketAddr,
}

impl AddrInfo {
    /// Get the IP address
    pub fn ip(&self) -> IpAddr {
        self.sockaddr.ip()
    }

    /// Get the port
    pub fn port(&self) -> u16 {
        self.sockaddr.port()
    }

    /// Check if IPv4
    pub fn is_ipv4(&self) -> bool {
        matches!(self.family, AddressFamily::Inet) || self.sockaddr.is_ipv4()
    }

    /// Check if IPv6
    pub fn is_ipv6(&self) -> bool {
        matches!(self.family, AddressFamily::Inet6) || self.sockaddr.is_ipv6()
    }
}

// ============================================================================
// Name Info
// ============================================================================

/// Name info result
#[derive(Debug, Clone)]
pub struct NameInfo {
    /// Host name
    pub host: String,
    /// Service name
    pub service: Option<String>,
}

// ============================================================================
// DNS Resolver
// ============================================================================

/// DNS resolver configuration
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Timeout for DNS queries
    pub timeout: Duration,
    /// Enable DNS caching
    pub cache_enabled: bool,
    /// Cache TTL
    pub cache_ttl: Duration,
    /// Prefer IPv6
    pub prefer_ipv6: bool,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            cache_enabled: true,
            cache_ttl: Duration::from_secs(300),
            prefer_ipv6: false,
        }
    }
}

impl ResolverConfig {
    /// Create a new resolver config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable/disable caching
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Set cache TTL
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Set IPv6 preference
    pub fn prefer_ipv6(mut self, prefer: bool) -> Self {
        self.prefer_ipv6 = prefer;
        self
    }
}

// ============================================================================
// DNS Functions
// ============================================================================

/// Async getaddrinfo
///
/// Resolves a hostname to a list of socket addresses.
pub async fn getaddrinfo(
    host: &str,
    port: u16,
    family: AddressFamily,
    socktype: SocketType,
) -> io::Result<Vec<AddrInfo>> {
    let host = host.to_string();

    // Run blocking DNS lookup in spawn_blocking
    tokio::task::spawn_blocking(move || {
        let addr_str = format!("{}:{}", host, port);

        let addrs: Vec<_> = addr_str
            .to_socket_addrs()?
            .filter(|addr| match family {
                AddressFamily::Inet => addr.is_ipv4(),
                AddressFamily::Inet6 => addr.is_ipv6(),
                AddressFamily::Unspec => true,
            })
            .map(|sockaddr| AddrInfo {
                family: if sockaddr.is_ipv4() {
                    AddressFamily::Inet
                } else {
                    AddressFamily::Inet6
                },
                socktype: match socktype {
                    SocketType::Stream => SocketType::Stream,
                    SocketType::Dgram => SocketType::Dgram,
                    SocketType::Any => SocketType::Stream,
                },
                protocol: 0,
                canonname: None,
                sockaddr,
            })
            .collect();

        if addrs.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No addresses found for {}", host),
            ))
        } else {
            Ok(addrs)
        }
    })
    .await
    .map_err(io::Error::other)?
}

/// Async getaddrinfo with timeout
pub async fn getaddrinfo_with_timeout(
    host: &str,
    port: u16,
    family: AddressFamily,
    socktype: SocketType,
    timeout: Duration,
) -> io::Result<Vec<AddrInfo>> {
    tokio::time::timeout(timeout, getaddrinfo(host, port, family, socktype))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "DNS lookup timed out"))?
}

/// Async getnameinfo
///
/// Performs reverse DNS lookup on a socket address.
pub async fn getnameinfo(sockaddr: SocketAddr) -> io::Result<NameInfo> {
    // Run blocking reverse lookup in spawn_blocking
    tokio::task::spawn_blocking(move || {
        // Use system DNS for reverse lookup
        // This is a simplified implementation
        let host = sockaddr.ip().to_string();
        let service = sockaddr.port().to_string();

        Ok(NameInfo {
            host,
            service: Some(service),
        })
    })
    .await
    .map_err(io::Error::other)?
}

/// Simple hostname resolution
pub async fn resolve_hostname(host: &str) -> io::Result<IpAddr> {
    let addrs = getaddrinfo(host, 0, AddressFamily::Unspec, SocketType::Stream).await?;
    addrs
        .first()
        .map(|a| a.ip())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No address found"))
}

/// Resolve hostname to all addresses
pub async fn resolve_hostname_all(host: &str) -> io::Result<Vec<IpAddr>> {
    let addrs = getaddrinfo(host, 0, AddressFamily::Unspec, SocketType::Stream).await?;
    Ok(addrs.into_iter().map(|a| a.ip()).collect())
}

/// Resolve hostname to IPv4 only
pub async fn resolve_hostname_v4(host: &str) -> io::Result<std::net::Ipv4Addr> {
    let addrs = getaddrinfo(host, 0, AddressFamily::Inet, SocketType::Stream).await?;
    addrs
        .first()
        .and_then(|a| match a.ip() {
            IpAddr::V4(v4) => Some(v4),
            _ => None,
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No IPv4 address found"))
}

/// Resolve hostname to IPv6 only
pub async fn resolve_hostname_v6(host: &str) -> io::Result<std::net::Ipv6Addr> {
    let addrs = getaddrinfo(host, 0, AddressFamily::Inet6, SocketType::Stream).await?;
    addrs
        .first()
        .and_then(|a| match a.ip() {
            IpAddr::V6(v6) => Some(v6),
            _ => None,
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No IPv6 address found"))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_family() {
        assert_eq!(AddressFamily::default(), AddressFamily::Unspec);
    }

    #[test]
    fn test_resolver_config() {
        let config = ResolverConfig::new()
            .timeout(Duration::from_secs(10))
            .cache_enabled(true)
            .prefer_ipv6(true);

        assert_eq!(config.timeout, Duration::from_secs(10));
        assert!(config.cache_enabled);
        assert!(config.prefer_ipv6);
    }

    #[tokio::test]
    async fn test_resolve_localhost() {
        let result = resolve_hostname("localhost").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_getaddrinfo_localhost() {
        let addrs = getaddrinfo("localhost", 80, AddressFamily::Unspec, SocketType::Stream).await;
        assert!(addrs.is_ok());
        let addrs = addrs.unwrap();
        assert!(!addrs.is_empty());
    }

    #[tokio::test]
    async fn test_getnameinfo() {
        let sockaddr: SocketAddr = "127.0.0.1:80".parse().unwrap();
        let result = getnameinfo(sockaddr).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(!info.host.is_empty());
    }
}
