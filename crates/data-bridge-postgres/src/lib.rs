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
//!
//! # Key Features
//!
//! - **Advanced Query Builder**: Fluent API with support for complex queries including:
//!   - Joins (INNER, LEFT, RIGHT, FULL OUTER)
//!   - Subqueries and CTEs (Common Table Expressions)
//!   - Window functions (ROW_NUMBER, RANK, LEAD, LAG)
//!   - Aggregations with GROUP BY and HAVING clauses
//!   - DISTINCT ON for PostgreSQL-specific deduplication
//!
//! - **Relationship Management**:
//!   - One-to-One, One-to-Many, Many-to-Many relationships
//!   - Cascade operations (ON DELETE CASCADE, SET NULL, RESTRICT)
//!   - Back-references with automatic join queries
//!   - Lazy and eager loading strategies
//!
//! - **Transaction Support**:
//!   - ACID-compliant transactions with savepoints
//!   - Nested transaction support via savepoints
//!   - Automatic rollback on error
//!   - Connection pooling for optimal performance
//!
//! - **Schema Management**:
//!   - Schema introspection and validation
//!   - Migration system with version tracking
//!   - Type-safe column operations
//!   - Foreign key constraint validation
//!
//! - **Performance Optimizations**:
//!   - Connection pooling with configurable limits
//!   - Prepared statement caching
//!   - Bulk insert/update operations
//!   - Parallel query execution
//!
//! # Usage Examples
//!
//! ## Basic Query Execution
//!
//! ```rust,no_run
//! use data_bridge_postgres::{Connection, QueryBuilder, Operator};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to PostgreSQL
//! let conn = Connection::new("postgresql://user:pass@localhost/dbname").await?;
//!
//! // Build and execute a query
//! let query = QueryBuilder::new("users")
//!     .select(&["id", "name", "email"])
//!     .where_clause("age", Operator::GreaterThan, "18")
//!     .order_by("name", data_bridge_postgres::OrderDirection::Asc)
//!     .limit(10);
//!
//! let rows = conn.fetch_all(&query).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Transaction with Savepoints
//!
//! ```rust,no_run
//! use data_bridge_postgres::Connection;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let conn = Connection::new("postgresql://localhost/db").await?;
//! let mut txn = conn.begin().await?;
//!
//! // Insert user
//! txn.execute("INSERT INTO users (name) VALUES ($1)", &["Alice"]).await?;
//!
//! // Create savepoint
//! txn.savepoint("sp1").await?;
//!
//! // This might fail
//! if let Err(_) = txn.execute("INSERT INTO users (name) VALUES ($1)", &["Bob"]).await {
//!     // Rollback to savepoint, keeping Alice's insert
//!     txn.rollback_to("sp1").await?;
//! }
//!
//! // Commit the transaction
//! txn.commit().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Complex Query with Joins and Window Functions
//!
//! ```rust,no_run
//! use data_bridge_postgres::{QueryBuilder, JoinType, WindowFunction, WindowSpec};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let query = QueryBuilder::new("orders")
//!     .select(&["orders.id", "orders.amount", "customers.name"])
//!     .join(
//!         JoinType::Inner,
//!         "customers",
//!         "orders.customer_id",
//!         "customers.id"
//!     )
//!     .window_function(
//!         WindowFunction::RowNumber,
//!         WindowSpec::new()
//!             .partition_by(&["customers.id"])
//!             .order_by("orders.created_at", data_bridge_postgres::OrderDirection::Desc)
//!     )
//!     .order_by("orders.created_at", data_bridge_postgres::OrderDirection::Desc);
//!
//! // Execute query...
//! # Ok(())
//! # }
//! ```
//!
//! # Async Runtime
//!
//! This crate requires an async runtime (Tokio) to function. All database operations
//! are async and must be awaited. When used from Python via PyO3, the GIL is released
//! during I/O operations to allow concurrent Python threads to execute.
//!
//! # Safety
//!
//! All SQL queries are parameterized to prevent SQL injection attacks. Table names,
//! column names, and operators are validated before query execution. Foreign key
//! references are validated to ensure referential integrity.
//!
//! # Thread Safety
//!
//! The `Connection` type is **not** `Send` or `Sync` by design. Each async task should
//! obtain its own connection from the pool. The connection pool itself is thread-safe
//! and can be cloned cheaply across threads.

/// Database connection management with connection pooling.
///
/// Provides the `Connection` type for executing queries and the `PoolConfig`
/// for configuring connection pool behavior (max connections, timeouts, etc).
pub mod connection;

/// Query builder with support for complex SQL operations.
///
/// Includes support for SELECT, INSERT, UPDATE, DELETE operations with:
/// - WHERE clauses with various operators
/// - JOIN operations (INNER, LEFT, RIGHT, FULL OUTER)
/// - Subqueries and CTEs (Common Table Expressions)
/// - Window functions (ROW_NUMBER, RANK, LEAD, LAG)
/// - GROUP BY and HAVING clauses
/// - DISTINCT ON for PostgreSQL-specific operations
pub mod query;

/// Row representation and relationship configuration.
///
/// Provides the `Row` type for working with query results and `RelationConfig`
/// for defining relationships between tables (One-to-One, One-to-Many, Many-to-Many).
pub mod row;

/// Transaction management with savepoint support.
///
/// ACID-compliant transactions with support for nested transactions via savepoints.
/// Automatic rollback on error ensures data integrity.
pub mod transaction;

/// Type conversion utilities for PostgreSQL types.
///
/// Handles conversion between Rust types and PostgreSQL types, including support
/// for arrays, JSON, and custom types.
pub mod types;

/// Database migration system with version tracking.
///
/// Provides tools for managing schema changes over time with up/down migrations,
/// automatic version tracking, and migration history.
pub mod migration;

/// Schema introspection and validation utilities.
///
/// Tools for inspecting database schema, validating foreign key relationships,
/// and managing cascade rules for referential integrity.
pub mod schema;

/// Input validation for SQL operations.
///
/// Security-focused validation for table names, column names, and foreign key
/// references to prevent SQL injection and ensure data integrity.
pub mod validation;

pub use connection::{Connection, PoolConfig};
pub use query::{
    QueryBuilder, Operator, OrderDirection, JoinType, JoinCondition,
    AggregateFunction, HavingCondition, WindowFunction, WindowSpec, WindowExpression
};
pub use row::{Row, RelationConfig};
pub use transaction::Transaction;
pub use types::{ExtractedValue, row_to_extracted};
pub use migration::{Migration, MigrationRunner, MigrationStatus};
pub use schema::{SchemaInspector, CascadeRule, BackRef, ManyToManyConfig};
pub use validation::validate_foreign_key_reference;

pub use data_bridge_common::{DataBridgeError, Result};
