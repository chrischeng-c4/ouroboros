# PostgreSQL Benchmark Architecture

## Overview

This benchmark suite provides comprehensive performance comparisons between data-bridge-postgres and other popular Python PostgreSQL libraries. The architecture follows the same proven patterns used in the MongoDB benchmarks.

## Design Principles

### 1. Framework Parity

All benchmarks test the same operations across all frameworks:
- **data-bridge-postgres**: Rust-backed ORM with zero Python byte handling
- **asyncpg**: High-performance async driver (baseline for async operations)
- **psycopg2**: Traditional sync driver (baseline for sync operations)
- **SQLAlchemy**: Popular ORM (baseline for ORM operations)

### 2. Fair Comparison

- Identical data sets for all frameworks
- Same connection pool sizes (min=2, max=10)
- Warm-up iterations to stabilize JIT/cache effects
- Isolated test tables per framework to prevent interference

### 3. Real-World Scenarios

Benchmarks cover common database operations:
- Single row operations (find one, insert one, update one)
- Bulk operations (1000, 10000 rows)
- Filtered queries (WHERE clauses)
- Aggregations (COUNT)
- Transactions (future)

## Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│         Benchmark Runner (dbtest CLI)                   │
│  python/data_bridge/test/cli.py                         │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│         Benchmark Groups (BenchmarkGroup)                │
│  - bench_insert.py: Insert operations                   │
│  - bench_find.py: Select/find operations                │
│  - bench_update.py: Update/delete operations            │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│         Test Fixtures (conftest.py)                      │
│  - Connection pools (session-scoped)                    │
│  - Data generators                                       │
│  - Table setup/cleanup (function-scoped)                │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│         Framework Adapters                               │
│  - data_bridge.postgres (Rust engine)                   │
│  - asyncpg (native async driver)                        │
│  - psycopg2 (sync driver)                               │
│  - SQLAlchemy (ORM layer)                               │
└─────────────────────────────────────────────────────────┘
```

## File Organization

```
tests/postgres/benchmarks/
├── __init__.py              # Package marker
├── README.md                # User guide
├── ARCHITECTURE.md          # This file
├── conftest.py              # Pytest fixtures
├── benchmark_setup.py       # Auto-initialization for dbtest
├── models.py                # Shared model definitions
├── bench_insert.py          # Insert benchmarks
├── bench_find.py            # Select/find benchmarks
└── bench_update.py          # Update/delete benchmarks

benchmarks/
└── bench_postgres_comparison.py  # Standalone comparison script
```

## Benchmark Registration Pattern

Each benchmark file uses the `BenchmarkGroup` pattern:

```python
from data_bridge.test import BenchmarkGroup, register_group

# Create group
insert_one = BenchmarkGroup("Insert One")

# Add implementations
@insert_one.add("data-bridge")
async def db_insert_one():
    user = DBUser(name="Test", email="test@test.com", age=30)
    await user.save()

@insert_one.add("asyncpg")
async def asyncpg_insert_one(asyncpg_pool):
    async with asyncpg_pool.acquire() as conn:
        await conn.execute(
            "INSERT INTO users (name, email, age) VALUES ($1, $2, $3)",
            "Test", "test@test.com", 30
        )

# Register group
register_group(insert_one)
```

## Fixture Scopes

### Session-Scoped (shared across all tests)
- `data_bridge_db`: data-bridge connection
- `asyncpg_pool`: asyncpg connection pool
- `psycopg2_conn`: psycopg2 connection pool
- `sqlalchemy_engine`: SQLAlchemy async engine

### Function-Scoped (fresh for each test)
- `sqlalchemy_session`: SQLAlchemy session
- `setup_tables`: Table creation and cleanup (autouse)

## Data Generation

Test data follows a consistent schema:

```python
{
    "name": "User{i}",
    "email": "user{i}@example.com",
    "age": 20 + (i % 50),          # Ages 20-69
    "city": ["NYC", "LA", "SF", ...][i % 5],
    "score": float(i * 1.5),
    "active": i % 2 == 0,          # 50% true, 50% false
}
```

This creates realistic distributions for benchmarking queries with filters.

## Performance Measurement

### Metrics

1. **Latency (ms)**: Time to complete operation
2. **Throughput (ops/sec)**: Operations per second
3. **Speedup**: Ratio vs baseline (SQLAlchemy or asyncpg)

### Baseline Selection

- **ORM operations**: Compare to SQLAlchemy
- **Raw queries**: Compare to asyncpg
- **Sync operations**: Compare to psycopg2

### Adaptive Iterations

Iterations scale based on batch size to keep test duration reasonable:

| Batch Size | Iterations | Rounds | Warmup |
|-----------|-----------|--------|--------|
| ≤100      | 50        | 5      | 3      |
| ≤1000     | 20        | 5      | 2      |
| ≤10000    | 10        | 3      | 1      |
| >10000    | 3         | 3      | 1      |

## Expected Performance Characteristics

### data-bridge Advantages

1. **Zero Python Byte Handling**: All PostgreSQL wire protocol in Rust
2. **GIL Release**: No GIL contention during I/O and serialization
3. **Parallel Processing**: Rayon for bulk operations (≥50 rows)
4. **Connection Pooling**: Tokio-based async pool in Rust
5. **Type Validation**: Compile-time + runtime safety without overhead

### Performance Targets

| Operation | vs SQLAlchemy | vs asyncpg | Rationale |
|-----------|--------------|------------|-----------|
| Insert 1000 | ≥2.0x | ≥1.3x | Parallel batch conversion |
| Select 1000 | ≥1.5x | ≥1.2x | Zero-copy deserialization |
| Update Many | ≥1.5x | ≥1.3x | GIL-free execution |
| Single ops | ≥1.3x | ≥1.1x | Reduced Python overhead |

## Integration with dbtest

The `dbtest` CLI can auto-discover and run these benchmarks:

```bash
# Run all PostgreSQL benchmarks
dbtest tests/postgres/benchmarks/

