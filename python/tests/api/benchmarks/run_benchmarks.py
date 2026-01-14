#!/usr/bin/env python3
"""
API Server Benchmarks Runner

Standalone runner for comparing data-bridge-api vs FastAPI performance.
Uses ouroboros.test framework directly.

Usage:
    python run_benchmarks.py              # Run all benchmarks
    python run_benchmarks.py --quick      # Quick mode (fewer iterations)
    python run_benchmarks.py --throughput # Only throughput tests
"""

import argparse
import asyncio
import subprocess
import sys
import time
import tempfile
from pathlib import Path
from typing import Optional

from ouroboros.http import HttpClient
from ouroboros.test import (
    run_benchmarks,
    print_comparison_table,
)

# Import benchmark groups
from bench_throughput import plaintext, path_params, json_response
# TODO: Create these benchmark files
# from bench_serialization import serialize_small, serialize_medium, serialize_large, serialize_xlarge
# from bench_latency import latency_100, latency_1000, latency_5000
# from bench_gil import gil_verification


# =====================
# Server Configuration
# =====================

DATA_BRIDGE_PORT = 8001
FASTAPI_PORT = 8002
STARTUP_TIMEOUT = 30


class ServerManager:
    """Manage test server lifecycle."""

    def __init__(self, name: str, port: int, script_content: str):
        self.name = name
        self.port = port
        self.script_content = script_content
        self.process: Optional[subprocess.Popen] = None
        self.script_file: Optional[Path] = None

    def start(self):
        """Start the server."""
        # Create temporary script
        fd, path = tempfile.mkstemp(suffix=".py", prefix=f"{self.name}_server_")
        self.script_file = Path(path)
        self.script_file.write_text(self.script_content)

        print(f"Starting {self.name} server on port {self.port}...")

        # Start process
        self.process = subprocess.Popen(
            [sys.executable, str(self.script_file)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )

        # Wait for ready
        if not self._wait_ready():
            stderr = self.process.stderr.read().decode() if self.process.stderr else ""
            raise RuntimeError(
                f"Failed to start {self.name} server on port {self.port}.\n"
                f"Stderr: {stderr}"
            )

        print(f"✓ {self.name} server ready at http://localhost:{self.port}")

    def _wait_ready(self) -> bool:
        """Wait for server to respond."""
        async def check():
            client = HttpClient(timeout=1.0)
            try:
                response = await client.get(f"http://localhost:{self.port}/health")
                return response.status_code == 200
            except:
                return False

        start = time.time()
        while time.time() - start < STARTUP_TIMEOUT:
            if asyncio.run(check()):
                return True
            time.sleep(0.1)
        return False

    def stop(self):
        """Stop the server."""
        if self.process:
            print(f"Stopping {self.name} server...")
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()

        # Cleanup script file
        if self.script_file and self.script_file.exists():
            self.script_file.unlink()

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.stop()


# =====================
# Server Scripts
# =====================

DATA_BRIDGE_SCRIPT = """
from ouroboros.api import App

app = App()

# Health check
@app.get("/health")
async def health():
    return {"status": "ok"}

# Plaintext response
@app.get("/plaintext")
async def plaintext():
    return "Hello, World!"

# Path parameters
@app.get("/items/{item_id}")
async def get_item(item_id: int):
    return {"item_id": item_id, "name": f"Item {item_id}"}

# JSON serialization (various sizes)
@app.get("/json/{size}")
async def get_json(size: int):
    items = []
    approx_item_size = 50
    num_items = max(1, size // approx_item_size)

    for i in range(num_items):
        items.append({
            "id": i,
            "name": f"Item {i}",
            "value": i * 1.5,
            "active": i % 2 == 0,
        })

    return {"items": items, "count": len(items)}

app.run(host="127.0.0.1", port=""" + str(DATA_BRIDGE_PORT) + """)
"""

FASTAPI_SCRIPT = """
from fastapi import FastAPI
import uvicorn

app = FastAPI()

# Health check
@app.get("/health")
async def health():
    return {"status": "ok"}

# Plaintext response
@app.get("/plaintext")
async def plaintext():
    return "Hello, World!"

# Path parameters
@app.get("/items/{item_id}")
async def get_item(item_id: int):
    return {"item_id": item_id, "name": f"Item {item_id}"}

# JSON serialization (various sizes)
@app.get("/json/{size}")
async def get_json(size: int):
    items = []
    approx_item_size = 50
    num_items = max(1, size // approx_item_size)

    for i in range(num_items):
        items.append({
            "id": i,
            "name": f"Item {i}",
            "value": i * 1.5,
            "active": i % 2 == 0,
        })

    return {"items": items, "count": len(items)}

uvicorn.run(app, host="127.0.0.1", port=""" + str(FASTAPI_PORT) + """, log_level="error")
"""


# =====================
# Main Runner
# =====================

parser = argparse.ArgumentParser(description="API Server Benchmarks")
parser.add_argument("--quick", action="store_true", help="Quick mode (fewer iterations)")
parser.add_argument("--throughput", action="store_true", help="Only run throughput tests")
parser.add_argument("--serialization", action="store_true", help="Only run serialization tests")
parser.add_argument("--latency", action="store_true", help="Only run latency tests")
parser.add_argument("--gil", action="store_true", help="Only run GIL tests")
args = parser.parse_args()

# Start servers
print("=" * 80)
print("API Server Benchmarks - data-bridge-api vs FastAPI")
print("=" * 80)
print()

with ServerManager("data-bridge", DATA_BRIDGE_PORT, DATA_BRIDGE_SCRIPT) as db_server, \
     ServerManager("FastAPI", FASTAPI_PORT, FASTAPI_SCRIPT) as fa_server:

    # Initialize benchmark_setup
    import benchmark_setup
    benchmark_setup.init_session(
        data_bridge_url=f"http://localhost:{DATA_BRIDGE_PORT}",
        fastapi_url=f"http://localhost:{FASTAPI_PORT}",
    )

    # Collect benchmark groups to run
    groups = []

    # Determine which groups to run
    run_all = not (args.throughput or args.serialization or args.latency or args.gil)

    if run_all or args.throughput:
        groups.extend([plaintext, path_params, json_response])

    # TODO: Uncomment when files are created
    # if run_all or args.serialization:
    #     groups.extend([serialize_small, serialize_medium, serialize_large, serialize_xlarge])
    #
    # if run_all or args.latency:
    #     groups.extend([latency_100, latency_1000, latency_5000])
    #
    # if run_all or args.gil:
    #     groups.append(gil_verification)

    # Run benchmarks
    print("\nRunning benchmarks...")
    print("=" * 80)

    all_results = []
    for i, group in enumerate(groups, 1):
        print(f"\n[{i}/{len(groups)}] {group.name}")
        print("-" * 80)

        results = run_benchmarks(group)
        all_results.append((group.name, results))

        # Print comparison for this group
        result_list = list(results.values())
        if len(result_list) >= 2:
            print_comparison_table(result_list, baseline="FastAPI")

    # Print summary
    print("\n" + "=" * 80)
    print("SUMMARY - All Benchmarks")
    print("=" * 80)

    print(f"\n{'Benchmark':<30} {'data-bridge':<15} {'FastAPI':<15} {'Speedup':<10}")
    print("-" * 80)

    for name, results in all_results:
        db_result = results.get("data-bridge")
        fa_result = results.get("FastAPI")

        if db_result and fa_result:
            speedup = db_result.ops_per_sec / fa_result.ops_per_sec
            print(
                f"{name:<30} "
                f"{db_result.ops_per_sec:>12.0f}/s "
                f"{fa_result.ops_per_sec:>12.0f}/s "
                f"{speedup:>8.2f}x"
            )

    print("=" * 80)
    print("✓ Benchmarks complete")
