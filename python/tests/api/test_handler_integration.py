"""
Integration tests for Python handler invocation through Rust HTTP server.

These tests verify end-to-end handler invocation including:
- JSON response handling
- Path parameter extraction
- Query parameter handling
- Both sync and async handlers
"""

import subprocess
import sys
import tempfile
import time
from pathlib import Path

import httpx
import pytest

# Test port - different from benchmark ports
TEST_PORT = 18765


def wait_for_server(url: str, timeout: float = 10.0) -> bool:
    """Wait for server to become ready."""
    start = time.time()
    while time.time() - start < timeout:
        try:
            response = httpx.get(f"{url}/health", timeout=1.0)
            if response.status_code == 200:
                return True
        except httpx.RequestError:
            pass
        time.sleep(0.1)
    return False


class TestHandlerIntegration:
    """Integration tests for handler invocation."""

    @pytest.fixture(scope="class")
    def server(self):
        """Start a test server for the integration tests."""
        server_script = '''
import asyncio
from ouroboros.api import App

app = App(title="Test API", version="1.0.0")

@app.get("/health")
async def health(request):
    return {"status": "ok"}

@app.get("/json")
async def json_handler(request):
    """Return a JSON response."""
    return {"message": "Hello, World!", "count": 42}

@app.get("/users/{user_id}")
async def get_user(request):
    """Handler with path parameter."""
    user_id = request["path_params"]["user_id"]
    return {"user_id": user_id, "type": "path_param"}

@app.get("/search")
async def search(request):
    """Handler with query parameters."""
    q = request["query_params"].get("q", "")
    limit = int(request["query_params"].get("limit", "10"))
    return {"query": q, "limit": limit}

@app.post("/echo")
async def echo(request):
    """Echo back the request body."""
    body = request.get("body", {})
    return {"received": body}

@app.get("/sync")
def sync_handler(request):
    """Synchronous handler - should also work."""
    return {"sync": True, "value": "from sync handler"}

if __name__ == "__main__":
    app.run(host="127.0.0.1", port=TEST_PORT)
'''
        # Replace TEST_PORT in script
        server_script = server_script.replace("TEST_PORT", str(TEST_PORT))

        # Write script to temp file
        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            f.write(server_script)
            script_path = f.name

        # Start server process
        process = subprocess.Popen(
            [sys.executable, script_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        base_url = f"http://127.0.0.1:{TEST_PORT}"

        # Wait for server to be ready
        if not wait_for_server(base_url):
            process.terminate()
            process.wait()
            stdout, stderr = process.communicate(timeout=5)
            pytest.fail(
                f"Server failed to start.\n"
                f"stdout: {stdout.decode()}\n"
                f"stderr: {stderr.decode()}"
            )

        yield base_url

        # Cleanup
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

        # Remove temp file
        Path(script_path).unlink(missing_ok=True)

    def test_json_response(self, server):
        """Test handler returning JSON response."""
        response = httpx.get(f"{server}/json")

        assert response.status_code == 200
        assert response.headers.get("content-type", "").startswith("application/json")

        data = response.json()
        assert data["message"] == "Hello, World!"
        assert data["count"] == 42

    def test_path_parameters(self, server):
        """Test handler with path parameters."""
        user_id = "user-123"
        response = httpx.get(f"{server}/users/{user_id}")

        assert response.status_code == 200
        data = response.json()
        assert data["user_id"] == user_id
        assert data["type"] == "path_param"

    def test_path_parameters_special_chars(self, server):
        """Test path parameters with special characters."""
        # URL-encoded special characters
        user_id = "user%40example.com"  # user@example.com encoded
        response = httpx.get(f"{server}/users/{user_id}")

        assert response.status_code == 200
        data = response.json()
        # The server should decode the URL-encoded value
        assert "user" in data["user_id"]

    def test_query_parameters(self, server):
        """Test handler with query parameters."""
        response = httpx.get(f"{server}/search", params={"q": "test query", "limit": "25"})

        assert response.status_code == 200
        data = response.json()
        assert data["query"] == "test query"
        assert data["limit"] == 25 or data["limit"] == "25"  # May be string

    def test_query_parameters_defaults(self, server):
        """Test query parameters with default values."""
        response = httpx.get(f"{server}/search")

        assert response.status_code == 200
        data = response.json()
        assert data["query"] == ""
        assert data["limit"] == 10

    def test_post_json_body(self, server):
        """Test POST handler with JSON body."""
        payload = {"name": "Test User", "email": "test@example.com"}
        response = httpx.post(f"{server}/echo", json=payload)

        assert response.status_code == 200
        data = response.json()
        assert data["received"] == payload

    def test_sync_handler(self, server):
        """Test synchronous handler works correctly."""
        response = httpx.get(f"{server}/sync")

        assert response.status_code == 200
        data = response.json()
        assert data["sync"] is True
        assert data["value"] == "from sync handler"

    def test_health_endpoint(self, server):
        """Test health check endpoint."""
        response = httpx.get(f"{server}/health")

        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"


class TestValidation:
    """Tests for request validation."""

    @pytest.fixture(scope="class")
    def validation_server(self):
        """Start a validation test server."""
        server_script = '''
from ouroboros.api import App

app = App(title="Validation Test API", version="1.0.0")

@app.get("/health")
async def health(request):
    return {"status": "ok"}

@app.get("/items/{item_id}")
async def get_item(request):
    """Handler with typed path parameter."""
    item_id = int(request["path_params"]["item_id"])
    return {"item_id": item_id, "doubled": item_id * 2}

@app.get("/validate")
async def validate(request):
    """Handler with required query parameters."""
    count = int(request["query_params"]["count"])
    name = request["query_params"]["name"]
    return {"count": count, "name": name}

if __name__ == "__main__":
    app.run(host="127.0.0.1", port=TEST_PORT)
'''
        server_script = server_script.replace("TEST_PORT", str(TEST_PORT + 1))

        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            f.write(server_script)
            script_path = f.name

        process = subprocess.Popen(
            [sys.executable, script_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        base_url = f"http://127.0.0.1:{TEST_PORT + 1}"

        if not wait_for_server(base_url):
            process.terminate()
            process.wait()
            stdout, stderr = process.communicate(timeout=5)
            pytest.fail(
                f"Validation server failed to start.\n"
                f"stdout: {stdout.decode()}\n"
                f"stderr: {stderr.decode()}"
            )

        yield base_url

        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

        Path(script_path).unlink(missing_ok=True)

    def test_typed_path_parameter(self, validation_server):
        """Test path parameter type conversion."""
        response = httpx.get(f"{validation_server}/items/42")

        assert response.status_code == 200
        data = response.json()
        assert data["item_id"] == 42
        assert data["doubled"] == 84

    def test_required_query_parameters(self, validation_server):
        """Test required query parameters."""
        response = httpx.get(
            f"{validation_server}/validate",
            params={"count": "5", "name": "test"}
        )

        assert response.status_code == 200
        data = response.json()
        assert data["count"] == 5 or data["count"] == "5"
        assert data["name"] == "test"


class TestQueryParameterPassthrough:
    """Tests for query parameter passthrough behavior."""

    @pytest.fixture(scope="class")
    def passthrough_server(self):
        """Start a server to test query parameter passthrough."""
        server_script = '''
from ouroboros.api import App

app = App(title="Passthrough Test API", version="1.0.0")

@app.get("/health")
async def health(request):
    return {"status": "ok"}

@app.get("/echo-query")
async def echo_query(request):
    """Echo back all query parameters."""
    return {"query_params": request["query_params"]}

@app.get("/mixed-query")
async def mixed_query(request):
    """Handler that expects some specific params but should receive all."""
    # Only reads specific params, but all should be available
    page = request["query_params"].get("page", "1")
    # Extra params like "filter" or "search" should also be present
    return {
        "page": page,
        "all_params": request["query_params"]
    }

if __name__ == "__main__":
    app.run(host="127.0.0.1", port=TEST_PORT)
'''
        server_script = server_script.replace("TEST_PORT", str(TEST_PORT + 2))

        with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False) as f:
            f.write(server_script)
            script_path = f.name

        process = subprocess.Popen(
            [sys.executable, script_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        base_url = f"http://127.0.0.1:{TEST_PORT + 2}"

        if not wait_for_server(base_url):
            process.terminate()
            process.wait()
            stdout, stderr = process.communicate(timeout=5)
            pytest.fail(
                f"Passthrough server failed to start.\n"
                f"stdout: {stdout.decode()}\n"
                f"stderr: {stderr.decode()}"
            )

        yield base_url

        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

        Path(script_path).unlink(missing_ok=True)

    def test_all_query_params_passed_through(self, passthrough_server):
        """Test that all query parameters are passed to handler."""
        response = httpx.get(
            f"{passthrough_server}/echo-query",
            params={
                "search": "test",
                "filter": "active",
                "page": "1",
                "limit": "10",
                "sort": "name"
            }
        )

        assert response.status_code == 200
        data = response.json()
        query_params = data["query_params"]

        # All parameters should be present
        assert "search" in query_params
        assert "filter" in query_params
        assert "page" in query_params
        assert "limit" in query_params
        assert "sort" in query_params

        # Verify values
        assert query_params["search"] == "test"
        assert query_params["filter"] == "active"

    def test_extra_query_params_with_known_params(self, passthrough_server):
        """Test that extra query params are passed even when handler only uses some."""
        response = httpx.get(
            f"{passthrough_server}/mixed-query",
            params={
                "page": "2",
                "search": "test query",
                "filter": "active",
                "extra_param": "should_be_present"
            }
        )

        assert response.status_code == 200
        data = response.json()

        # The handler reads 'page'
        assert data["page"] == "2"

        # But all params should be in the full dict
        all_params = data["all_params"]
        assert "page" in all_params
        assert "search" in all_params
        assert "filter" in all_params
        assert "extra_param" in all_params

        # Verify extra params have correct values
        assert all_params["search"] == "test query"
        assert all_params["filter"] == "active"
        assert all_params["extra_param"] == "should_be_present"
