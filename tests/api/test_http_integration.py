"""Tests for HTTP client integration in data-bridge-api."""
import pytest
from data_bridge.test import expect
from typing import Annotated
from data_bridge.api import App, Depends, RequestContext
from data_bridge.http import HttpClient
from data_bridge.api.http_integration import HttpClientProvider, create_http_client


def test_create_http_client():
    """Test create_http_client factory function."""
    client = create_http_client(
        base_url="https://api.example.com",
        timeout=30.0,
        connect_timeout=10.0
    )
    assert isinstance(client, HttpClient)


def test_http_client_provider():
    """Test HttpClientProvider singleton pattern."""
    provider = HttpClientProvider()

    # Configure
    provider.configure(
        base_url="https://api.example.com",
        timeout=30.0
    )

    # Get client
    client1 = provider.get_client()
    client2 = provider.get_client()

    # Should be same instance (singleton)
    assert client1 is client2

    # Reconfigure should reset client
    provider.configure(base_url="https://api2.example.com")
    client3 = provider.get_client()

    # Should be different instance after reconfigure
    assert client3 is not client1


def test_http_client_provider_callable():
    """Test HttpClientProvider is callable."""
    provider = HttpClientProvider()
    provider.configure(base_url="https://api.example.com")

    client = provider()
    assert isinstance(client, HttpClient)


def test_app_configure_http_client():
    """Test App.configure_http_client() method."""
    app = App()

    # Configure
    app.configure_http_client(
        base_url="https://api.example.com",
        timeout=30.0
    )

    # Should be able to access client
    client = app.http_client
    assert isinstance(client, HttpClient)


def test_app_http_client_property():
    """Test App.http_client property."""
    app = App()
    app.configure_http_client(base_url="https://api.example.com")

    # Should return HttpClient
    client = app.http_client
    assert isinstance(client, HttpClient)

    # Multiple accesses should return same instance
    assert app.http_client is client


@pytest.mark.asyncio
async def test_http_client_as_dependency_bare_type():
    """Test HttpClient can be used as dependency with bare type hint."""
    app = App()
    app.configure_http_client(base_url="https://jsonplaceholder.typicode.com")

    @app.get("/test")
    async def handler(http: HttpClient) -> dict:
        response = await http.get("/posts/1")
        return response.json()

    # Resolve dependencies
    from data_bridge.api.dependencies import RequestContext as DepRequestContext
    context = DepRequestContext()
    deps = await app.resolve_dependencies(handler, context)

    assert 'http' in deps
    assert isinstance(deps['http'], HttpClient)

    # Execute handler
    result = await handler(http=deps['http'])
    assert result['id'] == 1
    assert 'title' in result


@pytest.mark.asyncio
async def test_http_client_multiple_handlers():
    """Test HttpClient dependency is shared across handlers."""
    app = App()
    app.configure_http_client(base_url="https://api.example.com")

    @app.get("/handler1")
    async def handler1(http: HttpClient):
        return http

    @app.get("/handler2")
    async def handler2(http: HttpClient):
        return http

    # Resolve for both handlers
    from data_bridge.api.dependencies import RequestContext as DepRequestContext
    context = DepRequestContext()

    deps1 = await app.resolve_dependencies(handler1, context)
    deps2 = await app.resolve_dependencies(handler2, context)

    # Should be same instance (singleton)
    assert deps1['http'] is deps2['http']


def test_request_context_http_property():
    """Test RequestContext.http property."""
    from data_bridge.http import HttpClient

    client = HttpClient()
    ctx = RequestContext(_http_client=client)

    assert ctx.http is client


def test_request_context_http_property_not_configured():
    """Test RequestContext.http raises error if not configured."""
    ctx = RequestContext()

    expect(lambda: ctx.http).to_raise(RuntimeError)


def test_request_context_get_header():
    """Test RequestContext.get_header() case-insensitive lookup."""
    ctx = RequestContext(
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer token123"
        }
    )

    # Exact case
    assert ctx.get_header("Content-Type") == "application/json"

    # Different case
    assert ctx.get_header("content-type") == "application/json"
    assert ctx.get_header("AUTHORIZATION") == "Bearer token123"

    # Not found
    assert ctx.get_header("X-Custom") is None
    assert ctx.get_header("X-Custom", "default") == "default"


def test_request_context_metadata():
    """Test RequestContext metadata fields."""
    ctx = RequestContext(
        client_ip="192.168.1.1",
        method="GET",
        path="/users/123",
        headers={"User-Agent": "test"},
        path_params={"user_id": "123"},
        query_params={"limit": "10"}
    )

    assert ctx.client_ip == "192.168.1.1"
    assert ctx.method == "GET"
    assert ctx.path == "/users/123"
    assert ctx.headers["User-Agent"] == "test"
    assert ctx.path_params["user_id"] == "123"
    assert ctx.query_params["limit"] == "10"
    assert ctx.request_id  # Should have auto-generated UUID


@pytest.mark.asyncio
async def test_http_client_with_custom_config():
    """Test HTTP client with custom configuration."""
    app = App()
    app.configure_http_client(
        base_url="https://api.example.com",
        timeout=60.0,
        connect_timeout=5.0,
        pool_max_idle_per_host=20
    )

    client = app.http_client
    assert isinstance(client, HttpClient)


@pytest.mark.asyncio
async def test_dependency_injection_order():
    """Test that HttpClient is registered before routes are compiled."""
    app = App()

    # Register route first
    @app.get("/test")
    async def handler(http: HttpClient):
        return "ok"

    # Then configure HTTP client
    app.configure_http_client(base_url="https://api.example.com")

    # Should still resolve correctly
    from data_bridge.api.dependencies import RequestContext as DepRequestContext
    context = DepRequestContext()
    deps = await app.resolve_dependencies(handler, context)

    assert 'http' in deps
    assert isinstance(deps['http'], HttpClient)
