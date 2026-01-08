---
title: Core PostgreSQL Data Flows
status: planning
component: postgres
type: data-flow
---

# Core PostgreSQL Data Flows

> Part of [Core PostgreSQL Engine Documentation](./index.md)

This document illustrates the data flow for key operations, highlighting the interaction between Python, PyO3, and Rust, specifically focusing on GIL management and SQL query execution.

## 1. Insert One Operation (Write Path)

Goal: Save a Python object to PostgreSQL efficiently.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant Bind as PyO3 Binding
    participant Rust as Rust Core
    participant DB as PostgreSQL

    Py->>Bind: table.save()
    Note right of Py: GIL HELD

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 1: Extraction
        Bind->>Bind: extract_py_dict(self)<br/>(PyDict -> ExtractedRow)
        Note right of Bind: Fast, memory-only
    end

    Bind->>Rust: async insert_one(extracted)
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rust: Phase 2: SQL Build & Execute
        Rust->>Rust: extracted_to_params()<br/>(ExtractedRow -> SqlParams)
        Rust->>Rust: build_insert_sql(table, fields)
        Rust->>Rust: validate_field_names()
        Rust->>DB: INSERT INTO ... VALUES ($1, $2, ...) RETURNING id
        DB-->>Rust: result (id)
    end

    Rust-->>Bind: result
    Note right of Py: GIL ACQUIRED

    Bind-->>Py: return result
```

## 2. Find One Operation (Read Path)

Goal: Retrieve a row and convert it to a Python dict/object.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant Bind as PyO3 Binding
    participant Rust as Rust Core
    participant DB as PostgreSQL

    Py->>Bind: find_one(filter)
    Note right of Py: GIL HELD

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 1: Filter Extraction
        Bind->>Bind: extract_filter(filter)<br/>(PyDict -> FilterExpr)
    end

    Bind->>Rust: async fetch_one(filter)
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rust: Phase 2: SQL Build & I/O
        Rust->>Rust: build_select_sql(filter)
        Rust->>DB: SELECT * FROM table WHERE ...
        DB-->>Rust: Row
        Rust->>Rust: row_to_dict()<br/>(sqlx::Row -> HashMap)
    end

    Rust-->>Bind: HashMap<String, Value>
    Note right of Py: GIL ACQUIRED

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 3: Object Creation
        Bind->>Bind: dict_to_py(map)<br/>(HashMap -> PyDict)
        Note right of Bind: Construct Python dicts
    end

    Bind-->>Py: return Python Object
```

## 3. Bulk Insert (Parallel Optimization)

Goal: Insert 10,000 rows as fast as possible with parallel conversion.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant Bind as PyO3 Binding
    participant Rayon as Rust Rayon Pool
    participant DB as PostgreSQL

    Py->>Bind: insert_many(rows)
    Note right of Py: GIL HELD

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 1: Serial Extraction
        Bind->>Bind: Loop rows: extract_py_dict()
        Note right of Bind: Creates Vec<ExtractedRow>
    end

    Bind->>Rayon: spawn_parallel(extracted_list)
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rayon: Phase 2: Parallel Conversion
        alt len(rows) >= 50
            Rayon->>Rayon: par_iter().map(to_sql_params)
            Note right of Rayon: Uses all CPU cores
        else len(rows) < 50
            Rayon->>Rayon: Sequential conversion
            Note right of Rayon: Overhead not worth it
        end
        Rayon->>Rayon: build_batch_insert_sql()
        Rayon->>DB: INSERT INTO ... VALUES ($1, $2), ($3, $4), ...
        DB-->>Rayon: results (ids)
    end

    Rayon-->>Bind: Vec<id>
    Note right of Py: GIL ACQUIRED

    Bind-->>Py: return results
