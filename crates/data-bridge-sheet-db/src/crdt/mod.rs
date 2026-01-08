//! CRDT (Conflict-free Replicated Data Type) layer for collaborative editing
//!
//! Provides eventual consistency for concurrent spreadsheet operations.

pub mod operations;

pub use operations::{CrdtOperation, OperationMetadata};
