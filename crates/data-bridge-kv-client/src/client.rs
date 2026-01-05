//! KV Client implementation

use crate::protocol::{
    decode_value, encode_value, Command, ProtocolError, Status,
};
use data_bridge_kv::KvValue;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Client error types
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Connection error: {0}")]
    Connection(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Key not found")]
    KeyNotFound,
}

/// KV Store client
pub struct KvClient {
    stream: TcpStream,
    namespace: Option<String>,
}

impl KvClient {
    /// Connect to a KV server
    ///
    /// Supports namespace via connection string:
    /// - `127.0.0.1:6380` → no namespace
    /// - `127.0.0.1:6380/tasks` → namespace "tasks"
    /// - `127.0.0.1:6380/prod/cache` → namespace "prod/cache"
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        // Parse: "host:port/namespace" or "host:port"
        let (host_port, namespace) = if let Some(idx) = addr.find('/') {
            let (hp, ns) = addr.split_at(idx);
            (hp, Some(ns[1..].to_string()))  // skip the '/'
        } else {
            (addr, None)
        };

        let stream = TcpStream::connect(host_port).await?;
        stream.set_nodelay(true)?;
        Ok(Self { stream, namespace })
    }

    /// Prefix a key with namespace if configured
    fn prefix_key(&self, key: &str) -> String {
        match &self.namespace {
            Some(ns) => format!("{}:{}", ns, key),
            None => key.to_string(),
        }
    }

    /// Get the namespace if configured
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Send a request and read the response
    async fn request(&mut self, cmd: Command, payload: &[u8]) -> Result<(Status, Vec<u8>), ClientError> {
        // Build request
        let mut req = Vec::with_capacity(5 + payload.len());
        req.push(cmd as u8);
        req.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        req.extend_from_slice(payload);

        // Send request
        self.stream.write_all(&req).await?;

        // Read response header (5 bytes)
        let mut header = [0u8; 5];
        self.stream.read_exact(&mut header).await?;

        let status = match header[0] {
            0x00 => Status::Ok,
            0x01 => Status::Null,
            0x02 => Status::Error,
            _ => return Err(ClientError::Protocol(ProtocolError::InvalidCommand(header[0]))),
        };

        let payload_len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

        // Read response payload
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            self.stream.read_exact(&mut payload).await?;
        }

        // Check for error status
        if status == Status::Error {
            let msg = String::from_utf8_lossy(&payload).to_string();
            return Err(ClientError::Server(msg));
        }

        Ok((status, payload))
    }

    /// Ping the server
    pub async fn ping(&mut self) -> Result<String, ClientError> {
        let (_, payload) = self.request(Command::Ping, &[]).await?;
        Ok(String::from_utf8_lossy(&payload).to_string())
    }

    /// Get a value by key
    pub async fn get(&mut self, key: &str) -> Result<Option<KvValue>, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let (status, payload) = self.request(Command::Get, prefixed_key.as_bytes()).await?;

        if status == Status::Null {
            return Ok(None);
        }

        let (value, _) = decode_value(&payload)?;
        Ok(Some(value))
    }

    /// Set a value
    pub async fn set(&mut self, key: &str, value: KvValue, ttl: Option<Duration>) -> Result<(), ClientError> {
        let prefixed_key = self.prefix_key(key);
        let mut payload = Vec::new();

        // key_len (2 bytes) + key
        payload.extend_from_slice(&(prefixed_key.len() as u16).to_be_bytes());
        payload.extend_from_slice(prefixed_key.as_bytes());

        // ttl in ms (8 bytes)
        let ttl_ms = ttl.map(|d| d.as_millis() as u64).unwrap_or(0);
        payload.extend_from_slice(&ttl_ms.to_be_bytes());

        // value
        payload.extend_from_slice(&encode_value(&value));

        self.request(Command::Set, &payload).await?;
        Ok(())
    }

    /// Delete a key
    pub async fn delete(&mut self, key: &str) -> Result<bool, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let (_, payload) = self.request(Command::Del, prefixed_key.as_bytes()).await?;
        Ok(payload.first() == Some(&1))
    }

    /// Check if key exists
    pub async fn exists(&mut self, key: &str) -> Result<bool, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let (_, payload) = self.request(Command::Exists, prefixed_key.as_bytes()).await?;
        Ok(payload.first() == Some(&1))
    }

    /// Increment an integer value
    pub async fn incr(&mut self, key: &str, delta: i64) -> Result<i64, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let mut payload = Vec::new();
        payload.extend_from_slice(&(prefixed_key.len() as u16).to_be_bytes());
        payload.extend_from_slice(prefixed_key.as_bytes());
        payload.extend_from_slice(&delta.to_be_bytes());

        let (_, resp) = self.request(Command::Incr, &payload).await?;
        if resp.len() >= 8 {
            Ok(i64::from_be_bytes(resp[0..8].try_into().unwrap()))
        } else {
            Err(ClientError::Protocol(ProtocolError::UnexpectedEof))
        }
    }

    /// Decrement an integer value
    pub async fn decr(&mut self, key: &str, delta: i64) -> Result<i64, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let mut payload = Vec::new();
        payload.extend_from_slice(&(prefixed_key.len() as u16).to_be_bytes());
        payload.extend_from_slice(prefixed_key.as_bytes());
        payload.extend_from_slice(&delta.to_be_bytes());

        let (_, resp) = self.request(Command::Decr, &payload).await?;
        if resp.len() >= 8 {
            Ok(i64::from_be_bytes(resp[0..8].try_into().unwrap()))
        } else {
            Err(ClientError::Protocol(ProtocolError::UnexpectedEof))
        }
    }

    /// Get server info
    pub async fn info(&mut self) -> Result<String, ClientError> {
        let (_, payload) = self.request(Command::Info, &[]).await?;
        Ok(String::from_utf8_lossy(&payload).to_string())
    }

    /// Set if not exists (atomic)
    pub async fn setnx(&mut self, key: &str, value: KvValue, ttl: Option<Duration>) -> Result<bool, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let mut payload = Vec::new();

        // key_len (2 bytes) + key
        payload.extend_from_slice(&(prefixed_key.len() as u16).to_be_bytes());
        payload.extend_from_slice(prefixed_key.as_bytes());

        // ttl in ms (8 bytes)
        let ttl_ms = ttl.map(|d| d.as_millis() as u64).unwrap_or(0);
        payload.extend_from_slice(&ttl_ms.to_be_bytes());

        // value
        payload.extend_from_slice(&encode_value(&value));

        let (_, resp) = self.request(Command::Setnx, &payload).await?;
        Ok(resp.first() == Some(&1))
    }

    /// Acquire a distributed lock
    pub async fn lock(&mut self, key: &str, owner: &str, ttl: Duration) -> Result<bool, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let mut payload = Vec::new();

        // key_len (2 bytes) + key
        payload.extend_from_slice(&(prefixed_key.len() as u16).to_be_bytes());
        payload.extend_from_slice(prefixed_key.as_bytes());

        // owner_len (2 bytes) + owner
        payload.extend_from_slice(&(owner.len() as u16).to_be_bytes());
        payload.extend_from_slice(owner.as_bytes());

        // ttl in ms (8 bytes)
        let ttl_ms = ttl.as_millis() as u64;
        payload.extend_from_slice(&ttl_ms.to_be_bytes());

        let (_, resp) = self.request(Command::Lock, &payload).await?;
        Ok(resp.first() == Some(&1))
    }

    /// Release a distributed lock
    pub async fn unlock(&mut self, key: &str, owner: &str) -> Result<bool, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let mut payload = Vec::new();

        // key_len (2 bytes) + key
        payload.extend_from_slice(&(prefixed_key.len() as u16).to_be_bytes());
        payload.extend_from_slice(prefixed_key.as_bytes());

        // owner_len (2 bytes) + owner
        payload.extend_from_slice(&(owner.len() as u16).to_be_bytes());
        payload.extend_from_slice(owner.as_bytes());

        let (_, resp) = self.request(Command::Unlock, &payload).await?;
        Ok(resp.first() == Some(&1))
    }

    /// Extend lock TTL
    pub async fn extend_lock(&mut self, key: &str, owner: &str, ttl: Duration) -> Result<bool, ClientError> {
        let prefixed_key = self.prefix_key(key);
        let mut payload = Vec::new();

        // key_len (2 bytes) + key
        payload.extend_from_slice(&(prefixed_key.len() as u16).to_be_bytes());
        payload.extend_from_slice(prefixed_key.as_bytes());

        // owner_len (2 bytes) + owner
        payload.extend_from_slice(&(owner.len() as u16).to_be_bytes());
        payload.extend_from_slice(owner.as_bytes());

        // ttl in ms (8 bytes)
        let ttl_ms = ttl.as_millis() as u64;
        payload.extend_from_slice(&ttl_ms.to_be_bytes());

        let (_, resp) = self.request(Command::ExtendLock, &payload).await?;
        Ok(resp.first() == Some(&1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_parsing() {
        // Test without namespace
        let addr = "127.0.0.1:6380";
        let (host_port, namespace) = if let Some(idx) = addr.find('/') {
            let (hp, ns) = addr.split_at(idx);
            (hp, Some(ns[1..].to_string()))
        } else {
            (addr, None)
        };
        assert_eq!(host_port, "127.0.0.1:6380");
        assert_eq!(namespace, None);

        // Test with simple namespace
        let addr = "127.0.0.1:6380/tasks";
        let (host_port, namespace) = if let Some(idx) = addr.find('/') {
            let (hp, ns) = addr.split_at(idx);
            (hp, Some(ns[1..].to_string()))
        } else {
            (addr, None)
        };
        assert_eq!(host_port, "127.0.0.1:6380");
        assert_eq!(namespace, Some("tasks".to_string()));

        // Test with nested namespace
        let addr = "127.0.0.1:6380/prod/cache";
        let (host_port, namespace) = if let Some(idx) = addr.find('/') {
            let (hp, ns) = addr.split_at(idx);
            (hp, Some(ns[1..].to_string()))
        } else {
            (addr, None)
        };
        assert_eq!(host_port, "127.0.0.1:6380");
        assert_eq!(namespace, Some("prod/cache".to_string()));
    }

    #[test]
    fn test_prefix_key_logic() {
        // Test without namespace
        let namespace: Option<String> = None;
        let key = match &namespace {
            Some(ns) => format!("{}:{}", ns, "mykey"),
            None => "mykey".to_string(),
        };
        assert_eq!(key, "mykey");

        // Test with namespace
        let namespace = Some("tasks".to_string());
        let key = match &namespace {
            Some(ns) => format!("{}:{}", ns, "mykey"),
            None => "mykey".to_string(),
        };
        assert_eq!(key, "tasks:mykey");

        // Test with nested namespace
        let namespace = Some("prod/cache".to_string());
        let key = match &namespace {
            Some(ns) => format!("{}:{}", ns, "mykey"),
            None => "mykey".to_string(),
        };
        assert_eq!(key, "prod/cache:mykey");
    }

    // Integration tests require a running server
    // Run: cargo run -p data-bridge-kv-server
    // Then: cargo test -p data-bridge-kv-client -- --ignored

    #[tokio::test]
    #[ignore]
    async fn test_ping() {
        let mut client = KvClient::connect("127.0.0.1:6380").await.unwrap();
        let result = client.ping().await.unwrap();
        assert_eq!(result, "PONG");
    }

    #[tokio::test]
    #[ignore]
    async fn test_set_get() {
        let mut client = KvClient::connect("127.0.0.1:6380").await.unwrap();

        client.set("test_key", KvValue::String("hello".to_string()), None).await.unwrap();

        let result = client.get("test_key").await.unwrap();
        assert_eq!(result, Some(KvValue::String("hello".to_string())));
    }

    #[tokio::test]
    #[ignore]
    async fn test_incr() {
        let mut client = KvClient::connect("127.0.0.1:6380").await.unwrap();

        client.set("counter", KvValue::Int(10), None).await.unwrap();

        let result = client.incr("counter", 5).await.unwrap();
        assert_eq!(result, 15);
    }

    #[tokio::test]
    #[ignore]
    async fn test_delete() {
        let mut client = KvClient::connect("127.0.0.1:6380").await.unwrap();

        client.set("delete_me", KvValue::String("temp".to_string()), None).await.unwrap();

        let deleted = client.delete("delete_me").await.unwrap();
        assert!(deleted);

        let result = client.get("delete_me").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_exists() {
        let mut client = KvClient::connect("127.0.0.1:6380").await.unwrap();

        client.set("exists_key", KvValue::String("yes".to_string()), None).await.unwrap();

        let exists = client.exists("exists_key").await.unwrap();
        assert!(exists);

        let not_exists = client.exists("nonexistent").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    #[ignore]
    async fn test_ttl() {
        let mut client = KvClient::connect("127.0.0.1:6380").await.unwrap();

        // Set with 1 second TTL
        client.set(
            "ttl_key",
            KvValue::String("expires".to_string()),
            Some(Duration::from_secs(1))
        ).await.unwrap();

        // Should exist immediately
        let exists = client.exists("ttl_key").await.unwrap();
        assert!(exists);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Should be expired
        let result = client.get("ttl_key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_namespace() {
        // Connect with namespace
        let mut client1 = KvClient::connect("127.0.0.1:6380/test_ns").await.unwrap();
        assert_eq!(client1.namespace(), Some("test_ns"));

        // Connect without namespace
        let mut client2 = KvClient::connect("127.0.0.1:6380").await.unwrap();
        assert_eq!(client2.namespace(), None);

        // Set value in namespace
        client1.set("key1", KvValue::String("value1".to_string()), None).await.unwrap();

        // Should be able to read from same namespace
        let result = client1.get("key1").await.unwrap();
        assert_eq!(result, Some(KvValue::String("value1".to_string())));

        // Should NOT be visible from non-namespaced client
        let result = client2.get("key1").await.unwrap();
        assert_eq!(result, None);

        // Can access with manual prefix
        let result = client2.get("test_ns:key1").await.unwrap();
        assert_eq!(result, Some(KvValue::String("value1".to_string())));

        // Clean up
        client1.delete("key1").await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_nested_namespace() {
        // Connect with nested namespace
        let mut client = KvClient::connect("127.0.0.1:6380/prod/cache").await.unwrap();
        assert_eq!(client.namespace(), Some("prod/cache"));

        // Set and get with nested namespace
        client.set("session", KvValue::String("data".to_string()), None).await.unwrap();
        let result = client.get("session").await.unwrap();
        assert_eq!(result, Some(KvValue::String("data".to_string())));

        // Clean up
        client.delete("session").await.unwrap();
    }
}