```

## 4. Transaction Flow

Goal: Execute multiple operations within a ACID transaction.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant Bind as PyO3 Binding
    participant Rust as Rust Core
    participant DB as PostgreSQL

    Py->>Bind: async with pg_transaction()
    Note right of Py: GIL HELD

    Bind->>Rust: begin_transaction()
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rust: Transaction Started
        Rust->>DB: BEGIN
        DB-->>Rust: Transaction handle
    end

    Rust-->>Bind: Transaction ID
    Note right of Py: GIL ACQUIRED

    loop Operations within transaction
        Py->>Bind: operation(txn_id, ...)
        Note right of Py: GIL HELD -> RELEASED
        Bind->>Rust: execute_in_txn(txn_id, op)
        Rust->>DB: INSERT/UPDATE/DELETE
        DB-->>Rust: result
        Rust-->>Bind: result
        Bind-->>Py: result
        Note right of Py: GIL ACQUIRED
    end

    alt Success path
        Py->>Bind: commit(txn_id)
        Note right of Py: GIL RELEASED
        Bind->>Rust: commit_transaction(txn_id)
        Rust->>DB: COMMIT
        DB-->>Rust: success
        Rust-->>Bind: success
        Note right of Py: GIL ACQUIRED
        Bind-->>Py: Transaction committed
    else Error path
        Py->>Bind: rollback(txn_id)
        Note right of Py: GIL RELEASED
        Bind->>Rust: rollback_transaction(txn_id)
        Rust->>DB: ROLLBACK
        DB-->>Rust: rolled back
        Rust-->>Bind: rolled back
        Note right of Py: GIL ACQUIRED
        Bind-->>Py: Transaction aborted
    end
```

## 5. Query Builder Flow

Goal: Build and execute complex queries with filters, ordering, and pagination.

```mermaid
sequenceDiagram
    participant Py as Python Runtime
    participant QB as QueryBuilder (Python)
    participant Bind as PyO3 Binding
    participant Rust as Rust Core
    participant DB as PostgreSQL

    Py->>QB: User.find(User.age > 25)
    Note right of Py: GIL HELD
    QB->>QB: Chain filter expression
    Note right of QB: QueryBuilder state

    Py->>QB: .order_by(User.name)
    QB->>QB: Add ordering clause

    Py->>QB: .limit(10)
    QB->>QB: Add limit clause

    Py->>QB: .to_list()
    Note right of QB: Execute query

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 1: Extract Query State
        QB->>Bind: execute_query(query_state)
        Bind->>Bind: extract_filters(filters)
        Bind->>Bind: extract_ordering(ordering)
        Bind->>Bind: extract_limit(limit)
    end

    Bind->>Rust: async execute(query_components)
    Note right of Py: GIL RELEASED

    rect rgb(200, 255, 200)
        Note right of Rust: Phase 2: SQL Build & Execute
        Rust->>Rust: build_select_sql()<br/>SELECT * FROM users<br/>WHERE age > $1<br/>ORDER BY name<br/>LIMIT 10
        Rust->>Rust: bind_parameters([25])
        Rust->>DB: Execute prepared statement
        DB-->>Rust: Vec<Row>
        Rust->>Rust: rows_to_dicts()<br/>(Vec<Row> -> Vec<HashMap>)
    end

    Rust-->>Bind: Vec<HashMap>
    Note right of Py: GIL ACQUIRED

    rect rgb(255, 255, 200)
        Note right of Bind: Phase 3: Convert to Python
        Bind->>Bind: Loop: dict_to_py(map)
        Bind->>Bind: Construct Python list of objects
    end

    Bind-->>QB: Vec<PyObject>
    QB-->>Py: return [User(...), User(...), ...]
```

## Data Transformation Pipeline

The following diagram shows how data types are transformed at each stage.

```
[ Python Layer ]       [ PyO3 Bridge ]                  [ Rust Core ]              [ PostgreSQL ]
  dict {                 ExtractedRow {                   SqlParams {                SQL Wire Protocol
    "name": "A",   ->      name: String("A"),     ->        params: [              ->   Binary data
    "age": 30              age: Int(30)                       "A", 30
  }                      }                                  ]
                                                         }
      |                          |                                |
      | (GIL Held)               | (GIL Released)                 | (Network)
      +------------------------> +------------------------------> +
         Extraction                  SQL Param Binding              Wire Protocol
```

