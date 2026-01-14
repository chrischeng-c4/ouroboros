"""Throughput benchmarks for API servers."""

from ouroboros.test import BenchmarkGroup, register_group
from . import benchmark_setup


# =====================
# Plaintext Response (Minimal Overhead)
# =====================

plaintext = BenchmarkGroup("Plaintext Response")


@plaintext.add("data-bridge")
async def db_plaintext():
    """GET /plaintext with data-bridge."""
    response = await benchmark_setup.make_request("data-bridge", "/plaintext")
    assert response.status_code == 200


@plaintext.add("FastAPI")
async def fastapi_plaintext():
    """GET /plaintext with FastAPI."""
    response = await benchmark_setup.make_request("fastapi", "/plaintext")
    assert response.status_code == 200


register_group(plaintext)


# =====================
# Path Parameter Extraction
# =====================

path_params = BenchmarkGroup("Path Parameters")


@path_params.add("data-bridge")
async def db_path_params():
    """GET /items/{id} with data-bridge."""
    response = await benchmark_setup.make_request("data-bridge", "/items/42")
    assert response.status_code == 200
    data = response.json()
    assert data["item_id"] == 42


@path_params.add("FastAPI")
async def fastapi_path_params():
    """GET /items/{id} with FastAPI."""
    response = await benchmark_setup.make_request("fastapi", "/items/42")
    assert response.status_code == 200
    data = response.json()
    assert data["item_id"] == 42


register_group(path_params)


# =====================
# JSON Response
# =====================

json_response = BenchmarkGroup("JSON Response")


@json_response.add("data-bridge")
async def db_json_response():
    """GET /items/{id} with JSON response (data-bridge)."""
    response = await benchmark_setup.make_request("data-bridge", "/items/1")
    assert response.status_code == 200
    data = response.json()
    assert "item_id" in data
    assert "name" in data


@json_response.add("FastAPI")
async def fastapi_json_response():
    """GET /items/{id} with JSON response (FastAPI)."""
    response = await benchmark_setup.make_request("fastapi", "/items/1")
    assert response.status_code == 200
    data = response.json()
    assert "item_id" in data
    assert "name" in data


register_group(json_response)
