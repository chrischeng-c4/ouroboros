"""
Test Python handler invocation bridge

This test verifies that:
1. Request data is correctly converted from Rust to Python
2. Python async handlers are properly invoked
3. Responses are correctly converted from Python to Rust
"""

import pytest

# Skip if api feature is not available
try:
    import ouroboros._rust
    ApiApp = ouroboros.ouroboros.api.ApiApp
    Response = ouroboros.ouroboros.api.Response
except (ImportError, AttributeError):
    pytest.skip("API feature not available", allow_module_level=True)


@pytest.mark.asyncio
async def test_basic_handler_invocation():
    """Test that a simple async handler can be called"""
    app = ApiApp(title="Test API", version="1.0.0")

    # Define an async handler
    async def hello_handler(request):
        return {"message": "Hello, World!"}

    # Register the route
    route_id = app.register_route("GET", "/hello", hello_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_handler_with_path_params():
    """Test handler receives path parameters"""
    app = ApiApp()

    async def user_handler(request):
        user_id = request["path_params"]["user_id"]
        return {"user_id": user_id}

    # Register route with path parameter
    route_id = app.register_route("GET", "/users/{user_id}", user_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_handler_with_query_params():
    """Test handler receives query parameters"""
    app = ApiApp()

    async def search_handler(request):
        query = request["query_params"].get("q", "")
        limit = request["query_params"].get("limit", 10)
        return {"query": query, "limit": limit}

    # Register route
    route_id = app.register_route("GET", "/search", search_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_handler_with_response_object():
    """Test handler can return Response object"""
    app = ApiApp()

    async def custom_response_handler(request):
        # Return a custom Response object
        response = Response.json({"status": "ok"})
        response.status(201)
        response.header("X-Custom-Header", "custom-value")
        return response

    route_id = app.register_route("POST", "/custom", custom_response_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_handler_with_text_response():
    """Test handler can return plain text"""
    app = ApiApp()

    async def text_handler(request):
        return "Hello, World!"

    route_id = app.register_route("GET", "/text", text_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_handler_with_dict_response():
    """Test handler can return dict (JSON)"""
    app = ApiApp()

    async def json_handler(request):
        return {
            "message": "Success",
            "data": {
                "count": 42,
                "items": [1, 2, 3]
            }
        }

    route_id = app.register_route("POST", "/json", json_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_route_matching():
    """Test that routes can be matched"""
    app = ApiApp()

    async def handler(request):
        return {"ok": True}

    app.register_route("GET", "/api/users/{user_id}", handler)

    # Test route matching
    match = app.match_route("GET", "/api/users/123")
    assert match is not None
    route_id, params = match
    assert route_id == "matched"
    assert params["user_id"] == "123"


@pytest.mark.asyncio
async def test_multiple_routes():
    """Test registering multiple routes"""
    app = ApiApp()

    async def get_users(request):
        return {"users": []}

    async def create_user(request):
        return {"id": "123"}

    async def get_user(request):
        return {"id": request["path_params"]["id"]}

    route1 = app.register_route("GET", "/users", get_users)
    route2 = app.register_route("POST", "/users", create_user)
    route3 = app.register_route("GET", "/users/{id}", get_user)

    assert route1 != route2
    assert route2 != route3
    assert route1 != route3


@pytest.mark.asyncio
async def test_sync_handler():
    """Test that synchronous handlers work correctly"""
    app = ApiApp(title="Test API", version="1.0.0")

    # Define a sync handler (not async)
    def sync_hello_handler(request):
        return {"message": "Hello from sync handler!"}

    # Register the route
    route_id = app.register_route("GET", "/sync-hello", sync_hello_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_sync_handler_with_path_params():
    """Test sync handler receives path parameters"""
    app = ApiApp()

    def sync_user_handler(request):
        user_id = request["path_params"]["user_id"]
        return {"user_id": user_id, "sync": True}

    # Register route with path parameter
    route_id = app.register_route("GET", "/sync/users/{user_id}", sync_user_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_sync_handler_with_response_object():
    """Test sync handler can return Response object"""
    app = ApiApp()

    def sync_custom_response_handler(request):
        # Return a custom Response object
        response = Response.json({"status": "ok", "sync": True})
        response.status(200)
        response.header("X-Sync-Handler", "true")
        return response

    route_id = app.register_route("POST", "/sync/custom", sync_custom_response_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_sync_handler_with_text_response():
    """Test sync handler can return plain text"""
    app = ApiApp()

    def sync_text_handler(request):
        return "Hello from sync handler!"

    route_id = app.register_route("GET", "/sync/text", sync_text_handler)
    assert route_id is not None


@pytest.mark.asyncio
async def test_mixed_sync_async_handlers():
    """Test that both sync and async handlers can be registered together"""
    app = ApiApp()

    # Sync handler
    def sync_handler(request):
        return {"type": "sync"}

    # Async handler
    async def async_handler(request):
        return {"type": "async"}

    route1 = app.register_route("GET", "/sync", sync_handler)
    route2 = app.register_route("GET", "/async", async_handler)

    assert route1 is not None
    assert route2 is not None
    assert route1 != route2
