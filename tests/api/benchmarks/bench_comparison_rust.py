#!/usr/bin/env python3
"""
API Server Comparison Benchmarks (Rust Framework)

Standalone benchmark comparing data-bridge-api vs FastAPI using the
data_bridge.test framework (pure Rust benchmark implementation).

This replaces pytest-benchmark with our native Rust-backed framework for:
- Consistent benchmark methodology
- Better GIL release verification
- Integration with data-bridge ecosystem

Usage:
    python bench_comparison_rust.py              # Run all benchmarks
    python bench_comparison_rust.py --quick      # Quick mode (fewer rounds)
    python bench_comparison_rust.py --verbose    # Detailed output
"""

import argparse
import asyncio
import subprocess
import sys
import time
import tempfile
from pathlib import Path
from typing import Optional, Dict, List

from data_bridge.http import HttpClient
from data_bridge.test import BenchmarkGroup, register_group

# =====================
# Server Configuration
# =====================

DATA_BRIDGE_PORT = 8001
FASTAPI_PORT = 8002
STARTUP_TIMEOUT = 30

# Global URL storage
_data_bridge_url: Optional[str] = None
_fastapi_url: Optional[str] = None


def init_urls(data_bridge_url: str, fastapi_url: str):
    """Initialize global URLs for benchmark functions."""
    global _data_bridge_url, _fastapi_url
    _data_bridge_url = data_bridge_url
    _fastapi_url = fastapi_url


def get_data_bridge_url() -> str:
    """Get data-bridge-api base URL."""
    if _data_bridge_url is None:
        raise RuntimeError("data-bridge URL not initialized")
    return _data_bridge_url


def get_fastapi_url() -> str:
    """Get FastAPI base URL."""
    if _fastapi_url is None:
        raise RuntimeError("FastAPI URL not initialized")
    return _fastapi_url


async def make_request(framework: str, endpoint: str, method: str = "GET", **kwargs):
    """
    Make HTTP request to the specified framework.

    Args:
        framework: "data-bridge" or "fastapi"
        endpoint: API endpoint (e.g., "/plaintext")
        method: HTTP method
        **kwargs: Additional arguments passed to HttpClient

    Returns:
        HttpResponse object
    """
    base_url = get_data_bridge_url() if framework == "data-bridge" else get_fastapi_url()
    client = HttpClient(base_url=base_url)

    if method == "GET":
        return await client.get(endpoint, **kwargs)
    elif method == "POST":
        return await client.post(endpoint, **kwargs)
    elif method == "PUT":
        return await client.put(endpoint, **kwargs)
    elif method == "DELETE":
        return await client.delete(endpoint, **kwargs)
    else:
        raise ValueError(f"Unsupported HTTP method: {method}")


# =====================
# Server Management
# =====================

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

DATA_BRIDGE_SCRIPT = f"""
from data_bridge.api import App

app = App()

@app.get("/health")
async def health(request):
    return {{"status": "ok"}}

@app.get("/plaintext")
async def plaintext(request):
    return "Hello, World!"

@app.get("/json")
async def json_response(request):
    return {{
        "message": "Hello, World!",
        "status": "success",
        "timestamp": 1234567890,
        "data": {{"key": "value"}}
    }}

@app.get("/users/{{user_id}}")
async def get_user(request):
    user_id = int(request["path_params"]["user_id"])
    return {{
        "id": user_id,
        "name": f"User {{user_id}}",
        "email": f"user{{user_id}}@example.com",
        "active": True
    }}

if __name__ == "__main__":
    app.run(host="127.0.0.1", port={DATA_BRIDGE_PORT})
"""

FASTAPI_SCRIPT = f"""
from fastapi import FastAPI
import uvicorn

app = FastAPI()

@app.get("/health")
async def health():
    return {{"status": "ok"}}

@app.get("/plaintext")
async def plaintext():
    return "Hello, World!"

@app.get("/json")
async def json_response():
    return {{
        "message": "Hello, World!",
        "status": "success",
        "timestamp": 1234567890,
        "data": {{"key": "value"}}
    }}

@app.get("/users/{{user_id}}")
async def get_user(user_id: int):
    return {{
        "id": user_id,
        "name": f"User {{user_id}}",
        "email": f"user{{user_id}}@example.com",
        "active": True
    }}

if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port={FASTAPI_PORT}, log_level="error")
"""


# =====================
# Benchmark Groups
# =====================

# Group 1: Plaintext Response (Minimal Overhead)
plaintext_group = BenchmarkGroup("Plaintext Response")


@plaintext_group.add("data-bridge")
async def db_plaintext():
    """GET /plaintext with data-bridge-api."""
    response = await make_request("data-bridge", "/plaintext")
    assert response.status_code == 200
    text = response.text()
    # data-bridge returns plain text
    assert "Hello, World!" in text


@plaintext_group.add("FastAPI")
async def fastapi_plaintext():
    """GET /plaintext with FastAPI + Uvicorn."""
    response = await make_request("fastapi", "/plaintext")
    assert response.status_code == 200
    # FastAPI returns JSON-encoded string
    text = response.text()
    assert "Hello, World!" in text


register_group(plaintext_group)


# Group 2: JSON Response
json_group = BenchmarkGroup("JSON Response")


@json_group.add("data-bridge")
async def db_json():
    """GET /json with data-bridge-api."""
    response = await make_request("data-bridge", "/json")
    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Hello, World!"
    assert data["status"] == "success"


@json_group.add("FastAPI")
async def fastapi_json():
    """GET /json with FastAPI + Uvicorn."""
    response = await make_request("fastapi", "/json")
    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Hello, World!"
    assert data["status"] == "success"


register_group(json_group)


