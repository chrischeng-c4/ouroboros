---
name: bench
description: Run MongoDB benchmarks with dbtest
user-invocable: true
---

# /bench

Runs MongoDB benchmarks using the dbtest CLI tool.

## Usage

```bash
/bench [options]
```

## Options

- No arguments: Run all benchmarks in `python/tests/mongo/benchmarks/`
- `--verbose` or `-v`: Show detailed output
- `--pattern <pattern>`: Filter benchmark files (e.g., `bench_insert.py`)

## Examples

```bash
# Run all benchmarks
/bench

# Run with verbose output
/bench --verbose

# Run specific benchmark
/bench --pattern bench_insert.py
```

## Command

Run the following command:

```bash
uv run python -m ouroboros.qc.cli --root python/tests/mongo/benchmarks bench
```

For verbose mode:
```bash
uv run python -m ouroboros.qc.cli --root python/tests/mongo/benchmarks bench --verbose
```

## Prerequisites

- MongoDB must be running locally on `localhost:27017`
- Python dependencies installed via `uv`

## What it tests

Benchmarks compare performance between:
- **data-bridge**: The Rust-based MongoDB ODM
- **Beanie**: The Python-based async MongoDB ODM

Operations benchmarked:
- Insert (single and bulk)
- Find (one and many)
- Update (single and bulk)
- Delete
- Upsert
- Aggregation
- Count
