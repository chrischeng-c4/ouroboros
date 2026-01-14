"""
Shared setup and utilities for API benchmarks.

This module provides shared state and helper functions for API benchmarks,
using ouroboros.http.HttpClient.
"""

import asyncio
from typing import Optional
from ouroboros.http import HttpClient

# Global URL storage
_data_bridge_url: Optional[str] = None
_fastapi_url: Optional[str] = None


def init_session(data_bridge_url: str, fastapi_url: str):
    """Initialize global URLs."""
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
