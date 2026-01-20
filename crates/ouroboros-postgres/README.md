# data-bridge-postgres

High-performance async PostgreSQL ORM for Python with Rust backend.

## Overview

This crate provides a pure Rust PostgreSQL ORM layer that serves as the backend for the data-bridge Python library. It follows the same architectural principles as `data-bridge-mongodb`:

- **Zero Python byte handling**: All SQL generation and serialization in Rust
- **GIL release**: During I/O and CPU-intensive operations
- **Parallel processing**: For bulk operations using Rayon
- **Security-first**: Input validation to prevent SQL injection
- **Type safety**: Full type mapping between Python and PostgreSQL

## Architecture

```
Python API Layer (document.py, fields.py, query.py)
          |
     PyO3 Bridge (crates/data-bridge/src/postgres.rs)
          |
   Pure Rust ORM (crates/data-bridge-postgres/src/)
          |
        SQLx (PostgreSQL driver)
```

## Modules

- **connection**: Connection pooling and management
- **types**: Type mapping between Python and PostgreSQL
- **row**: Row representation for query results
- **query**: Type-safe SQL query builder
- **transaction**: Transaction management with ACID guarantees
- **migration**: Database migration management
- **schema**: Schema introspection utilities

## Type Mapping

| Python Type | PostgreSQL Type | ExtractedValue Variant |
|------------|-----------------|------------------------|
| None | NULL | Null |
| bool | BOOLEAN | Bool |
| int (small) | INTEGER | Int |
| int (large) | BIGINT | BigInt |
| float | DOUBLE PRECISION | Double |
| str | TEXT | String |
| bytes | BYTEA | Bytes |
| uuid.UUID | UUID | Uuid |
| datetime.date | DATE | Date |
| datetime.time | TIME | Time |
| datetime.datetime (naive) | TIMESTAMP | Timestamp |
| datetime.datetime (aware) | TIMESTAMPTZ | TimestampTz |
| dict/list | JSONB | Json |
| list[T] | ARRAY | Array |
| Decimal | NUMERIC | Decimal |

## Features

- Connection pooling with configurable min/max connections
- Parameterized queries to prevent SQL injection
- Transaction support with savepoints
- Schema introspection (tables, columns, indexes, foreign keys)
- Migration management (up/down migrations)
- Comprehensive error handling with type conversion

## Status

**Version**: 0.1.0-alpha (Pre-release)

### Implemented
- Connection pooling and management ✅
- Type mapping between Python and PostgreSQL ✅
- Basic CRUD operations (insert, fetch, update, delete) ✅
- Query builder with WHERE, ORDER BY, LIMIT, OFFSET ✅
- Transaction support with isolation levels ✅
- Raw SQL execution ✅
- Schema introspection (tables, columns, indexes, foreign keys) ✅
- Migration management (up/down migrations) ✅
- Advanced query features (JOINs) ✅

### Roadmap (Future Releases)
- Subquery support
- Bulk operations optimization with Rayon
- Connection pool metrics and monitoring
- Prepared statement caching
- Advanced migration features (rollback chains, dry-run)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
data-bridge-postgres = "0.1.0-alpha"
```

Or use via the Python `data-bridge` package:

```bash
pip install data-bridge[postgres]
```

## Quick Start

```rust
use data_bridge_postgres::{ConnectionPool, QueryBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create connection pool
    let pool = ConnectionPool::new("postgresql://user:pass@localhost/db").await?;

    // Insert data
    let query = QueryBuilder::new("users")
        .insert(vec![
            ("name", "Alice".into()),
            ("email", "alice@example.com".into()),
        ]);
    pool.execute(query).await?;

    // Query data
    let query = QueryBuilder::new("users")
        .where_eq("name", "Alice")
        .limit(10);
    let rows = pool.fetch_all(query).await?;

    println!("Found {} users", rows.len());
    Ok(())
}
```

## Dependencies

- **sqlx** 0.8: Async PostgreSQL driver with connection pooling
- **tokio**: Async runtime
- **serde**: Serialization framework
- **uuid**: UUID support
- **chrono**: Date/time handling
- **rust_decimal**: Precise decimal arithmetic

## Development

```bash
# Check compilation
cargo check -p ouroboros-postgres

# Run unit tests (no database required)
cargo test -p ouroboros-postgres

# Run integration tests (requires PostgreSQL)
cargo test -p ouroboros-postgres --test test_transaction

# Run all tests including ignored ones
cargo test -p ouroboros-postgres -- --ignored

# Lint
cargo clippy -p ouroboros-postgres
```

### PostgreSQL Setup (macOS)

Using Homebrew:

```bash
# Install PostgreSQL
brew install postgresql@15

# Start PostgreSQL service
brew services start postgresql@15

# Create test database
createdb test_db

# Verify connection
psql -d test_db -c "SELECT version();"
```

Set the database URL environment variable:

```bash
# Default connection (local socket)
export DATABASE_URL="postgresql://localhost/test_db"

# Or with explicit credentials
export DATABASE_URL="postgresql://username:password@localhost:5432/test_db"
```

### PostgreSQL Setup (Docker)

```bash
# Start PostgreSQL container
docker run -d --name postgres-test \
  -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=test_db \
  postgres:15

# Set connection URL
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/test_db"

# Run tests
cargo test -p ouroboros-postgres --test test_transaction

# Cleanup
docker stop postgres-test && docker rm postgres-test
```

### Running Migration Tests

The migration integration tests require a running PostgreSQL database. They are marked with `#[ignore]` to prevent them from running in CI without a database.

```bash
# Start PostgreSQL (using Docker)
docker run -d --name postgres-test -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:15

# Create test database
docker exec -it postgres-test psql -U postgres -c "CREATE DATABASE test_db;"

# Run migration tests
export POSTGRES_URL="postgresql://postgres:postgres@localhost/test_db"
cargo test -p data-bridge-postgres --test test_migration -- --ignored

# Clean up
docker stop postgres-test && docker rm postgres-test
```

Each test creates its own migration table with a unique name (e.g., `_test_migrations_apply`) to avoid conflicts when running tests in parallel. Tables are automatically cleaned up after each test.

## License

MIT
