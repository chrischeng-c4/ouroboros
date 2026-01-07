//! High-performance, multi-core key-value store for data-bridge
//!
//! # Features
//! - Sharded storage engine for multi-core scalability
//! - High-precision numeric types (Decimal, f64, i64)
//! - Hybrid tiered storage (RAM + Disk)
//! - Compare-and-swap (CAS) for atomic state transitions
//! - Zero-copy serialization

pub mod engine;
pub mod types;
pub mod error;
pub mod persistence;

pub use data_bridge_common::{DataBridgeError, Result};
pub use engine::KvEngine;
pub use types::{KvKey, KvValue};
pub use error::KvError;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