# Run specific file
dbtest tests/postgres/benchmarks/bench_insert.py

# Run specific group
dbtest tests/postgres/benchmarks/ -k "Insert One"
```

Output format:

```
============================================================
Benchmark: Insert One
============================================================

  data-bridge    :     0.85 ms  (2.72x faster)
  asyncpg        :     1.12 ms  (2.06x faster)
  psycopg2       :     1.45 ms  (1.59x faster)
  sqlalchemy     :     2.31 ms  (baseline)

Winner: data-bridge (2.72x faster than baseline)
```

## Adding New Benchmarks

### Step 1: Define Operation

Choose operation type:
- Insert/bulk insert
- Select/find (single, many, with filters)
- Update (single, bulk)
- Delete (single, bulk)
- Aggregation (count, sum, avg)
- Transaction (multi-op)

### Step 2: Create Benchmark Group

```python
from data_bridge.test import BenchmarkGroup, register_group

operation_name = BenchmarkGroup("Operation Name")
```

### Step 3: Implement for Each Framework

```python
@operation_name.add("data-bridge")
async def db_operation():
    # data-bridge implementation
    pass

@operation_name.add("asyncpg")
async def asyncpg_operation(asyncpg_pool):
    # asyncpg implementation
    pass

@operation_name.add("psycopg2")
def psycopg2_operation(psycopg2_conn):
    # psycopg2 implementation (sync)
    pass

if SQLALCHEMY_AVAILABLE:
    @operation_name.add("SQLAlchemy")
    async def sqlalchemy_operation(sqlalchemy_session):
        # SQLAlchemy implementation
        pass
```

### Step 4: Add Setup (if needed)

```python
@operation_name.add("data-bridge", setup="await _setup_db()")
async def db_operation():
    # ...

async def _setup_db():
    # Insert test data, etc.
    pass
```

### Step 5: Register Group

```python
register_group(operation_name)
```

## Troubleshooting

### Common Issues

1. **Connection refused**: PostgreSQL not running
   ```bash
   pg_isready  # Check status
   ```

2. **Permission denied**: User lacks privileges
   ```bash
   createuser -s postgres
   ```

3. **Import errors**: Missing dependencies
   ```bash
   uv pip install asyncpg psycopg2-binary "sqlalchemy[asyncio]"
   ```

4. **Fixture errors**: Scope mismatch
   - Use `function` scope for tests that modify data
   - Use `session` scope for expensive setup (connections)

### Performance Issues

1. **Unexpected slowness**: Check connection pooling
2. **High variance**: Increase warmup rounds
3. **Memory growth**: Ensure cleanup fixtures run
4. **GIL contention**: Verify async/await usage

## Future Enhancements

1. **Transaction Benchmarks**: Multi-operation transactions
2. **Complex Queries**: JOINs, subqueries, CTEs
3. **Prepared Statements**: Parameterized query performance
4. **Connection Pooling**: Pool size impact studies
5. **Memory Profiling**: Heap usage comparisons
6. **Concurrent Load**: Multi-threaded stress tests

## References

- MongoDB benchmarks: `tests/mongo/benchmarks/`
- data-bridge test framework: `python/data_bridge/test/`
- PostgreSQL API: `python/data_bridge/postgres/`
- Rust engine: `crates/data-bridge-postgres/`
