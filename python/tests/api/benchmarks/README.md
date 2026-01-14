# API Server Benchmarks

Comprehensive benchmarks comparing **data-bridge-api** vs **FastAPI + Uvicorn**.

## Overview

This benchmark suite validates performance claims against the industry baseline (FastAPI + Uvicorn) across three key areas:

1. **Throughput** - Requests per second
2. **Serialization** - JSON performance with different payload sizes
3. **Latency** - Response times under load (P50, P99)
4. **GIL Release** - Verification that Rust doesn't block Python threads

## Prerequisites

- Python 3.12+
- data-bridge-api installed (`maturin develop`)
- FastAPI installed (`pip install fastapi`)
- Uvicorn installed (`pip install uvicorn`)
- pytest installed (`pip install pytest`)

## Running Benchmarks

### Option 1: Rust Framework (Recommended)

Run benchmarks using the native data-bridge-test framework:

```bash
# Run all benchmarks with default settings (3 rounds)
uv run python tests/api/benchmarks/bench_comparison_rust.py

# Quick mode (1 round, no warmup) - faster for development
uv run python tests/api/benchmarks/bench_comparison_rust.py --quick

# Verbose mode (detailed statistics)
uv run python tests/api/benchmarks/bench_comparison_rust.py --verbose

# Custom rounds and warmup
uv run python tests/api/benchmarks/bench_comparison_rust.py --rounds 5 --warmup 2
```

**Advantages:**
- Uses pure Rust benchmark framework (no pytest-benchmark dependency)
- Consistent with other data-bridge benchmarks
- Better GIL release verification
- Standalone script (no pytest overhead)

### Option 2: pytest Integration

Run benchmarks through pytest (uses same Rust framework under the hood):

```bash
pytest tests/api/benchmarks/test_api_benchmarks.py -v
```

### Run Specific Benchmark

```bash
# Plaintext response (minimal overhead)
pytest tests/api/benchmarks/test_api_benchmarks.py::test_plaintext -v

# Path parameter extraction
pytest tests/api/benchmarks/test_api_benchmarks.py::test_path_params -v

# Serialization (different payload sizes)
pytest tests/api/benchmarks/test_api_benchmarks.py::test_serialize_small -v
pytest tests/api/benchmarks/test_api_benchmarks.py::test_serialize_medium -v
pytest tests/api/benchmarks/test_api_benchmarks.py::test_serialize_large -v
pytest tests/api/benchmarks/test_api_benchmarks.py::test_serialize_xlarge -v

# Latency (under different concurrent loads)
pytest tests/api/benchmarks/test_api_benchmarks.py::test_latency_100 -v
pytest tests/api/benchmarks/test_api_benchmarks.py::test_latency_1000 -v
pytest tests/api/benchmarks/test_api_benchmarks.py::test_latency_5000 -v

# GIL Release verification
pytest tests/api/benchmarks/test_api_benchmarks.py::test_gil_release -v
```

### Run Summary Comparison

```bash
pytest tests/api/benchmarks/test_api_benchmarks.py::test_summary -v
```

## Benchmark Structure

```
tests/api/benchmarks/
├── README.md                    # This file
├── conftest.py                  # pytest fixtures (server startup, HTTP client)
├── benchmark_setup.py           # Shared utilities and state management
├── bench_comparison_rust.py     # Standalone Rust framework runner (recommended)
├── run_benchmarks.py            # Alternative standalone runner
├── bench_throughput.py          # Throughput benchmarks
├── bench_serialization.py       # Serialization benchmarks
├── bench_latency.py             # Latency benchmarks (P50, P99)
├── bench_gil.py                 # GIL release verification
└── test_api_benchmarks.py       # pytest integration
```

## Performance Targets

Based on `openspec/changes/add-api-benchmarks/specs/api-server/spec.md`:

### Throughput
- **Target**: >1.5x FastAPI/Uvicorn baseline
- **Scenarios**:
  - GET /plaintext (minimal overhead)
  - GET /items/{id} (path parameter extraction)
  - GET /items/1 (JSON response)

### Serialization
- **Target**: ≥2x faster than standard `json` library
- **Payload Sizes**:
  - Small: 1KB
  - Medium: 10KB
  - Large: 100KB
  - XLarge: 1MB

## Output Format

Results are displayed in a comparison table:

```
Scenario             data-bridge      FastAPI          Speedup
----------------------------------------------------------------------
Plaintext            45000/s          30000/s          1.50x
Path Params          42000/s          28000/s          1.50x
JSON Response        40000/s          27000/s          1.48x
Serialize 1KB        35000/s          25000/s          1.40x
Serialize 10KB       30000/s          20000/s          1.50x
Serialize 100KB      15000/s          10000/s          1.50x
Serialize 1MB        2000/s           1000/s           2.00x
```

## Implementation Details

### Server Management

- Each framework runs on a dedicated port (data-bridge: 8001, FastAPI: 8002)
- Servers are started once per test session (session-scoped fixtures)
- Servers are automatically stopped after tests complete

### HTTP Client

- Uses Python `requests` library with session pooling
- Shared HTTP session across all tests for efficiency

### Benchmark Groups

Benchmarks use the `data-bridge.test.BenchmarkGroup` API:

```python
from data_bridge.test import BenchmarkGroup, register_group

group = BenchmarkGroup("Test Name")

@group.add("data-bridge")
def test_databridge():
    # Test implementation
    pass

@group.add("FastAPI")
def test_fastapi():
    # Test implementation
    pass

register_group(group)
```

## Troubleshooting

### Server fails to start

Check if ports 8001 and 8002 are available:

```bash
lsof -i :8001
lsof -i :8002
```

Kill processes if needed:

```bash
kill -9 <PID>
```

### Import errors

Ensure data-bridge is installed in development mode:

```bash
maturin develop
```

### Latency
- **Target**: Stable P99 latency under high concurrency (5000 clients)
- **Scenarios**:
  - 100 concurrent clients
  - 1000 concurrent clients
  - 5000 concurrent clients

### GIL Release
- **Target**: No Python thread starvation
- **Test**: 10 concurrent requests from Python threads should complete in <2 seconds

## Future Enhancements

- [ ] Add POST request benchmarks with body validation
- [ ] Add benchmark result persistence and trend analysis
- [ ] Add comparison with other frameworks (Sanic, Starlette)
- [ ] Add memory usage profiling