# Group 3: Path Parameters
path_params_group = BenchmarkGroup("Path Parameters")


@path_params_group.add("data-bridge")
async def db_path_params():
    """GET /users/{id} with data-bridge-api."""
    response = await make_request("data-bridge", "/users/42")
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == 42
    assert data["name"] == "User 42"


@path_params_group.add("FastAPI")
async def fastapi_path_params():
    """GET /users/{id} with FastAPI + Uvicorn."""
    response = await make_request("fastapi", "/users/42")
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == 42
    assert data["name"] == "User 42"


register_group(path_params_group)


# =====================
# Benchmark Runner
# =====================

async def run_all_benchmarks(rounds: int = 3, warmup: int = 1, verbose: bool = False):
    """
    Run all benchmark groups and print comparison.

    Args:
        rounds: Number of benchmark rounds
        warmup: Number of warmup rounds
        verbose: Show detailed statistics
    """
    groups = [plaintext_group, json_group, path_params_group]

    print("\n" + "=" * 80)
    print("Running benchmarks...")
    print("=" * 80)

    all_results = []

    for i, group in enumerate(groups, 1):
        print(f"\n[{i}/{len(groups)}] {group.name}")
        print("-" * 80)

        # Run the group
        results = await group.run(rounds=rounds, warmup=warmup)

        # Convert to dict for easier lookup
        result_dict = {r.name: r for r in results}
        all_results.append((group.name, result_dict))

        # Print individual results
        for result in results:
            if verbose:
                print(f"  {result.name}:")
                print(f"    Ops/sec:  {result.stats.ops_per_second():>12,.0f}")
                print(f"    Mean:     {result.stats.mean_ms:>12,.3f} ms")
                print(f"    Std Dev:  {result.stats.stddev_ms:>12,.3f} ms")
                print(f"    Min:      {result.stats.min_ms:>12,.3f} ms")
                print(f"    Max:      {result.stats.max_ms:>12,.3f} ms")
            else:
                print(f"  {result.name}: {result.stats.ops_per_second():>12,.0f} ops/sec")

        # Print comparison for this group
        if "data-bridge" in result_dict and "FastAPI" in result_dict:
            db_ops = result_dict["data-bridge"].stats.ops_per_second()
            fa_ops = result_dict["FastAPI"].stats.ops_per_second()
            speedup = db_ops / fa_ops if fa_ops > 0 else 0
            if speedup > 1:
                print(f"\n  → Speedup: {speedup:.2f}x faster")
            elif speedup > 0:
                print(f"\n  → Slowdown: {1/speedup:.2f}x slower")
            else:
                print(f"\n  → Unable to calculate speedup")

    # Print summary table
    print("\n" + "=" * 80)
    print("SUMMARY - All Benchmarks")
    print("=" * 80)

    print(f"\n{'Benchmark':<30} {'data-bridge':<15} {'FastAPI':<15} {'Speedup':<10}")
    print("-" * 80)

    for name, results in all_results:
        db_result = results.get("data-bridge")
        fa_result = results.get("FastAPI")

        if db_result and fa_result:
            db_ops = db_result.stats.ops_per_second()
            fa_ops = fa_result.stats.ops_per_second()
            speedup = db_ops / fa_ops if fa_ops > 0 else 0
            print(
                f"{name:<30} "
                f"{db_ops:>12,.0f}/s "
                f"{fa_ops:>12,.0f}/s "
                f"{speedup:>8.2f}x"
            )

    print("=" * 80)
    print("✓ Benchmarks complete")


# =====================
# Main Entry Point
# =====================

def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="API Server Comparison Benchmarks (Rust Framework)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python bench_comparison_rust.py              # Run with default settings (3 rounds)
  python bench_comparison_rust.py --quick      # Quick mode (1 round, no warmup)
  python bench_comparison_rust.py --verbose    # Show detailed statistics
  python bench_comparison_rust.py --rounds 5   # Run 5 rounds per benchmark
        """
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Quick mode (1 round, no warmup)"
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Show detailed statistics (mean, stddev, min, max)"
    )
    parser.add_argument(
        "--rounds",
        type=int,
        default=3,
        help="Number of benchmark rounds (default: 3)"
    )
    parser.add_argument(
        "--warmup",
        type=int,
        default=1,
        help="Number of warmup rounds (default: 1)"
    )

    args = parser.parse_args()

    # Adjust settings for quick mode
    if args.quick:
        args.rounds = 1
        args.warmup = 0

    # Print header
    print("=" * 80)
    print("API Server Comparison Benchmarks")
    print("data-bridge-api vs FastAPI + Uvicorn")
    print("=" * 80)
    print(f"\nConfiguration:")
    print(f"  Rounds:  {args.rounds}")
    print(f"  Warmup:  {args.warmup}")
    print(f"  Verbose: {args.verbose}")
    print()

    # Start servers
    try:
        with ServerManager("data-bridge", DATA_BRIDGE_PORT, DATA_BRIDGE_SCRIPT) as db_server, \
             ServerManager("FastAPI", FASTAPI_PORT, FASTAPI_SCRIPT) as fa_server:

            # Initialize URLs
            init_urls(
                data_bridge_url=f"http://localhost:{DATA_BRIDGE_PORT}",
                fastapi_url=f"http://localhost:{FASTAPI_PORT}",
            )

            # Run benchmarks
            asyncio.run(run_all_benchmarks(
                rounds=args.rounds,
                warmup=args.warmup,
                verbose=args.verbose
            ))

    except KeyboardInterrupt:
        print("\n\nBenchmark interrupted by user")
        sys.exit(1)
    except Exception as e:
        import traceback
        print(f"\n\nError: {e}")
        print("\nFull traceback:")
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
