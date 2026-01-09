"""
API Server Benchmarks

Run benchmarks comparing data-bridge-api vs FastAPI + Uvicorn.

Usage:
    # Run all benchmarks
    pytest tests/api/benchmarks/test_api_benchmarks.py -v

    # Run specific test
    pytest tests/api/benchmarks/test_api_benchmarks.py::test_plaintext -v

    # Run with benchmark output
    pytest tests/api/benchmarks/test_api_benchmarks.py --benchmark
"""

import pytest
from data_bridge.test import run_benchmarks
from tests.api.benchmarks import benchmark_setup

# Import benchmark groups
from tests.api.benchmarks import bench_throughput
from tests.api.benchmarks import bench_serialization
from tests.api.benchmarks import bench_latency
from tests.api.benchmarks import bench_gil


# =====================
# Setup
# =====================

@pytest.fixture(scope="module", autouse=True)
def setup_benchmarks(data_bridge_server, fastapi_server, http_client):
    """Initialize benchmark setup with fixtures."""
    benchmark_setup.init_session(
        session=http_client,
        data_bridge_url=data_bridge_server,
        fastapi_url=fastapi_server,
    )


# =====================
# Throughput Benchmarks
# =====================

def test_plaintext(benchmark):
    """Benchmark: Plaintext response (minimal overhead)."""
    results = run_benchmarks(bench_throughput.plaintext)
    print("\nPlaintext Response Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


def test_path_params(benchmark):
    """Benchmark: Path parameter extraction."""
    results = run_benchmarks(bench_throughput.path_params)
    print("\nPath Parameters Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


def test_json_response(benchmark):
    """Benchmark: JSON response."""
    results = run_benchmarks(bench_throughput.json_response)
    print("\nJSON Response Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


# =====================
# Serialization Benchmarks
# =====================

def test_serialize_small(benchmark):
    """Benchmark: Serialize small payload (1KB)."""
    results = run_benchmarks(bench_serialization.serialize_small)
    print("\nSerialize Small (1KB) Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


def test_serialize_medium(benchmark):
    """Benchmark: Serialize medium payload (10KB)."""
    results = run_benchmarks(bench_serialization.serialize_medium)
    print("\nSerialize Medium (10KB) Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


def test_serialize_large(benchmark):
    """Benchmark: Serialize large payload (100KB)."""
    results = run_benchmarks(bench_serialization.serialize_large)
    print("\nSerialize Large (100KB) Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


def test_serialize_xlarge(benchmark):
    """Benchmark: Serialize extra large payload (1MB)."""
    results = run_benchmarks(bench_serialization.serialize_xlarge)
    print("\nSerialize XLarge (1MB) Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


# =====================
# Latency Benchmarks
# =====================

def test_latency_100(benchmark):
    """Benchmark: Latency under 100 concurrent connections."""
    results = run_benchmarks(bench_latency.latency_100)
    print("\nLatency (100 Concurrent) Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


def test_latency_1000(benchmark):
    """Benchmark: Latency under 1000 concurrent connections."""
    results = run_benchmarks(bench_latency.latency_1000)
    print("\nLatency (1000 Concurrent) Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


def test_latency_5000(benchmark):
    """Benchmark: Latency under 5000 concurrent connections."""
    results = run_benchmarks(bench_latency.latency_5000)
    print("\nLatency (5000 Concurrent) Benchmarks:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


# =====================
# GIL Verification
# =====================

def test_gil_release(benchmark):
    """Benchmark: Verify GIL is released during concurrent requests."""
    results = run_benchmarks(bench_gil.gil_verification)
    print("\nGIL Release Verification:")
    for framework, result in results.items():
        print(f"  {framework}: {result.ops_per_sec:.0f} ops/sec")


# =====================
# Comparison Summary
# =====================

def test_summary(benchmark):
    """Print summary comparison."""
    print("\n" + "=" * 70)
    print("API Server Benchmark Summary")
    print("=" * 70)

    # Run all benchmarks and collect results
    all_results = {}

    # Throughput
    all_results["Plaintext"] = run_benchmarks(bench_throughput.plaintext)
    all_results["Path Params"] = run_benchmarks(bench_throughput.path_params)
    all_results["JSON Response"] = run_benchmarks(bench_throughput.json_response)

    # Serialization
    all_results["Serialize 1KB"] = run_benchmarks(bench_serialization.serialize_small)
    all_results["Serialize 10KB"] = run_benchmarks(bench_serialization.serialize_medium)
    all_results["Serialize 100KB"] = run_benchmarks(bench_serialization.serialize_large)
    all_results["Serialize 1MB"] = run_benchmarks(bench_serialization.serialize_xlarge)

    # Latency
    all_results["Latency 100"] = run_benchmarks(bench_latency.latency_100)
    all_results["Latency 1000"] = run_benchmarks(bench_latency.latency_1000)
    all_results["Latency 5000"] = run_benchmarks(bench_latency.latency_5000)

    # GIL
    all_results["GIL Release"] = run_benchmarks(bench_gil.gil_verification)

    # Print comparison table
    print(f"\n{'Scenario':<20} {'data-bridge':<15} {'FastAPI':<15} {'Speedup':<10}")
    print("-" * 70)

    for scenario, results in all_results.items():
        db_ops = results.get("data-bridge", None)
        fa_ops = results.get("FastAPI", None)

        if db_ops and fa_ops:
            speedup = db_ops.ops_per_sec / fa_ops.ops_per_sec
            print(
                f"{scenario:<20} {db_ops.ops_per_sec:>12.0f}/s "
                f"{fa_ops.ops_per_sec:>12.0f}/s {speedup:>8.2f}x"
            )