## PostgreSQL Type Mapping

```
Python Type      ExtractedValue          sqlx::Type           PostgreSQL Type
-----------      --------------          ----------           ---------------
int         ->   Int64(n)           ->   i64             ->   BIGINT
float       ->   Float64(f)         ->   f64             ->   DOUBLE PRECISION
str         ->   String(s)          ->   &str            ->   TEXT/VARCHAR
bool        ->   Bool(b)            ->   bool            ->   BOOLEAN
datetime    ->   Timestamp(ts)      ->   chrono::Dt      ->   TIMESTAMPTZ
date        ->   Date(d)            ->   chrono::Date    ->   DATE
bytes       ->   Bytes(b)           ->   &[u8]           ->   BYTEA
list        ->   List(vec)          ->   Vec<T>          ->   ARRAY
dict        ->   Json(map)          ->   serde_json      ->   JSONB
None        ->   Null               ->   Option<T>       ->   NULL
UUID        ->   Uuid(u)            ->   uuid::Uuid      ->   UUID
Decimal     ->   Decimal(d)         ->   rust_decimal    ->   NUMERIC
```

## Error Propagation Flow

```mermaid
sequenceDiagram
    participant Rust
    participant PyO3
    participant Python

    alt Database Error
        Rust->>Rust: sqlx::Error (Constraint violation)
        Rust->>PyO3: Result::Err(Error::Database(...))
        PyO3->>PyO3: Map Error -> PyErr (IntegrityError)
        PyO3->>Python: Raise Exception
        Note over Python: try/except block catches it
    else Connection Error
        Rust->>Rust: sqlx::Error (Connection refused)
        Rust->>PyO3: Result::Err(Error::Connection(...))
        PyO3->>PyO3: Map Error -> PyErr (ConnectionError)
        PyO3->>Python: Raise Exception
    else Type Conversion Error
        Rust->>Rust: Failed to parse timestamp
        Rust->>PyO3: Result::Err(Error::TypeConversion(...))
        PyO3->>PyO3: Map Error -> PyErr (TypeError)
        PyO3->>Python: Raise Exception
    end
```

## Key Design Decisions

### 1. GIL Release Strategy
- **Extract in Python land** (GIL held): Fast, minimal overhead
- **Convert & execute in Rust** (GIL released): CPU-intensive, I/O-bound work
- **Construct Python objects** (GIL acquired): Necessary for return

### 2. Parallel Conversion Threshold
- **Threshold: 50 rows**
  - Below: Sequential conversion (less overhead)
  - Above: Rayon parallel conversion (utilize all cores)

### 3. SQL Parameter Binding
- **Always use parameterized queries**: Prevents SQL injection
- **Never inline values**: Security and performance (query plan caching)
- **Type-safe binding**: sqlx validates types at compile time

### 4. Connection Pooling
- **Pool per database**: Reuse connections, avoid handshake overhead
- **Lazy initialization**: Only create connections when needed
- **Health checks**: Periodic validation of pooled connections

### 5. Transaction Management
- **Context manager pattern**: `async with pg_transaction()`
- **Automatic rollback**: On exception or explicit rollback
- **Nested transaction support**: Using SAVEPOINT (future enhancement)

## Performance Characteristics

| Operation | GIL Held Time | GIL Released Time | Bottleneck |
|-----------|---------------|-------------------|------------|
| Insert One | ~50μs (extract) | ~1-5ms (SQL exec) | Network I/O |
| Find One | ~20μs (extract) | ~1-3ms (SQL exec) | Network I/O |
| Insert Many (1000) | ~5ms (extract) | ~10-20ms (convert + exec) | CPU (conversion) |
| Transaction | ~100μs (setup) | ~5-50ms (operations) | Network I/O |
| Complex Query | ~200μs (build) | ~5-100ms (exec) | Database query planner |

**Key Insight**: By releasing GIL during SQL execution and conversion, other Python threads can run concurrently, maximizing throughput in multi-threaded applications.
