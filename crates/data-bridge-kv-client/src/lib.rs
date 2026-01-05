//! TCP client for data-bridge KV store
//!
//! High-performance async client for connecting to kv-server.

mod protocol;
mod client;
mod pool;

pub use client::{ClientError, KvClient};
pub use pool::{KvPool, PoolConfig, PooledClient, PoolStats};
pub use data_bridge_kv::{KvError, KvValue};

// Re-export protocol types for advanced usage
pub use protocol::{ProtocolError, Command, Status};
