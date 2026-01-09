"""Latency benchmarks for API servers under load."""

import subprocess
import statistics
import time
import concurrent.futures
from typing import Dict, List
from data_bridge.test import BenchmarkGroup, register_group
from tests.api.benchmarks import benchmark_setup
from tests.api.benchmarks.conftest import CONCURRENCY_LEVELS


def check_wrk_available() -> bool:
    """Check if wrk is installed."""
    try:
        subprocess.run(["wrk", "--version"], capture_output=True, check=True)
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def run_wrk_latency(url: str, connections: int, duration: int = 10) -> Dict:
    """Run wrk and extract latency percentiles."""
    try:
        cmd = [
            "wrk",
            "-t", "4",
            "-c", str(connections),
            "-d", f"{duration}s",
            "--latency",
            url
        ]

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=duration + 5)
        output = result.stdout

        stats = {}
        for line in output.split("\n"):
            if "50.000%" in line or "50%" in line:
                stats["p50"] = line.split()[1]
            elif "99.000%" in line or "99%" in line:
                stats["p99"] = line.split()[1]

        return stats
    except Exception as e:
        return {"error": str(e)}


def run_python_latency(framework: str, endpoint: str, connections: int, requests_per_thread: int = 10) -> Dict:
    """Fallback: Pure Python latency measurement using ThreadPoolExecutor."""
    latencies = []

    def make_request():
        start = time.perf_counter()
        try:
            response = benchmark_setup.make_request(framework, endpoint)
            elapsed = (time.perf_counter() - start) * 1000  # Convert to ms
            if response.status_code == 200:
                return elapsed
        except Exception:
            pass
        return None

    # Use thread pool to simulate concurrent connections
    with concurrent.futures.ThreadPoolExecutor(max_workers=connections) as executor:
        futures = [executor.submit(make_request) for _ in range(connections * requests_per_thread)]
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            if result is not None:
                latencies.append(result)

    if not latencies:
        return {"error": "No successful requests"}

    latencies.sort()
    return {
        "p50": f"{statistics.median(latencies):.2f}ms",
        "p99": f"{latencies[int(len(latencies) * 0.99)]:.2f}ms" if len(latencies) > 1 else f"{latencies[0]:.2f}ms",
        "mean": f"{statistics.mean(latencies):.2f}ms",
        "samples": len(latencies)
    }


# =====================
# Latency - Low Concurrency (100)
# =====================

latency_100 = BenchmarkGroup("Latency 100 Concurrent")


@latency_100.add("data-bridge")
def db_latency_100():
    """Measure latency at 100 concurrent connections (data-bridge)."""
    base_url = benchmark_setup.get_data_bridge_url()

    if check_wrk_available():
        stats = run_wrk_latency(f"{base_url}/plaintext", connections=100)
    else:
        stats = run_python_latency("data-bridge", "/plaintext", connections=100)

    # Store results for comparison
    return stats


@latency_100.add("FastAPI")
def fastapi_latency_100():
    """Measure latency at 100 concurrent connections (FastAPI)."""
    base_url = benchmark_setup.get_fastapi_url()

    if check_wrk_available():
        stats = run_wrk_latency(f"{base_url}/plaintext", connections=100)
    else:
        stats = run_python_latency("fastapi", "/plaintext", connections=100)

    return stats


register_group(latency_100)


# =====================
# Latency - Medium Concurrency (1000)
# =====================

latency_1000 = BenchmarkGroup("Latency 1000 Concurrent")


@latency_1000.add("data-bridge")
def db_latency_1000():
    """Measure latency at 1000 concurrent connections (data-bridge)."""
    base_url = benchmark_setup.get_data_bridge_url()

    if check_wrk_available():
        stats = run_wrk_latency(f"{base_url}/plaintext", connections=1000)
    else:
        stats = run_python_latency("data-bridge", "/plaintext", connections=100, requests_per_thread=10)

    return stats


@latency_1000.add("FastAPI")
def fastapi_latency_1000():
    """Measure latency at 1000 concurrent connections (FastAPI)."""
    base_url = benchmark_setup.get_fastapi_url()

    if check_wrk_available():
        stats = run_wrk_latency(f"{base_url}/plaintext", connections=1000)
    else:
        stats = run_python_latency("fastapi", "/plaintext", connections=100, requests_per_thread=10)

    return stats


register_group(latency_1000)


# =====================
# Latency - High Concurrency (5000)
# =====================

latency_5000 = BenchmarkGroup("Latency 5000 Concurrent")


@latency_5000.add("data-bridge")
def db_latency_5000():
    """Measure latency at 5000 concurrent connections (data-bridge)."""
    base_url = benchmark_setup.get_data_bridge_url()

    if check_wrk_available():
        stats = run_wrk_latency(f"{base_url}/plaintext", connections=5000, duration=15)
    else:
        # For high concurrency, use more threads and requests
        stats = run_python_latency("data-bridge", "/plaintext", connections=200, requests_per_thread=25)

    return stats


@latency_5000.add("FastAPI")
def fastapi_latency_5000():
    """Measure latency at 5000 concurrent connections (FastAPI)."""
    base_url = benchmark_setup.get_fastapi_url()

    if check_wrk_available():
        stats = run_wrk_latency(f"{base_url}/plaintext", connections=5000, duration=15)
    else:
        stats = run_python_latency("fastapi", "/plaintext", connections=200, requests_per_thread=25)

    return stats


register_group(latency_5000)
