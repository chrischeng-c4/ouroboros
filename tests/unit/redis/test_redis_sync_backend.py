"""Tests for Redis synchronous backend implementation (actual interface)."""

import json
from unittest.mock import MagicMock, patch

import pytest
import redis

from data_bridge.redis.sync.backend import RedisSyncBackend


class MockHashModel:
    """Mock HashModel for testing."""
    _key_prefix = "hash:"
    _fields = {
        "name": MagicMock(type_=str),
        "age": MagicMock(type_=int),
        "active": MagicMock(type_=bool),
        "tags": MagicMock(type_=list)
    }
    
    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            setattr(self, key, value)
    
    def get_key(self):
        return f"hash:{getattr(self, 'id', 'test')}"
    
    def to_dict(self):
        return {key: getattr(self, key, None) for key in self._fields}
    
    @classmethod
    def from_dict(cls, data):
        return cls(**data)


class MockJSONModel:
    """Mock JSONModel for testing."""
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


class TestRedisSyncBackendActual:
    """Test Redis synchronous backend with actual interface."""
    
    def test_backend_initialization_default(self) -> None:
        """Test backend initialization with default parameters."""
        backend = RedisSyncBackend()
        assert backend.connection_string == "redis://localhost:6379"
        assert backend.redis_kwargs == {}
        assert backend.client is None
    
    def test_backend_initialization_custom(self) -> None:
        """Test backend initialization with custom parameters."""
        backend = RedisSyncBackend(
            connection_string="redis://custom:6380/2",
            socket_timeout=30,
            retry_on_timeout=True
        )
        assert backend.connection_string == "redis://custom:6380/2"
        assert backend.redis_kwargs["socket_timeout"] == 30
        assert backend.redis_kwargs["retry_on_timeout"] is True
        assert backend.client is None
    
    @patch('redis.from_url')
    def test_connect(self, mock_redis_from_url: MagicMock) -> None:
        """Test connection establishment."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        
        backend = RedisSyncBackend()
        backend.connect()
        
        mock_redis_from_url.assert_called_once_with("redis://localhost:6379")
        assert backend.client == mock_client
    
    def test_connect_already_connected(self) -> None:
        """Test connect when already connected."""
        backend = RedisSyncBackend()
        mock_client = MagicMock(spec=redis.Redis)
        backend.client = mock_client
        
        backend.connect()
        # Should not create new client
        assert backend.client == mock_client
    
    def test_disconnect(self) -> None:
        """Test disconnection."""
        backend = RedisSyncBackend()
        mock_client = MagicMock(spec=redis.Redis)
        backend.client = mock_client
        
        backend.disconnect()
        
        mock_client.close.assert_called_once()
        assert backend.client is None
    
    def test_disconnect_not_connected(self) -> None:
        """Test disconnect when not connected."""
        backend = RedisSyncBackend()
        backend.disconnect()  # Should not raise error
        assert backend.client is None
    
    @patch('redis.from_url')
    def test_get_client(self, mock_redis_from_url: MagicMock) -> None:
        """Test _get_client method."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        
        backend = RedisSyncBackend()
        client = backend._get_client()
        
        mock_redis_from_url.assert_called_once()
        assert client == mock_client
        assert backend.client == mock_client
    
    @patch('redis.from_url')
    def test_get_client_when_none(self, mock_redis_from_url: MagicMock) -> None:
        """Test _get_client method when client is None."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        
        backend = RedisSyncBackend()
        # Ensure client starts as None
        assert backend.client is None
        
        # Call _get_client directly
        client = backend._get_client()
        
        mock_redis_from_url.assert_called_once_with("redis://localhost:6379")
        assert client == mock_client
        assert backend.client == mock_client
    
    def test_get_client_when_already_connected(self) -> None:
        """Test _get_client method when client is already set."""
        backend = RedisSyncBackend()
        mock_client = MagicMock(spec=redis.Redis)
        
        # Pre-set the client
        backend.client = mock_client
        
        # Call _get_client - should return existing client without calling connect
        client = backend._get_client()
        
        assert client == mock_client
        assert backend.client == mock_client
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.isinstance')
    def test_save_hash_model(self, mock_isinstance: MagicMock, mock_redis_from_url: MagicMock) -> None:
        """Test saving a HashModel instance."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_pipe = MagicMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.pipeline.return_value.__enter__.return_value = mock_pipe
        mock_pipe.execute.return_value = [3, True]
        
        # Configure isinstance mock for HashModel
        def isinstance_side_effect(obj, cls):
            return hasattr(cls, '__name__') and cls.__name__ == 'HashModel'
        
        mock_isinstance.side_effect = isinstance_side_effect
        
        # Create mock instance
        instance = MockHashModel(name="John", age=30, active=True)
        
        backend = RedisSyncBackend()
        backend.save(instance)
        
        # Verify pipeline operations - only non-None values are included
        mock_pipe.hset.assert_called_once()
        call_args = mock_pipe.hset.call_args
        assert call_args[0][0] == "hash:test"
        
        # Check that mapping contains expected fields (tags is None so not included)
        mapping = call_args[1]["mapping"]
        assert mapping["name"] == "John"
        assert mapping["age"] == "30"
        assert mapping["active"] == "True"
        # tags field should not be included since it's None
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.isinstance')
    def test_save_json_model_with_redisjson(self, mock_isinstance: MagicMock, mock_redis_from_url: MagicMock) -> None:
        """Test saving a JSONModel instance with RedisJSON."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_json = MagicMock()
        mock_client.json.return_value = mock_json
        mock_redis_from_url.return_value = mock_client
        
        # Configure isinstance mock for JSONModel
        def isinstance_side_effect(obj, cls):
            if hasattr(cls, '__name__'):
                return cls.__name__ == 'JSONModel'
            return False
        
        mock_isinstance.side_effect = isinstance_side_effect
        
        # Create mock instance
        instance = MockJSONModel(name="Jane", age=25, tags=["python", "redis"])
        
        backend = RedisSyncBackend()
        backend.save(instance)
        
        # Verify RedisJSON set
        expected_data = {"name": "Jane", "age": 25, "tags": ["python", "redis"]}
        mock_json.set.assert_called_once_with("json:test", "$", expected_data)
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.isinstance')
    def test_save_json_model_fallback(self, mock_isinstance: MagicMock, mock_redis_from_url: MagicMock) -> None:
        """Test saving a JSONModel instance with fallback to string storage."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        
        # Simulate RedisJSON not available
        mock_client.json.side_effect = AttributeError("No JSON module")
        
        # Configure isinstance mock
        def isinstance_side_effect(obj, cls):
            if hasattr(cls, '__name__'):
                return cls.__name__ == 'JSONModel'
            return False
        
        mock_isinstance.side_effect = isinstance_side_effect
        
        # Create mock instance
        instance = MockJSONModel(name="Jane", age=25)
        
        backend = RedisSyncBackend()
        backend.save(instance)
        
        # Verify fallback to string storage
        expected_json = json.dumps({"name": "Jane", "age": 25})
        mock_client.set.assert_called_once_with("json:test", expected_json)
    
    @patch('redis.from_url')
    def test_delete(self, mock_redis_from_url: MagicMock) -> None:
        """Test deleting a model instance."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.delete.return_value = 1
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        backend.delete(instance)
        
        mock_client.delete.assert_called_once_with("hash:123")
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.issubclass')
    @patch('data_bridge.redis.sync.backend.RedisKeyPattern')
    def test_get_by_key_hash_model(self, mock_key_pattern: MagicMock, mock_issubclass: MagicMock, 
                                   mock_redis_from_url: MagicMock) -> None:
        """Test getting a HashModel by key."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "hash:123"
        
        # Mock Redis hash data
        mock_client.hgetall.return_value = {
            b"name": b"John",
            b"age": b"30",
            b"active": b"true"
        }
        
        # Configure issubclass mock
        mock_issubclass.side_effect = lambda cls, parent: parent.__name__ == 'HashModel'
        
        backend = RedisSyncBackend()
        result = backend.get_by_key(MockHashModel, "123")
        
        mock_client.hgetall.assert_called_once_with("hash:123")
        assert result is not None
        assert isinstance(result, MockHashModel)
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.issubclass')
    @patch('data_bridge.redis.sync.backend.RedisKeyPattern')
    def test_get_by_key_json_model_with_redisjson(self, mock_key_pattern: MagicMock, 
                                                  mock_issubclass: MagicMock, 
                                                  mock_redis_from_url: MagicMock) -> None:
        """Test getting a JSONModel by key with RedisJSON."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_json = MagicMock()
        mock_client.json.return_value = mock_json
        mock_json.get.return_value = {"name": "Jane", "age": 25}
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "json:123"
        
        # Configure issubclass mock
        mock_issubclass.side_effect = lambda cls, parent: parent.__name__ == 'JSONModel'
        
        backend = RedisSyncBackend()
        result = backend.get_by_key(MockJSONModel, "123")
        
        mock_json.get.assert_called_once_with("json:123")
        assert result is not None
        assert isinstance(result, MockJSONModel)
        assert result.name == "Jane"
        assert result.age == 25
    
    @patch('redis.from_url')
    def test_exists(self, mock_redis_from_url: MagicMock) -> None:
        """Test checking if instance exists."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.exists.return_value = 1
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.exists(instance)
        
        mock_client.exists.assert_called_once_with("hash:123")
        assert result is True
    
    @patch('redis.from_url')
    def test_exists_false(self, mock_redis_from_url: MagicMock) -> None:
        """Test exists returning False."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.exists.return_value = 0
        
        instance = MockHashModel(id="nonexistent")
        backend = RedisSyncBackend()
        result = backend.exists(instance)
        
        mock_client.exists.assert_called_once_with("hash:nonexistent")
        assert result is False
    
    @patch('redis.from_url')
    def test_get_field(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a single field from HashModel."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b"John"
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "name")
        
        mock_client.hget.assert_called_once_with("hash:123", "name")
        assert result == "John"
    
    @patch('redis.from_url')
    def test_get_field_integer(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting an integer field from HashModel."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b"30"
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "age")
        
        mock_client.hget.assert_called_once_with("hash:123", "age")
        assert result == 30
        assert isinstance(result, int)
    
    @patch('redis.from_url')
    def test_set_field(self, mock_redis_from_url: MagicMock) -> None:
        """Test setting a single field in HashModel."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_pipe = MagicMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.pipeline.return_value.__enter__.return_value = mock_pipe
        mock_pipe.execute.return_value = [1, True]
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        backend.set_field(instance, "name", "Jane")
        
        mock_pipe.hset.assert_called_once_with("hash:123", "name", "Jane")
    
    @patch('redis.from_url')
    def test_set_field_with_ttl(self, mock_redis_from_url: MagicMock) -> None:
        """Test setting a field with TTL."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_pipe = MagicMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.pipeline.return_value.__enter__.return_value = mock_pipe
        mock_pipe.execute.return_value = [1, True]
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        backend.set_field(instance, "name", "Jane", ttl=3600)
        
        mock_pipe.hset.assert_called_once_with("hash:123", "name", "Jane")
        mock_pipe.expire.assert_called_once_with("hash:123", 3600)
    
    def test_execute_query_simplified(self) -> None:
        """Test simplified execute_query implementation."""
        backend = RedisSyncBackend()
        result = backend.execute_query(None)
        assert result == []
    
    def test_count_query_simplified(self) -> None:
        """Test simplified count_query implementation."""
        backend = RedisSyncBackend()
        result = backend.count_query(None)
        assert result == 0
    
    def test_delete_query_simplified(self) -> None:
        """Test simplified delete_query implementation."""
        backend = RedisSyncBackend()
        result = backend.delete_query(None)
        assert result == 0
    
    def test_update_query_simplified(self) -> None:
        """Test simplified update_query implementation."""
        backend = RedisSyncBackend()
        result = backend.update_query(None, {})
        assert result == 0
    
    @patch('redis.from_url')
    def test_redis_connection_error(self, mock_redis_from_url: MagicMock) -> None:
        """Test Redis connection error handling."""
        mock_redis_from_url.side_effect = redis.ConnectionError("Connection failed")
        
        backend = RedisSyncBackend()
        
        with pytest.raises(redis.ConnectionError, match="Connection failed"):
            backend.connect()
    
    def test_save_unknown_model_type_error(self) -> None:
        """Test error when saving unknown model type."""
        backend = RedisSyncBackend()
        
        class UnknownModel:
            def get_key(self):
                return "unknown:test"
        
        instance = UnknownModel()
        
        with patch.object(backend, '_get_client'):
            with pytest.raises(ValueError, match="Unknown model type"):
                backend.save(instance)
    
    def test_get_by_key_unknown_model_type_error(self) -> None:
        """Test error when getting unknown model type by key."""
        backend = RedisSyncBackend()
        
        class UnknownModel:
            _key_prefix = "unknown:"
        
        with patch.object(backend, '_get_client'):
            with patch('data_bridge.redis.sync.backend.RedisKeyPattern.build_key') as mock_build:
                with patch('data_bridge.redis.sync.backend.issubclass', return_value=False):
                    mock_build.return_value = "unknown:test"
                    
                    with pytest.raises(ValueError, match="Unknown model type"):
                        backend.get_by_key(UnknownModel, "test")
    
    @patch('redis.from_url')
    def test_save_hash_model_with_complex_types(self, mock_redis_from_url: MagicMock) -> None:
        """Test saving a HashModel with complex types (dict/list) that get JSON serialized."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_pipe = MagicMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.pipeline.return_value.__enter__.return_value = mock_pipe
        mock_pipe.execute.return_value = [3, True]
        
        # Create a real HashModel instance with complex data
        from data_bridge.redis.sync.hash_model import HashModel
        
        # Create a proper HashModel subclass for testing
        class TestHashModel(HashModel):
            _key_prefix = "test:"
            
            def __init__(self, **data):
                for key, value in data.items():
                    setattr(self, key, value)
            
            def get_key(self):
                return f"test:{getattr(self, 'id', 'test')}"
            
            def to_dict(self):
                return {
                    "name": "John",
                    "metadata": {"key": "value"},  # This should trigger JSON serialization
                    "tags": ["tag1", "tag2"]       # This should trigger JSON serialization
                }
        
        instance = TestHashModel(name="John")
        backend = RedisSyncBackend()
        backend.save(instance)
        
        # Verify complex types are JSON serialized
        call_args = mock_pipe.hset.call_args
        mapping = call_args[1]["mapping"]
        assert mapping["name"] == "John"
        assert mapping["metadata"] == '{"key": "value"}'
        assert mapping["tags"] == '["tag1", "tag2"]'
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.isinstance')
    def test_save_hash_model_with_ttl(self, mock_isinstance: MagicMock, mock_redis_from_url: MagicMock) -> None:
        """Test saving a HashModel with TTL."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_pipe = MagicMock()
        mock_redis_from_url.return_value = mock_client
        mock_client.pipeline.return_value.__enter__.return_value = mock_pipe
        mock_pipe.execute.return_value = [1, True]
        
        # Configure isinstance mock for HashModel
        def isinstance_side_effect(obj, cls):
            return hasattr(cls, '__name__') and cls.__name__ == 'HashModel'
        
        mock_isinstance.side_effect = isinstance_side_effect
        
        instance = MockHashModel(name="John")
        backend = RedisSyncBackend()
        backend.save(instance, ttl=3600)
        
        # Verify TTL is set
        mock_pipe.hset.assert_called_once()
        mock_pipe.expire.assert_called_once_with("hash:test", 3600)
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.isinstance')
    def test_save_json_model_with_redisjson_and_ttl(self, mock_isinstance: MagicMock, mock_redis_from_url: MagicMock) -> None:
        """Test saving a JSONModel with RedisJSON and TTL."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_json = MagicMock()
        mock_client.json.return_value = mock_json
        mock_redis_from_url.return_value = mock_client
        
        # Configure isinstance mock for JSONModel
        def isinstance_side_effect(obj, cls):
            if hasattr(cls, '__name__'):
                return cls.__name__ == 'JSONModel'
            return False
        
        mock_isinstance.side_effect = isinstance_side_effect
        
        instance = MockJSONModel(name="Jane")
        backend = RedisSyncBackend()
        backend.save(instance, ttl=1800)
        
        # Verify RedisJSON set and TTL
        mock_json.set.assert_called_once_with("json:test", "$", {"name": "Jane"})
        mock_client.expire.assert_called_once_with("json:test", 1800)
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.isinstance')
    def test_save_json_model_fallback_with_ttl(self, mock_isinstance: MagicMock, mock_redis_from_url: MagicMock) -> None:
        """Test saving a JSONModel with fallback and TTL."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        
        # Simulate RedisJSON not available
        mock_client.json.side_effect = AttributeError("No JSON module")
        
        # Configure isinstance mock
        def isinstance_side_effect(obj, cls):
            if hasattr(cls, '__name__'):
                return cls.__name__ == 'JSONModel'
            return False
        
        mock_isinstance.side_effect = isinstance_side_effect
        
        instance = MockJSONModel(name="Jane")
        backend = RedisSyncBackend()
        backend.save(instance, ttl=1800)
        
        # Verify fallback with TTL
        expected_json = json.dumps({"name": "Jane"})
        mock_client.setex.assert_called_once_with("json:test", 1800, expected_json)
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.issubclass')
    @patch('data_bridge.redis.sync.backend.RedisKeyPattern')
    def test_get_by_key_hash_model_empty(self, mock_key_pattern: MagicMock, mock_issubclass: MagicMock, 
                                        mock_redis_from_url: MagicMock) -> None:
        """Test getting a HashModel when data is empty."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "hash:123"
        
        # Empty data
        mock_client.hgetall.return_value = {}
        
        # Configure issubclass mock
        mock_issubclass.side_effect = lambda cls, parent: parent.__name__ == 'HashModel'
        
        backend = RedisSyncBackend()
        result = backend.get_by_key(MockHashModel, "123")
        
        assert result is None
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.issubclass')
    @patch('data_bridge.redis.sync.backend.RedisKeyPattern')
    def test_get_by_key_hash_model_with_types(self, mock_key_pattern: MagicMock, mock_issubclass: MagicMock, 
                                             mock_redis_from_url: MagicMock) -> None:
        """Test getting a HashModel with various type conversions."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "hash:123"
        
        # Mock Redis hash data with various types
        mock_client.hgetall.return_value = {
            b"name": b"John",
            b"age": b"30", 
            b"score": b"95.5",
            b"active": b"true",
            b"tags": b'["python", "redis"]',
            b"metadata": b'{"key": "value"}',
            b"unknown": b"some_value"
        }
        
        # Add score and unknown fields to MockHashModel for this test
        MockHashModel._fields = {
            "name": MagicMock(type_=str),
            "age": MagicMock(type_=int),
            "score": MagicMock(type_=float),
            "active": MagicMock(type_=bool),
            "tags": MagicMock(type_=list),
            "metadata": MagicMock(type_=dict),
            "unknown": MagicMock(type_=object)
        }
        
        # Configure issubclass mock
        mock_issubclass.side_effect = lambda cls, parent: parent.__name__ == 'HashModel'
        
        backend = RedisSyncBackend()
        result = backend.get_by_key(MockHashModel, "123")
        
        assert result is not None
        assert isinstance(result, MockHashModel)
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.issubclass')
    @patch('data_bridge.redis.sync.backend.RedisKeyPattern')
    def test_get_by_key_json_model_with_redisjson_none(self, mock_key_pattern: MagicMock, 
                                                       mock_issubclass: MagicMock, 
                                                       mock_redis_from_url: MagicMock) -> None:
        """Test getting a JSONModel with RedisJSON when data is None."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_json = MagicMock()
        mock_client.json.return_value = mock_json
        mock_json.get.return_value = None
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "json:123"
        
        # Configure issubclass mock
        mock_issubclass.side_effect = lambda cls, parent: parent.__name__ == 'JSONModel'
        
        backend = RedisSyncBackend()
        result = backend.get_by_key(MockJSONModel, "123")
        
        assert result is None
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.issubclass')
    @patch('data_bridge.redis.sync.backend.RedisKeyPattern')
    def test_get_by_key_json_model_fallback_none(self, mock_key_pattern: MagicMock, 
                                                 mock_issubclass: MagicMock, 
                                                 mock_redis_from_url: MagicMock) -> None:
        """Test getting a JSONModel with fallback when data is None."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "json:123"
        
        # Simulate RedisJSON not available
        mock_client.json.side_effect = AttributeError("No JSON module")
        # No data found
        mock_client.get.return_value = None
        
        # Configure issubclass mock
        mock_issubclass.side_effect = lambda cls, parent: parent.__name__ == 'JSONModel'
        
        backend = RedisSyncBackend()
        result = backend.get_by_key(MockJSONModel, "123")
        
        assert result is None
    
    @patch('redis.from_url')
    @patch('data_bridge.redis.sync.backend.issubclass')
    @patch('data_bridge.redis.sync.backend.RedisKeyPattern')
    def test_get_by_key_json_model_fallback_with_data(self, mock_key_pattern: MagicMock, 
                                                      mock_issubclass: MagicMock, 
                                                      mock_redis_from_url: MagicMock) -> None:
        """Test getting a JSONModel with fallback and data."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_key_pattern.build_key.return_value = "json:123"
        
        # Simulate RedisJSON not available
        mock_client.json.side_effect = AttributeError("No JSON module")
        # Return JSON string
        mock_client.get.return_value = '{"name": "Jane", "age": 25}'
        
        # Configure issubclass mock
        mock_issubclass.side_effect = lambda cls, parent: parent.__name__ == 'JSONModel'
        
        backend = RedisSyncBackend()
        result = backend.get_by_key(MockJSONModel, "123")
        
        assert result is not None
        assert isinstance(result, MockJSONModel)
    
    @patch('redis.from_url')
    def test_get_field_none(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a field that returns None."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = None
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "nonexistent")
        
        assert result is None
    
    @patch('redis.from_url')
    def test_get_field_float(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a float field from HashModel."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b"95.5"
        
        # Add float field to MockHashModel
        MockHashModel._fields["score"] = MagicMock(type_=float)
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "score")
        
        assert result == 95.5
        assert isinstance(result, float)
    
    @patch('redis.from_url')
    def test_get_field_bool(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a bool field from HashModel."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b"false"
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "active")
        
        assert result is False
        assert isinstance(result, bool)
    
    @patch('redis.from_url')
    def test_get_field_dict(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a dict field from HashModel."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b'{"key": "value"}'
        
        # Add dict field to MockHashModel
        MockHashModel._fields["metadata"] = MagicMock(type_=dict)
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "metadata")
        
        assert result == {"key": "value"}
        assert isinstance(result, dict)
    
    @patch('redis.from_url')
    def test_get_field_list(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a list field from HashModel."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b'["tag1", "tag2"]'
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "tags")
        
        assert result == ["tag1", "tag2"]
        assert isinstance(result, list)
    
    @patch('redis.from_url')
    def test_get_field_unknown_type(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a field with unknown type."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b"some_value"
        
        # Add unknown type field
        MockHashModel._fields["unknown"] = MagicMock(type_=object)
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "unknown")
        
        assert result == "some_value"
        assert isinstance(result, str)
    
    @patch('redis.from_url')
    def test_get_field_no_field_definition(self, mock_redis_from_url: MagicMock) -> None:
        """Test getting a field with no field definition."""
        mock_client = MagicMock(spec=redis.Redis)
        mock_redis_from_url.return_value = mock_client
        mock_client.hget.return_value = b"some_value"
        
        instance = MockHashModel(id="123")
        backend = RedisSyncBackend()
        result = backend.get_field(instance, "nonexistent_field")
        
        assert result == "some_value"