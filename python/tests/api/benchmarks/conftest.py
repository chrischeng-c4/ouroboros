"""
Benchmark fixtures for API framework comparison.

Provides server setup and HTTP client fixtures for performance testing across
data-bridge-api and FastAPI.
"""

import pytest
import subprocess
import sys
import time
from typing import Dict
import asyncio
from data_bridge.http import HttpClient

# =====================
# Constants
# =====================

DATA_BRIDGE_PORT = 8001
FASTAPI_PORT = 8002

# Concurrency levels for latency tests
CONCURRENCY_LEVELS = [100, 1000, 5000]

# Payload sizes for serialization tests
PAYLOAD_SIZES = {
    "small": 1024,      # 1KB
    "medium": 10240,    # 10KB
    "large": 102400,    # 100KB
    "xlarge": 1048576,  # 1MB
}

# =====================
# Server Management
# =====================

class ServerProcess:
    """Manage a test server process."""

    def __init__(self, port: int, script_path: str):
        self.port = port
        self.script_path = script_path
        self.process = None

    def start(self):
        """Start the server process."""
        self.process = subprocess.Popen(
            [sys.executable, self.script_path],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )

    def stop(self):
        """Stop the server process."""
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()

    def wait_ready(self, timeout=30):
        """Wait for server to become ready by polling /health endpoint."""
        start = time.time()
        while time.time() - start < timeout:
            try:
                async def check():
                    client = HttpClient()
                    response = await client.get(f"http://localhost:{self.port}/health", timeout=1.0)
                    return response.status_code == 200

                if asyncio.run(check()):
                    return True
            except Exception:
                pass
            time.sleep(0.1)
        return False


# =====================
# data-bridge-api Server
# =====================

@pytest.fixture(scope="session")
def data_bridge_server():
    """Start data-bridge-api server (session-scoped)."""
    # Create server script
    server_script = f"""
from data_bridge.api import App

app = App()

@app.get("/plaintext")
async def plaintext(request):
    return "Hello, World!"

@app.get("/items/{{item_id}}")
async def get_item(request):
    item_id = int(request["path_params"]["item_id"])
    return {{"item_id": item_id, "name": f"Item {{item_id}}"}}

@app.post("/items")
async def create_item(request):
    item = request.get("body", {{}})
    return {{"id": 1, **item}}

@app.get("/json/{{size}}")
async def json_response(request):
    size = int(request["path_params"]["size"])
    return {{"data": "x" * size}}

@app.get("/health")
async def health(request):
    return {{"status": "ok"}}

if __name__ == "__main__":
    app.run(host="0.0.0.0", port={DATA_BRIDGE_PORT})
"""

    with open("/tmp/bench_databridge_server.py", "w") as f:
        f.write(server_script)

    server = ServerProcess(DATA_BRIDGE_PORT, "/tmp/bench_databridge_server.py")
    server.start()

    if not server.wait_ready():
        server.stop()
        pytest.fail("data-bridge-api server failed to start")

    yield f"http://localhost:{DATA_BRIDGE_PORT}"

    server.stop()


# =====================
# FastAPI Server
# =====================

@pytest.fixture(scope="session")
def fastapi_server():
    """Start FastAPI server (session-scoped)."""
    # Create server script
    server_script = f"""
from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class Item(BaseModel):
    name: str
    price: float

@app.get("/plaintext")
async def plaintext():
    return "Hello, World!"

@app.get("/items/{{item_id}}")
async def get_item(item_id: int):
    return {{"item_id": item_id, "name": f"Item {{item_id}}"}}

@app.post("/items")
async def create_item(item: Item):
    return {{"id": 1, **item.dict()}}

@app.get("/json/{{size}}")
async def json_response(size: int):
    return {{"data": "x" * size}}

@app.get("/health")
async def health():
    return {{"status": "ok"}}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port={FASTAPI_PORT}, log_level="error")
"""

    with open("/tmp/bench_fastapi_server.py", "w") as f:
        f.write(server_script)

    server = ServerProcess(FASTAPI_PORT, "/tmp/bench_fastapi_server.py")
    server.start()

    if not server.wait_ready():
        server.stop()
        pytest.fail("FastAPI server failed to start")

    yield f"http://localhost:{FASTAPI_PORT}"

    server.stop()


# =====================
# Benchmark Configuration
# =====================

def get_benchmark_params(scenario: str) -> Dict[str, int]:
    """
    Calculate adaptive benchmark parameters based on scenario.

    Args:
        scenario: Benchmark scenario name

    Returns:
        Dictionary with 'iterations', 'rounds', and 'warmup_rounds' keys
    """
    if scenario == "throughput":
        iterations = 100
        rounds = 5
        warmup_rounds = 3
    elif scenario == "latency":
        iterations = 50
        rounds = 5
        warmup_rounds = 2
    elif scenario == "serialization":
        iterations = 50
        rounds = 5
        warmup_rounds = 2
    else:
        iterations = 20
        rounds = 3
        warmup_rounds = 1

    return {
        "iterations": iterations,
        "rounds": rounds,
        "warmup_rounds": warmup_rounds,
    }
