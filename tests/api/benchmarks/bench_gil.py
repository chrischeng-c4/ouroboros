"""GIL Release verification benchmarks for API servers."""

import threading
import time
from data_bridge.test import BenchmarkGroup, register_group
from tests.api.benchmarks import benchmark_setup


# =====================
# GIL Release Verification
# =====================

gil_verification = BenchmarkGroup("GIL Release")


@gil_verification.add("data-bridge")
def db_gil_release():
    """Verify GIL is released during concurrent requests (data-bridge)."""
    completed = []

    def make_request():
        try:
            response = benchmark_setup.make_request("data-bridge", "/plaintext")
            if response.status_code == 200:
                completed.append(1)
        except Exception:
            pass

    # Launch 10 concurrent threads
    threads = []
    start = time.time()
    for _ in range(10):
        t = threading.Thread(target=make_request)
        t.start()
        threads.append(t)

    for t in threads:
        t.join()

    elapsed = time.time() - start

    # If GIL is properly released, concurrent requests should complete quickly
    # If GIL is held, this would take ~10x the time of a single request
    assert len(completed) == 10, f"Only {len(completed)}/10 requests succeeded"
    assert elapsed < 2.0, f"GIL may not be released: {elapsed:.2f}s for 10 concurrent requests"


@gil_verification.add("FastAPI")
def fastapi_gil_release():
    """Verify GIL behavior with FastAPI."""
    completed = []

    def make_request():
        try:
            response = benchmark_setup.make_request("fastapi", "/plaintext")
            if response.status_code == 200:
                completed.append(1)
        except Exception:
            pass

    # Launch 10 concurrent threads
    threads = []
    start = time.time()
    for _ in range(10):
        t = threading.Thread(target=make_request)
        t.start()
        threads.append(t)

    for t in threads:
        t.join()

    elapsed = time.time() - start

    assert len(completed) == 10, f"Only {len(completed)}/10 requests succeeded"
    # FastAPI baseline for comparison


register_group(gil_verification)
