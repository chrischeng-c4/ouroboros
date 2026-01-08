//! TCP server implementation

use crate::protocol::{
    encode_mget_response, encode_value, parse_incr_payload, parse_key, parse_lock_payload,
    parse_mget_payload, parse_mset_payload, parse_set_payload, read_request, write_response,
    Command, ProtocolError, Status,
};
use data_bridge_kv::{KvEngine, KvKey, KvValue};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, warn};

/// KV Server
pub struct KvServer {
    engine: Arc<KvEngine>,
}

impl KvServer {
    /// Create a new KV server
    pub fn new(num_shards: usize) -> Self {
        Self {
            engine: Arc::new(KvEngine::with_shards(num_shards)),
        }
    }

    /// Create a KV server with an existing engine (for persistence support)
    pub fn with_engine(engine: Arc<KvEngine>) -> Self {
        Self { engine }
    }

    /// Run the server
    pub async fn run(&self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        info!("Server listening on {}", addr);

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            let engine = self.engine.clone();

            tokio::spawn(async move {
                debug!("New connection from {}", peer_addr);
                if let Err(e) = handle_connection(socket, engine).await {
                    warn!("Connection error from {}: {}", peer_addr, e);
                }
                debug!("Connection closed: {}", peer_addr);
            });
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    engine: Arc<KvEngine>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Disable Nagle's algorithm for lower latency
    socket.set_nodelay(true)?;

    let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer

    loop {
        // Read header (5 bytes: 1 cmd + 4 len)
        let n = socket.read(&mut buf[..5]).await?;
        if n == 0 {
            return Ok(()); // Connection closed
        }
        if n < 5 {
            // Partial read, try to read more
            socket.read_exact(&mut buf[n..5]).await?;
        }

        // Parse payload length
        let payload_len = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;

        // Read payload
        if payload_len > 0 {
            if buf.len() < 5 + payload_len {
                buf.resize(5 + payload_len, 0);
            }
            socket.read_exact(&mut buf[5..5 + payload_len]).await?;
        }

        // Process request
        let response = match process_request(&buf[..5 + payload_len], &engine) {
            Ok(resp) => resp,
            Err(e) => {
                let msg = e.to_string();
                write_response(Status::Error, msg.as_bytes())
            }
        };

        // Send response
        socket.write_all(&response).await?;
    }
}

fn process_request(data: &[u8], engine: &KvEngine) -> Result<Vec<u8>, ProtocolError> {
    let (cmd, payload) = read_request(data)?;

    match cmd {
        Command::Ping => {
            Ok(write_response(Status::Ok, b"PONG"))
        }
        Command::Get => {
            let key_str = parse_key(&payload)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            match engine.get(&key) {
                Some(value) => {
                    let encoded = encode_value(&value);
                    Ok(write_response(Status::Ok, &encoded))
                }
                None => Ok(write_response(Status::Null, &[])),
            }
        }
        Command::Set => {
            let (key_str, ttl_ms, value) = parse_set_payload(&payload)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;
            let ttl = ttl_ms.map(Duration::from_millis);

            engine.set(&key, value, ttl);
            Ok(write_response(Status::Ok, &[]))
        }
        Command::Del => {
            let key_str = parse_key(&payload)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            let deleted = engine.delete(&key);
            let result = if deleted { 1u8 } else { 0u8 };
            Ok(write_response(Status::Ok, &[result]))
        }
        Command::Exists => {
            let key_str = parse_key(&payload)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            let exists = engine.exists(&key);
            let result = if exists { 1u8 } else { 0u8 };
            Ok(write_response(Status::Ok, &[result]))
        }
        Command::Incr => {
            let (key_str, delta) = parse_incr_payload(&payload)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            match engine.incr(&key, delta) {
                Ok(new_val) => {
                    Ok(write_response(Status::Ok, &new_val.to_be_bytes()))
                }
                Err(e) => {
                    Ok(write_response(Status::Error, e.to_string().as_bytes()))
                }
            }
        }
        Command::Decr => {
            let (key_str, delta) = parse_incr_payload(&payload)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            match engine.decr(&key, delta) {
                Ok(new_val) => {
                    Ok(write_response(Status::Ok, &new_val.to_be_bytes()))
                }
                Err(e) => {
                    Ok(write_response(Status::Error, e.to_string().as_bytes()))
                }
            }
        }
        Command::Cas => {
            // TODO: Implement CAS parsing and execution
            Ok(write_response(Status::Error, b"CAS not implemented yet"))
        }
        Command::Setnx => {
            let (key_str, ttl_ms, value) = parse_set_payload(&payload)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;
            let ttl = ttl_ms.map(Duration::from_millis);

            let success = engine.setnx(&key, value, ttl);
            let result = if success { 1u8 } else { 0u8 };
            Ok(write_response(Status::Ok, &[result]))
        }
        Command::Lock => {
            let (key_str, owner, ttl_ms) = parse_lock_payload(&payload, true)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;
            let ttl = Duration::from_millis(ttl_ms.unwrap_or(30000));

            let success = engine.lock(&key, &owner, ttl);
            let result = if success { 1u8 } else { 0u8 };
            Ok(write_response(Status::Ok, &[result]))
        }
        Command::Unlock => {
            let (key_str, owner, _) = parse_lock_payload(&payload, false)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            match engine.unlock(&key, &owner) {
                Ok(success) => {
                    let result = if success { 1u8 } else { 0u8 };
                    Ok(write_response(Status::Ok, &[result]))
                }
                Err(e) => {
                    Ok(write_response(Status::Error, e.to_string().as_bytes()))
                }
            }
        }
        Command::ExtendLock => {
            let (key_str, owner, ttl_ms) = parse_lock_payload(&payload, true)?;
            let key = KvKey::new(&key_str).map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;
            let ttl = Duration::from_millis(ttl_ms.unwrap_or(30000));

            match engine.extend_lock(&key, &owner, ttl) {
                Ok(success) => {
                    let result = if success { 1u8 } else { 0u8 };
                    Ok(write_response(Status::Ok, &[result]))
                }
                Err(e) => {
                    Ok(write_response(Status::Error, e.to_string().as_bytes()))
                }
            }
        }
        Command::MGet => {
            let keys = parse_mget_payload(&payload)?;
            let kv_keys: Result<Vec<_>, _> = keys.iter()
                .map(|k| KvKey::new(k))
                .collect();

            let kv_keys = kv_keys.map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            let key_refs: Vec<&KvKey> = kv_keys.iter().collect();
            let values = engine.mget(&key_refs);

            let encoded = encode_mget_response(&values);
            Ok(write_response(Status::Ok, &encoded))
        }
        Command::MSet => {
            let (pairs, ttl_ms) = parse_mset_payload(&payload)?;
            let ttl = ttl_ms.map(Duration::from_millis);

            let kv_pairs: Result<Vec<_>, _> = pairs.iter()
                .map(|(k, v)| KvKey::new(k).map(|key| (key, v.clone())))
                .collect();

            let kv_pairs = kv_pairs.map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            let pair_refs: Vec<(&KvKey, KvValue)> = kv_pairs.iter()
                .map(|(k, v)| (k, v.clone()))
                .collect();

            engine.mset(&pair_refs, ttl);
            Ok(write_response(Status::Ok, &[]))
        }
        Command::MDel => {
            let keys = parse_mget_payload(&payload)?; // Same format as MGET
            let kv_keys: Result<Vec<_>, _> = keys.iter()
                .map(|k| KvKey::new(k))
                .collect();

            let kv_keys = kv_keys.map_err(|e| ProtocolError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            ))?;

            let key_refs: Vec<&KvKey> = kv_keys.iter().collect();
            let deleted = engine.mdel(&key_refs);

            // Return count as u32 big-endian
            Ok(write_response(Status::Ok, &(deleted as u32).to_be_bytes()))
        }
        Command::Info => {
            let info = format!(
                r#"{{"shards":{},"entries":{}}}"#,
                engine.num_shards(),
                engine.len()
            );
            Ok(write_response(Status::Ok, info.as_bytes()))
        }
    }
}
