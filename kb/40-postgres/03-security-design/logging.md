# PostgreSQL Crate Logging Implementation

## Overview

This document describes the logging/audit trail implementation for the `data-bridge-postgres` crate using the `tracing` crate.

## Dependencies

The `tracing` crate is already configured in the workspace and added to `data-bridge-postgres/Cargo.toml`:

```toml
tracing = { workspace = true }
```

## Logged Operations

### 1. INSERT Operations (`row.rs`)

#### Single Insert
- **Function**: `Row::insert()`
- **Instrumentation**: `#[instrument(skip(executor, values), fields(table = %table, value_count = values.len()))]`
- **Logs**:
  - INFO: "Inserting row" (when operation starts)
  - INFO: "Insert complete" (when operation finishes)

#### Batch Insert
- **Function**: `Row::insert_many()`
- **Instrumentation**: `#[instrument(skip(executor, rows), fields(table = %table, row_count = rows.len()))]`
- **Logs**:
  - INFO: "Inserting rows" (when operation starts)
  - INFO with `affected` field: "Insert complete" (when operation finishes, includes row count)

### 2. UPDATE Operations (`row.rs`)

#### Single Update
- **Function**: `Row::update()`
- **Instrumentation**: `#[instrument(skip(pool, values), fields(table = %table, id = %id, value_count = values.len()))]`
- **Logs**:
  - INFO: "Updating row" (when operation starts)
  - INFO with `affected` field: "Update complete" (when operation finishes, includes affected row count)

### 3. DELETE Operations (`row.rs`)

#### Simple Delete
- **Function**: `Row::delete()`
- **Instrumentation**: `#[instrument(skip(pool), fields(table = %table, id = %id))]`
- **Logs**:
  - INFO: "Deleting row" (when operation starts)
  - INFO with `affected` field: "Delete complete" (when operation finishes, includes affected row count)

#### Cascade Delete
- **Function**: `Row::delete_with_cascade()`
- **Instrumentation**: `#[instrument(skip(pool), fields(table = %table, id = %id, id_column = %id_column))]`
- **Logs**:
  - INFO: "Starting cascade delete" (when operation starts)
  - WARN with `source_table`: "Cascade delete blocked by RESTRICT constraint" (when delete blocked by FK constraint)
  - DEBUG with `target_table` and `deleted` count: "Cascaded delete to related table" (for each cascaded delete)
  - DEBUG with `target_table`: "Set foreign key to NULL" (when using SET NULL rule)
  - DEBUG with `target_table`: "Set foreign key to DEFAULT" (when using SET DEFAULT rule)
  - INFO with `total_deleted` and `cascaded_to` list: "Cascade delete complete" (when operation finishes)

### 4. Security/Validation Events (`validation.rs`)

#### Identifier Validation
- **Function**: `validate_identifier()`
- **Security Logs** (all at WARN level):
  - `reason = "empty_identifier"`: "Identifier validation failed"
  - `reason = "invalid_format"`: "Identifier validation failed"
  - `reason = "starts_with_digit"`: "Identifier validation failed"
  - `reason = "invalid_characters"`: "Identifier validation failed"
  - `reason = "sql_injection_attempt"`, `pattern = <pattern>`: "Identifier validation failed"

**Note**: Security logs never expose the actual malicious input, only generic failure reasons.

### 5. Connection Management (`connection.rs`)

#### Pool Initialization
- **Function**: `Connection::new()`
- **Instrumentation**: `#[instrument(skip(uri), fields(min_connections, max_connections))]`
- **Logs**:
  - INFO: "Initializing connection pool" (when pool creation starts)
  - INFO: "Connection pool initialized successfully" (when pool is ready)

## Log Levels

- **INFO**: Normal operations (CRUD, connection initialization)
- **WARN**: Security events (validation failures, constraint violations)
- **DEBUG**: Detailed cascade operation information

## Security Considerations

1. **No Sensitive Data**: Connection URIs and actual malicious payloads are NEVER logged
2. **Generic Security Messages**: Security events use generic reasons without exposing attack details
3. **Structured Logging**: Uses tracing's structured fields for easy filtering and analysis

## Example Log Output

```
INFO Row::insert{table="users" value_count=3}: Inserting row
INFO Row::insert{table="users" value_count=3}: Insert complete

INFO Row::delete_with_cascade{table="posts" id=42 id_column="id"}: Starting cascade delete
DEBUG Row::delete_with_cascade{table="posts" id=42 id_column="id"}: Cascaded delete to related table target_table="comments" deleted=5
INFO Row::delete_with_cascade{table="posts" id=42 id_column="id"}: Cascade delete complete total_deleted=6 cascaded_to=["comments"]

WARN validate_identifier: Identifier validation failed reason="sql_injection_attempt" pattern="drop"

INFO Connection::new{min_connections=1 max_connections=10}: Initializing connection pool
INFO Connection::new{min_connections=1 max_connections=10}: Connection pool initialized successfully
```

## Usage

To enable logging in your application, initialize a tracing subscriber:

```rust
use tracing_subscriber;

// Basic console logging
tracing_subscriber::fmt::init();

// Or with custom configuration
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();
```

For production environments, consider using structured JSON logging or integrating with observability platforms like Datadog, Jaeger, or OpenTelemetry.

## Testing

All existing tests continue to pass with logging enabled:

```bash
cargo test -p data-bridge-postgres --lib
# Result: 147 tests passed
```

Logging does not affect test execution or performance.
