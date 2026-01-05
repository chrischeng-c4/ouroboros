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
}

impl KvClient {
    /// Connect to a KV server
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        Ok(Self { stream })
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
        let (status, payload) = self.request(Command::Get, key.as_bytes()).await?;

        if status == Status::Null {
            return Ok(None);
        }

        let (value, _) = decode_value(&payload)?;
        Ok(Some(value))
    }

    /// Set a value
    pub async fn set(&mut self, key: &str, value: KvValue, ttl: Option<Duration>) -> Result<(), ClientError> {
        let mut payload = Vec::new();

        // key_len (2 bytes) + key
        payload.extend_from_slice(&(key.len() as u16).to_be_bytes());
        payload.extend_from_slice(key.as_bytes());

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
        let (_, payload) = self.request(Command::Del, key.as_bytes()).await?;
        Ok(payload.first() == Some(&1))
    }

    /// Check if key exists
    pub async fn exists(&mut self, key: &str) -> Result<bool, ClientError> {
        let (_, payload) = self.request(Command::Exists, key.as_bytes()).await?;
        Ok(payload.first() == Some(&1))
    }

    /// Increment an integer value
    pub async fn incr(&mut self, key: &str, delta: i64) -> Result<i64, ClientError> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&(key.len() as u16).to_be_bytes());
        payload.extend_from_slice(key.as_bytes());
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
        let mut payload = Vec::new();
        payload.extend_from_slice(&(key.len() as u16).to_be_bytes());
        payload.extend_from_slice(key.as_bytes());
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
