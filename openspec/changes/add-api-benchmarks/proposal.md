# Change: Add API Benchmarks

## Why
The `data-bridge-api` component lacks a comprehensive benchmark suite to validate its performance claims against the industry baseline (FastAPI + Uvicorn). To ensure the goal of solving the "Python serialization bottleneck" is met, we need standardized metrics for throughput, latency, and serialization speed.

## What Changes
- Adds a new benchmark suite (`benchmarks/bench_api.py`) targeting the API server.
- Establishes performance requirements for Throughput, Latency, and Serialization.
- Verifies GIL release during heavy operations.

## Impact
- Affected specs: `api-server`
- Affected code: `benchmarks/`
