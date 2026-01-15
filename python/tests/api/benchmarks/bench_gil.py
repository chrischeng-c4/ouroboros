"""GIL Release verification benchmarks for API servers."""

import asyncio
import time
from ouroboros.qc import BenchmarkGroup, register_group
from . import benchmark_setup


# =====================
# GIL Release Verification
# =====================

gil_verification = BenchmarkGroup("GIL Release")


@gil_verification.add("data-bridge")
async def db_gil_release():
    """Verify GIL is released during concurrent requests (data-bridge)."""

    async def make_request():
        try:
            response = await benchmark_setup.make_request("data-bridge", "/plaintext")
            return response.status_code == 200
        except Exception:
            return False

    # Launch 10 concurrent requests
    start = time.time()
    results = await asyncio.gather(*[make_request() for _ in range(10)])
    elapsed = time.time() - start

    completed = sum(1 for r in results if r)

    # If GIL is properly released, concurrent requests should complete quickly
    # If GIL is held, this would take ~10x the time of a single request
    assert completed == 10, f"Only {completed}/10 requests succeeded"
    assert elapsed < 2.0, f"GIL may not be released: {elapsed:.2f}s for 10 concurrent requests"


@gil_verification.add("FastAPI")
async def fastapi_gil_release():
    """Verify GIL behavior with FastAPI."""

    async def make_request():
        try:
            response = await benchmark_setup.make_request("fastapi", "/plaintext")
            return response.status_code == 200
        except Exception:
            return False

    # Launch 10 concurrent requests
    start = time.time()
    results = await asyncio.gather(*[make_request() for _ in range(10)])
    elapsed = time.time() - start

    completed = sum(1 for r in results if r)

    assert completed == 10, f"Only {completed}/10 requests succeeded"
    # FastAPI baseline for comparison


register_group(gil_verification)
