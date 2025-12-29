//! High-performance async PostgreSQL ORM for Python with Rust backend.
//!
//! This crate provides a pure Rust PostgreSQL ORM layer that serves as the backend
//! for the data-bridge Python library. It follows the same architectural principles
//! as data-bridge-mongodb:
//!
//! - Zero Python byte handling (all SQL/serialization in Rust)
//! - GIL release during I/O and CPU-intensive operations
//! - Parallel processing for bulk operations
//! - Security-first design with input validation
//! - Copy-on-Write state management
//!
//! # Architecture
//!
//! ```text
//! Python API Layer (document.py, fields.py, query.py)
//!           |
//!      PyO3 Bridge (crates/data-bridge/src/postgres.rs)
//!           |
//!    Pure Rust ORM (crates/data-bridge-postgres/src/)
//!           |
//!         SQLx (PostgreSQL driver)
//! ```

pub mod connection;
pub mod query;
pub mod row;
pub mod transaction;
pub mod types;
pub mod migration;
pub mod schema;
pub mod validation;

pub use connection::{Connection, PoolConfig};
pub use query::{QueryBuilder, Operator, OrderDirection, JoinType};
pub use row::{Row, RelationConfig};
pub use transaction::Transaction;
pub use types::{ExtractedValue, row_to_extracted};
pub use migration::{Migration, MigrationRunner, MigrationStatus};
pub use schema::{SchemaInspector, CascadeRule, BackRef};
pub use validation::validate_foreign_key_reference;

pub use data_bridge_common::{DataBridgeError, Result};
