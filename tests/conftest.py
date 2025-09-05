"""Test configuration and fixtures."""

from __future__ import annotations

from collections.abc import Generator
from typing import Any
from unittest.mock import AsyncMock, MagicMock

import pytest


# Mock MongoDB Fixtures
@pytest.fixture
def mock_mongo_client() -> MagicMock:
    """Mock MongoDB client for unit tests."""
    client = MagicMock()
    db = MagicMock()
    collection = MagicMock()
    
    client.__getitem__.return_value = db
    db.__getitem__.return_value = collection
    
    # Mock common collection methods
    collection.find_one.return_value = {"_id": "test_id", "name": "test"}
    collection.find.return_value = [{"_id": "test_id", "name": "test"}]
    collection.insert_one.return_value = MagicMock(inserted_id="test_id")
    collection.update_one.return_value = MagicMock(modified_count=1)
    collection.delete_one.return_value = MagicMock(deleted_count=1)
    collection.count_documents.return_value = 1
    
    return client


@pytest.fixture
def mock_motor_client() -> AsyncMock:
    """Mock Motor (async MongoDB) client for unit tests."""
    client = AsyncMock()
    db = AsyncMock()
    collection = AsyncMock()
    
    client.__getitem__.return_value = db
    db.__getitem__.return_value = collection
    
    # Mock common async collection methods
    collection.find_one.return_value = {"_id": "test_id", "name": "test"}
    collection.find.return_value.to_list.return_value = [{"_id": "test_id", "name": "test"}]
    collection.insert_one.return_value.inserted_id = "test_id"
    collection.update_one.return_value.modified_count = 1
    collection.delete_one.return_value.deleted_count = 1
    collection.count_documents.return_value = 1
    
    return client


# Mock Redis Fixtures
@pytest.fixture
def mock_redis_client() -> MagicMock:
    """Mock Redis client for unit tests."""
    client = MagicMock()
    
    # Mock common Redis operations
    client.hset.return_value = 1
    client.hget.return_value = b"test_value"
    client.hgetall.return_value = {b"field1": b"value1", b"field2": b"value2"}
    client.hdel.return_value = 1
    client.delete.return_value = 1
    client.exists.return_value = 1
    client.scan_iter.return_value = [b"test:key1", b"test:key2"]
    client.expire.return_value = True
    client.ttl.return_value = 3600
    
    # Mock JSON operations (Redis JSON)
    client.json.return_value.set.return_value = True
    client.json.return_value.get.return_value = {"field1": "value1", "field2": "value2"}
    client.json.return_value.delete.return_value = 1
    
    return client


@pytest.fixture
def mock_redis_async_client() -> AsyncMock:
    """Mock Redis async client for unit tests."""
    client = AsyncMock()
    
    # Mock common async Redis operations
    client.hset.return_value = 1
    client.hget.return_value = b"test_value"
    client.hgetall.return_value = {b"field1": b"value1", b"field2": b"value2"}
    client.hdel.return_value = 1
    client.delete.return_value = 1
    client.exists.return_value = 1
    client.scan_iter.return_value = [b"test:key1", b"test:key2"]
    client.expire.return_value = True
    client.ttl.return_value = 3600
    
    # Mock JSON operations (Redis JSON)
    client.json.return_value.set.return_value = True
    client.json.return_value.get.return_value = {"field1": "value1", "field2": "value2"}
    client.json.return_value.delete.return_value = 1
    
    return client


# Test Model Fixtures
@pytest.fixture
def sample_user_data() -> dict[str, Any]:
    """Sample user data for testing."""
    return {
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30,
        "active": True,
        "tags": ["developer", "python"],
    }


@pytest.fixture
def sample_post_data() -> dict[str, Any]:
    """Sample post data for testing."""
    return {
        "title": "Test Post",
        "content": "This is a test post content",
        "author_id": "test_user_id",
        "published": True,
        "views": 0,
    }


# Environment Fixtures
@pytest.fixture(autouse=True)
def mock_environment() -> Generator[None, None, None]:
    """Automatically mock environment variables for all tests."""
    import os
    
    original_env = os.environ.copy()
    
    # Set test environment variables
    os.environ["MONGODB_URL"] = "mongodb://localhost:27017"
    os.environ["REDIS_URL"] = "redis://localhost:6379"
    os.environ["TEST_DATABASE"] = "test_db"
    
    yield
    
    # Restore original environment
    os.environ.clear()
    os.environ.update(original_env)


# Pytest Markers
def pytest_configure(config: pytest.Config) -> None:
    """Configure pytest markers."""
    config.addinivalue_line(
        "markers", "unit: mark test as unit test (no external dependencies)"
    )
    config.addinivalue_line(
        "markers", "integration: mark test as integration test (requires external services)"
    )
    config.addinivalue_line(
        "markers", "mongo: mark test as MongoDB-specific"
    )
    config.addinivalue_line(
        "markers", "redis: mark test as Redis-specific"
    )
    config.addinivalue_line(
        "markers", "async_test: mark test as async test"
    )


# Test Collection Rules
def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    """Modify test collection to add markers based on file paths."""
    for item in items:
        # Add unit marker for unit tests
        if "unit" in str(item.fspath):
            item.add_marker(pytest.mark.unit)
        
        # Add integration marker for integration tests
        if "integration" in str(item.fspath):
            item.add_marker(pytest.mark.integration)
        
        # Add database-specific markers
        if "mongo" in str(item.fspath):
            item.add_marker(pytest.mark.mongo)
        
        if "redis" in str(item.fspath):
            item.add_marker(pytest.mark.redis)
        
        # Add async marker for async tests
        if "async" in str(item.fspath) or item.name.startswith("test_async"):
            item.add_marker(pytest.mark.async_test)