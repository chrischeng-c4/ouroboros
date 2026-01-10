"""Serialization benchmarks for API servers."""

from data_bridge.test import BenchmarkGroup, register_group
from . import benchmark_setup
from .conftest import PAYLOAD_SIZES


# =====================
# Small Payload (1KB)
# =====================

serialize_small = BenchmarkGroup("Serialize Small (1KB)")


@serialize_small.add("data-bridge")
async def db_serialize_small():
    """Serialize 1KB payload with data-bridge."""
    size = PAYLOAD_SIZES["small"]
    response = await benchmark_setup.make_request("data-bridge", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


@serialize_small.add("FastAPI")
async def fastapi_serialize_small():
    """Serialize 1KB payload with FastAPI."""
    size = PAYLOAD_SIZES["small"]
    response = await benchmark_setup.make_request("fastapi", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


register_group(serialize_small)


# =====================
# Medium Payload (10KB)
# =====================

serialize_medium = BenchmarkGroup("Serialize Medium (10KB)")


@serialize_medium.add("data-bridge")
async def db_serialize_medium():
    """Serialize 10KB payload with data-bridge."""
    size = PAYLOAD_SIZES["medium"]
    response = await benchmark_setup.make_request("data-bridge", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


@serialize_medium.add("FastAPI")
async def fastapi_serialize_medium():
    """Serialize 10KB payload with FastAPI."""
    size = PAYLOAD_SIZES["medium"]
    response = await benchmark_setup.make_request("fastapi", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


register_group(serialize_medium)


# =====================
# Large Payload (100KB)
# =====================

serialize_large = BenchmarkGroup("Serialize Large (100KB)")


@serialize_large.add("data-bridge")
async def db_serialize_large():
    """Serialize 100KB payload with data-bridge."""
    size = PAYLOAD_SIZES["large"]
    response = await benchmark_setup.make_request("data-bridge", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


@serialize_large.add("FastAPI")
async def fastapi_serialize_large():
    """Serialize 100KB payload with FastAPI."""
    size = PAYLOAD_SIZES["large"]
    response = await benchmark_setup.make_request("fastapi", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


register_group(serialize_large)


# =====================
# Extra Large Payload (1MB)
# =====================

serialize_xlarge = BenchmarkGroup("Serialize XLarge (1MB)")


@serialize_xlarge.add("data-bridge")
async def db_serialize_xlarge():
    """Serialize 1MB payload with data-bridge."""
    size = PAYLOAD_SIZES["xlarge"]
    response = await benchmark_setup.make_request("data-bridge", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


@serialize_xlarge.add("FastAPI")
async def fastapi_serialize_xlarge():
    """Serialize 1MB payload with FastAPI."""
    size = PAYLOAD_SIZES["xlarge"]
    response = await benchmark_setup.make_request("fastapi", f"/json/{size}")
    assert response.status_code == 200
    data = response.json()
    assert "data" in data
    assert len(data["data"]) == size


register_group(serialize_xlarge)
