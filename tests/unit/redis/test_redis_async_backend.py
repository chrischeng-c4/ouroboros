"""Tests for Redis asynchronous backend implementation (actual interface)."""

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
import redis.asyncio as aioredis
import redis.exceptions

from data_bridge.redis.async_.backend import RedisAsyncBackend


class MockAsyncHashModel:
    """Mock AsyncHashModel for testing."""
    _key_prefix = "hash:"
    
    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            setattr(self, key, value)
    
    def get_key(self):
        return f"hash:{getattr(self, 'id', 'test')}"
    
    def to_dict(self):
        return {key: value for key, value in self.__dict__.items() 
                if not key.startswith('_')}
    
    @classmethod
    def from_dict(cls, data):
        return cls(**data)


class MockAsyncJSONModel:
    """Mock AsyncJSONModel for testing."""
    _key_prefix = "json:"
    
    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            setattr(self, key, value)
    
    def get_key(self):
        return f"json:{getattr(self, 'id', 'test')}"
    
    def to_dict(self):
        return {key: value for key, value in self.__dict__.items()
                if not key.startswith('_')}
    
    @classmethod
    def from_dict(cls, data):
        return cls(**data)


class TestRedisAsyncBackendActual:
    """Test Redis asynchronous backend with actual interface."""
    
    def test_backend_initialization_default(self) -> None:
        """Test backend initialization with default parameters."""
        backend = RedisAsyncBackend()
        assert backend.connection_string == "redis://localhost:6379"
        assert backend.redis_kwargs == {}
        assert backend.client is None
    
    def test_backend_initialization_custom(self) -> None:
        """Test backend initialization with custom parameters."""
        backend = RedisAsyncBackend(
            connection_string="redis://custom:6380/2",
            socket_timeout=30,
            retry_on_timeout=True
        )
        assert backend.connection_string == "redis://custom:6380/2"
        assert backend.redis_kwargs["socket_timeout"] == 30
        assert backend.redis_kwargs["retry_on_timeout"] is True
        assert backend.client is None
    
    @patch('redis.asyncio.from_url')
    async def test_connect(self, mock_redis_from_url: MagicMock) -> None:
        """Test connection establishment."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        
        backend = RedisAsyncBackend()
        await backend.connect()
        
        mock_redis_from_url.assert_called_once_with("redis://localhost:6379")
        assert backend.client == mock_client
    
    async def test_connect_already_connected(self) -> None:
        """Test connect when already connected."""
        backend = RedisAsyncBackend()
        mock_client = AsyncMock()
        backend.client = mock_client
        
        await backend.connect()
        # Should not create new client
        assert backend.client == mock_client
    
    async def test_disconnect(self) -> None:
        """Test disconnection."""
        backend = RedisAsyncBackend()
        mock_client = AsyncMock()
        backend.client = mock_client
        
        await backend.disconnect()
        
        mock_client.aclose.assert_called_once()
        assert backend.client is None
    
    async def test_disconnect_not_connected(self) -> None:
        """Test disconnect when not connected."""
        backend = RedisAsyncBackend()
        await backend.disconnect()  # Should not raise error
        assert backend.client is None
    
    @patch('redis.asyncio.from_url')
    async def test_get_client(self, mock_redis_from_url: MagicMock) -> None:
        """Test _get_client method."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        
        backend = RedisAsyncBackend()
        client = await backend._get_client()
        
        mock_redis_from_url.assert_called_once()
        assert client == mock_client
        assert backend.client == mock_client
    
    async def test_get_client_when_already_connected(self) -> None:
        """Test _get_client method when client is already set."""
        backend = RedisAsyncBackend()
        mock_client = AsyncMock()
        
        # Pre-set the client
        backend.client = mock_client
        
        # Call _get_client - should return existing client without calling connect
        client = await backend._get_client()
        
        assert client == mock_client
        assert backend.client == mock_client
    
    @patch('redis.asyncio.from_url')
    async def test_save_without_ttl(self, mock_redis_from_url: MagicMock) -> None:
        """Test saving a model instance without TTL."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        
        instance = MockAsyncHashModel(name="John", age=30)
        backend = RedisAsyncBackend()
        await backend.save(instance)
        
        expected_data = json.dumps({"name": "John", "age": 30})
        mock_client.set.assert_called_once_with("hash:test", expected_data)
        mock_client.setex.assert_not_called()
    
    @patch('redis.asyncio.from_url')
    async def test_save_with_ttl(self, mock_redis_from_url: MagicMock) -> None:
        """Test saving a model instance with TTL."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        
        instance = MockAsyncHashModel(name="John", age=30)
        backend = RedisAsyncBackend()
        await backend.save(instance, ttl=3600)
        
        expected_data = json.dumps({"name": "John", "age": 30})
        mock_client.setex.assert_called_once_with("hash:test", 3600, expected_data)
        mock_client.set.assert_not_called()
    
    @patch('redis.asyncio.from_url')
    async def test_delete(self, mock_redis_from_url: MagicMock) -> None:
        """Test deleting a model instance."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.delete.return_value = 1
        
        instance = MockAsyncHashModel(id="123")
        backend = RedisAsyncBackend()
        await backend.delete(instance)
        
        mock_client.delete.assert_called_once_with("hash:123")
    
    @patch('redis.asyncio.from_url')
    @patch('data_bridge.redis.async_.backend.RedisKeyPattern')
    async def test_get_by_key_found(self, mock_key_pattern: MagicMock, 
                                   mock_redis_from_url: MagicMock) -> None:
        """Test getting a model instance by key when found."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "hash:123"
        
        # Mock Redis returning JSON data
        test_data = {"name": "John", "age": 30}
        mock_client.get.return_value = json.dumps(test_data)
        
        backend = RedisAsyncBackend()
        result = await backend.get_by_key(MockAsyncHashModel, "123")
        
        mock_key_pattern.build_key.assert_called_once_with("hash:", "123")
        mock_client.get.assert_called_once_with("hash:123")
        assert result is not None
        assert isinstance(result, MockAsyncHashModel)
        assert result.name == "John"
        assert result.age == 30
    
    @patch('redis.asyncio.from_url')
    @patch('data_bridge.redis.async_.backend.RedisKeyPattern')
    async def test_get_by_key_not_found(self, mock_key_pattern: MagicMock, 
                                       mock_redis_from_url: MagicMock) -> None:
        """Test getting a model instance by key when not found."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "hash:nonexistent"
        
        # Mock Redis returning None
        mock_client.get.return_value = None
        
        backend = RedisAsyncBackend()
        result = await backend.get_by_key(MockAsyncHashModel, "nonexistent")
        
        mock_key_pattern.build_key.assert_called_once_with("hash:", "nonexistent")
        mock_client.get.assert_called_once_with("hash:nonexistent")
        assert result is None
    
    @patch('redis.asyncio.from_url')
    @patch('data_bridge.redis.async_.backend.RedisKeyPattern')
    async def test_get_by_key_json_parsing(self, mock_key_pattern: MagicMock, 
                                          mock_redis_from_url: MagicMock) -> None:
        """Test getting a model with complex JSON data."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "json:123"
        
        # Mock Redis returning complex JSON data
        test_data = {
            "name": "Jane",
            "metadata": {"role": "admin", "permissions": ["read", "write"]},
            "tags": ["python", "redis"]
        }
        mock_client.get.return_value = json.dumps(test_data)
        
        backend = RedisAsyncBackend()
        result = await backend.get_by_key(MockAsyncJSONModel, "123")
        
        assert result is not None
        assert isinstance(result, MockAsyncJSONModel)
        assert result.name == "Jane"
        assert result.metadata == {"role": "admin", "permissions": ["read", "write"]}
        assert result.tags == ["python", "redis"]
    
    async def test_execute_query_simplified(self) -> None:
        """Test simplified execute_query implementation."""
        backend = RedisAsyncBackend()
        result = await backend.execute_query(None)
        assert result == []
    
    async def test_count_query_simplified(self) -> None:
        """Test simplified count_query implementation."""
        backend = RedisAsyncBackend()
        result = await backend.count_query(None)
        assert result == 0
    
    async def test_delete_query_simplified(self) -> None:
        """Test simplified delete_query implementation."""
        backend = RedisAsyncBackend()
        result = await backend.delete_query(None)
        assert result == 0
    
    async def test_update_query_simplified(self) -> None:
        """Test simplified update_query implementation."""
        backend = RedisAsyncBackend()
        result = await backend.update_query(None, {})
        assert result == 0
    
    @patch('redis.asyncio.from_url')
    async def test_redis_connection_error(self, mock_redis_from_url: MagicMock) -> None:
        """Test Redis connection error handling."""
        mock_redis_from_url.side_effect = redis.exceptions.ConnectionError("Connection failed")
        
        backend = RedisAsyncBackend()
        
        with pytest.raises(redis.exceptions.ConnectionError, match="Connection failed"):
            await backend.connect()
    
    @patch('redis.asyncio.from_url')
    async def test_save_with_client_error(self, mock_redis_from_url: MagicMock) -> None:
        """Test save operation with Redis client error."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.set.side_effect = redis.exceptions.ResponseError("ERR unknown command")
        
        instance = MockAsyncHashModel(name="John")
        backend = RedisAsyncBackend()
        
        with pytest.raises(redis.exceptions.ResponseError, match="ERR unknown command"):
            await backend.save(instance)
    
    @patch('redis.asyncio.from_url')
    async def test_delete_with_client_error(self, mock_redis_from_url: MagicMock) -> None:
        """Test delete operation with Redis client error."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.delete.side_effect = redis.exceptions.TimeoutError("Timeout")
        
        instance = MockAsyncHashModel(id="123")
        backend = RedisAsyncBackend()
        
        with pytest.raises(redis.exceptions.TimeoutError, match="Timeout"):
            await backend.delete(instance)
    
    @patch('redis.asyncio.from_url')
    @patch('data_bridge.redis.async_.backend.RedisKeyPattern')
    async def test_get_by_key_with_invalid_json(self, mock_key_pattern: MagicMock, 
                                               mock_redis_from_url: MagicMock) -> None:
        """Test getting a model instance with invalid JSON data."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "hash:123"
        
        # Mock Redis returning invalid JSON
        mock_client.get.return_value = "invalid json data"
        
        backend = RedisAsyncBackend()
        
        with pytest.raises(json.JSONDecodeError):
            await backend.get_by_key(MockAsyncHashModel, "123")
    
    @patch('redis.asyncio.from_url')
    async def test_save_complex_data_serialization(self, mock_redis_from_url: MagicMock) -> None:
        """Test saving instance with complex data types."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        
        # Create instance with complex nested data
        instance = MockAsyncJSONModel(
            name="Complex User",
            metadata={
                "settings": {"theme": "dark", "notifications": True},
                "history": [{"action": "login", "timestamp": 1234567890}]
            },
            tags=["admin", "power-user"]
        )
        
        backend = RedisAsyncBackend()
        await backend.save(instance)
        
        # Verify the JSON serialization
        call_args = mock_client.set.call_args
        key, json_data = call_args[0]
        
        assert key == "json:test"
        parsed_data = json.loads(json_data)
        assert parsed_data["name"] == "Complex User"
        assert parsed_data["metadata"]["settings"]["theme"] == "dark"
        assert parsed_data["tags"] == ["admin", "power-user"]
    
    @patch('redis.asyncio.from_url') 
    async def test_save_setex_error_handling(self, mock_redis_from_url: MagicMock) -> None:
        """Test save with TTL when setex fails."""
        mock_client = AsyncMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.setex.side_effect = redis.exceptions.ResponseError("ERR invalid expire time")
        
        instance = MockAsyncHashModel(name="John")
        backend = RedisAsyncBackend()
        
        with pytest.raises(redis.exceptions.ResponseError, match="ERR invalid expire time"):
            await backend.save(instance, ttl=3600)