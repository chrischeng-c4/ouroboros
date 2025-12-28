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

**IN PROGRESS** - Core CRUD operations and schema introspection implemented.

### Implemented
- Connection pooling and management ✅
- Type mapping between Python and PostgreSQL ✅
- Basic CRUD operations (insert, fetch, update, delete) ✅
- Query builder with WHERE, ORDER BY, LIMIT, OFFSET ✅
- Transaction support with isolation levels ✅
- Raw SQL execution ✅
- **Schema introspection** ✅
  - `list_tables()` - List all tables in a schema
  - `table_exists()` - Check if a table exists
  - `get_columns()` - Get column information (types, constraints, defaults)
  - `get_indexes()` - Get index information
  - `inspect_table()` - Get complete table information

### TODO
- Migration management (up/down migrations)
- Foreign key introspection
- Advanced query features (JOINs, subqueries)
- Bulk operations optimization

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
cargo check -p data-bridge-postgres

# Run tests (when implemented)
cargo test -p data-bridge-postgres

# Lint
cargo clippy -p data-bridge-postgres
```

## License

MIT
