//! Storage layer for spreadsheet cells
//!
//! This module provides efficient storage for spreadsheet cells using:
//! - Morton encoding (Z-order curve) for spatial locality
//! - Write-ahead log (WAL) for durability
//! - Cell store with atomic operations
//!
//! ## Architecture
//!
//! ```text
//! CellStore
//!   ├── Morton Encoding (2D coordinates → 1D key)
//!   ├── WAL (Write-Ahead Log for durability)
//!   └── KV Store Backend
//! ```

pub mod cell_store;
pub mod morton;
pub mod wal;

pub use cell_store::{CellStore, StoredCell};
pub use morton::MortonKey;
pub use wal::WriteAheadLog;
