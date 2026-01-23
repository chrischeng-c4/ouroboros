//! # ouroboros-sheet-db
//!
//! High-performance spreadsheet database layer built on top of ouroboros-kv.

// WIP: Suppress warnings during development
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(clippy::all)]
//!
//! ## Architecture
//!
//! This crate provides:
//! - **Storage Layer**: Efficient cell storage using Morton encoding (Z-order curve)
//! - **Query Layer**: Range queries and spatial queries for spreadsheet operations
//! - **CRDT Layer**: Conflict-free replicated data types for collaborative editing
//!
//! ## Components
//!
//! - `storage`: Cell storage engine with WAL support
//! - `query`: Query builders for range and spatial queries
//! - `crdt`: CRDT operations for collaborative editing
//!
//! ## Example
//!
//! ```rust,ignore
//! use ouroboros_sheet_db::storage::CellStore;
//! use ouroboros_sheet_db::query::RangeQuery;
//!
//! // Create cell store
//! let store = CellStore::new("sheets.db").await?;
//!
//! // Query cells in range
//! let cells = store.query_range(0, 0, 10, 10).await?;
//! ```

pub mod storage;
pub mod query;
pub mod crdt;

use thiserror::Error;

/// Result type for sheet-db operations
pub type Result<T> = std::result::Result<T, SheetDbError>;

/// Error types for sheet-db operations
#[derive(Error, Debug)]
pub enum SheetDbError {
    /// Storage layer error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Query execution error
    #[error("Query error: {0}")]
    Query(String),

    /// CRDT operation error
    #[error("CRDT error: {0}")]
    Crdt(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// KV store error
    #[error("KV store error: {0}")]
    KvStore(String),

    /// Invalid input error
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// Re-export commonly used types
pub use storage::{CellStore, MortonKey};
pub use query::{RangeQuery, SpatialQuery};
pub use crdt::CrdtOperation;
