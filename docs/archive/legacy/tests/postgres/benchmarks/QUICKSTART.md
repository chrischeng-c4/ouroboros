# PostgreSQL Benchmarks - Quick Start Guide

## 1-Minute Setup

### Prerequisites

```bash
# 1. Ensure PostgreSQL is running
pg_isready

# 2. Create benchmark database
createdb data_bridge_benchmark

# 3. Install dependencies
uv sync
```

### Run Benchmarks

```bash
# Option A: Using dbtest (recommended)
POSTGRES_URI="postgresql://postgres:postgres@localhost:5432/data_bridge_benchmark" \
dbtest tests/postgres/benchmarks/

# Option B: Using standalone script
POSTGRES_URI="postgresql://postgres:postgres@localhost:5432/data_bridge_benchmark" \
uv run python benchmarks/bench_postgres_comparison.py

# Option C: Using pytest
POSTGRES_URI="postgresql://postgres:postgres@localhost:5432/data_bridge_benchmark" \
pytest tests/postgres/benchmarks/ -v
```

## What Gets Benchmarked?

### Operations
- **Insert One**: Single row insert
- **Insert Bulk**: 1000, 10000 rows
- **Select One**: Find by ID
- **Select Many**: Find 1000 rows with filter
- **Update One**: Update single row
- **Update Many**: Update multiple rows
- **Delete Many**: Delete multiple rows
- **Count**: Count with filter

### Frameworks
1. **data-bridge** - Our Rust-backed ORM
2. **asyncpg** - High-performance async driver
3. **psycopg2** - Traditional sync driver
4. **SQLAlchemy** - Popular ORM

## Expected Results

You should see output like:

```
============================================================
PostgreSQL Performance Benchmarks
============================================================

Benchmarking: Insert One (100 iterations)
  data_bridge    :     0.85 ms
  asyncpg        :     1.12 ms
  psycopg2       :     1.45 ms
  sqlalchemy     :     2.31 ms

Benchmarking: Bulk Insert 1000 rows
  data_bridge    :    15.32 ms
  asyncpg        :    18.45 ms
  psycopg2       :    24.56 ms
  sqlalchemy     :    45.67 ms

============================================================
Benchmark Complete
============================================================
```

## Performance Targets

| Operation | Target |
|-----------|--------|
| vs SQLAlchemy | ≥2.0x faster |
| vs asyncpg | ≥1.3x faster |
| vs psycopg2 | ≥1.5x faster |

## Troubleshooting

### Connection Issues

```bash
# Check PostgreSQL status
pg_isready

# Verify connection
psql -U postgres -d data_bridge_benchmark -c "SELECT version();"

# If connection fails, check:
# 1. PostgreSQL is running: sudo systemctl start postgresql
# 2. User exists: createuser -s postgres
# 3. Database exists: createdb data_bridge_benchmark
```

### Missing Dependencies

```bash
# Install benchmark dependencies
uv pip install asyncpg psycopg2-binary "sqlalchemy[asyncio]"
```

### Permission Denied

```bash
# Grant permissions
psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE data_bridge_benchmark TO postgres;"
```

## Next Steps

- Read [README.md](README.md) for detailed documentation
- See [ARCHITECTURE.md](ARCHITECTURE.md) for design details
- Add new benchmarks following existing patterns
- Compare results with MongoDB benchmarks in `tests/mongo/benchmarks/`

## Quick Commands

```bash
# Run single benchmark file
dbtest tests/postgres/benchmarks/bench_insert.py

# Run specific benchmark group
dbtest tests/postgres/benchmarks/ -k "Insert One"

# Run with custom connection
POSTGRES_URI="postgresql://myuser:mypass@remotehost:5432/mydb" \
dbtest tests/postgres/benchmarks/

# Generate comparison report
uv run python benchmarks/bench_postgres_comparison.py > results.txt
```
