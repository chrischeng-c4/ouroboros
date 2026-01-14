"""
API Server Benchmarks

Run benchmarks comparing data-bridge-api vs FastAPI + Uvicorn.

Usage:
    pytest tests/api/benchmarks/test_api_benchmarks.py -v
"""

import asyncio
import pytest
from tests.api.benchmarks import benchmark_setup
from tests.api.benchmarks import bench_throughput
from tests.api.benchmarks import bench_serialization
from tests.api.benchmarks import bench_latency
from tests.api.benchmarks import bench_gil


@pytest.fixture(scope="module", autouse=True)
def setup_benchmarks(data_bridge_server, fastapi_server):
    """Initialize benchmark setup with fixtures."""
    benchmark_setup.init_session(
        data_bridge_url=data_bridge_server,
        fastapi_url=fastapi_server
    )


def run_group_sync(group, rounds=3, warmup=1):
    """Run a benchmark group synchronously and return results as dict."""
    async def _run():
        return await group.run(rounds=rounds, warmup=warmup)

    results = asyncio.run(_run())
    # Convert to dict keyed by name
    return {r.name: r for r in results}


# =====================
# Throughput Benchmarks
# =====================

def test_plaintext():
    """Benchmark: Plaintext response (minimal overhead)."""
    results = run_group_sync(bench_throughput.plaintext)
    print("\nPlaintext Response Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


def test_path_params():
    """Benchmark: Path parameter extraction."""
    results = run_group_sync(bench_throughput.path_params)
    print("\nPath Parameters Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


def test_json_response():
    """Benchmark: JSON response."""
    results = run_group_sync(bench_throughput.json_response)
    print("\nJSON Response Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


# =====================
# Serialization Benchmarks
# =====================

def test_serialize_small():
    """Benchmark: Serialize small payload (1KB)."""
    results = run_group_sync(bench_serialization.serialize_small)
    print("\nSerialize Small (1KB) Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


def test_serialize_medium():
    """Benchmark: Serialize medium payload (10KB)."""
    results = run_group_sync(bench_serialization.serialize_medium)
    print("\nSerialize Medium (10KB) Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


def test_serialize_large():
    """Benchmark: Serialize large payload (100KB)."""
    results = run_group_sync(bench_serialization.serialize_large)
    print("\nSerialize Large (100KB) Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


def test_serialize_xlarge():
    """Benchmark: Serialize extra large payload (1MB)."""
    results = run_group_sync(bench_serialization.serialize_xlarge)
    print("\nSerialize XLarge (1MB) Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


# =====================
# Latency Benchmarks
# =====================

def test_latency_100():
    """Benchmark: Latency under 100 concurrent connections."""
    results = run_group_sync(bench_latency.latency_100)
    print("\nLatency (100 Concurrent) Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


def test_latency_1000():
    """Benchmark: Latency under 1000 concurrent connections."""
    results = run_group_sync(bench_latency.latency_1000)
    print("\nLatency (1000 Concurrent) Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


def test_latency_5000():
    """Benchmark: Latency under 5000 concurrent connections."""
    results = run_group_sync(bench_latency.latency_5000)
    print("\nLatency (5000 Concurrent) Benchmarks:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


# =====================
# GIL Verification
# =====================

def test_gil_release():
    """Benchmark: Verify GIL is released during concurrent requests."""
    results = run_group_sync(bench_gil.gil_verification)
    print("\nGIL Release Verification:")
    for name, result in results.items():
        print(f"  {name}: {result.stats.ops_per_second():.0f} ops/sec")


# =====================
# Comparison Summary
# =====================

def test_summary():
    """Print summary comparison."""
    print("\n" + "=" * 70)
    print("API Server Benchmark Summary")
    print("=" * 70)

    all_results = {
        "Plaintext": run_group_sync(bench_throughput.plaintext),
        "Path Params": run_group_sync(bench_throughput.path_params),
        "JSON Response": run_group_sync(bench_throughput.json_response),
        "Serialize 1KB": run_group_sync(bench_serialization.serialize_small),
        "Serialize 10KB": run_group_sync(bench_serialization.serialize_medium),
        "Serialize 100KB": run_group_sync(bench_serialization.serialize_large),
        "Serialize 1MB": run_group_sync(bench_serialization.serialize_xlarge),
        "Latency 100": run_group_sync(bench_latency.latency_100),
        "Latency 1000": run_group_sync(bench_latency.latency_1000),
        "Latency 5000": run_group_sync(bench_latency.latency_5000),
        "GIL Release": run_group_sync(bench_gil.gil_verification),
    }

    print(f"\n{'Scenario':<20} {'data-bridge':<15} {'FastAPI':<15} {'Speedup':<10}")
    print("-" * 70)

    for scenario, results in all_results.items():
        db_result = results.get("data-bridge")
        fa_result = results.get("FastAPI")

        if db_result and fa_result:
            db_ops = db_result.stats.ops_per_second()
            fa_ops = fa_result.stats.ops_per_second()
            speedup = db_ops / fa_ops if fa_ops > 0 else 0
            print(f"{scenario:<20} {db_ops:>12.0f}/s {fa_ops:>12.0f}/s {speedup:>8.2f}x")
