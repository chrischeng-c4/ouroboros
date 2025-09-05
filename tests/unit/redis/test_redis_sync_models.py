"""Tests for Redis synchronous models (HashModel and JSONModel)."""

from unittest.mock import MagicMock, patch

import pytest

from data_bridge.base.fields import StringField, IntField
from data_bridge.redis.sync.hash_model import HashModel
from data_bridge.redis.sync.json_model import JSONModel
from data_bridge.redis.sync.backend import RedisSyncBackend
from data_bridge.redis.sync.manager import RedisManager


class TestHashModel(HashModel):
    """Test HashModel for unit tests."""
    _key_prefix = "user:"
    _default_ttl = 3600
    
    id = StringField(primary_key=True, required=False)
    name = StringField(required=False)
    age = IntField(required=False)


class TestJSONModel(JSONModel):
    """Test JSONModel for unit tests."""
    _key_prefix = "profile:"
    _default_ttl = 7200
    
    id = StringField(primary_key=True, required=False)
    name = StringField(required=False)
    age = IntField(required=False)


class TestRedisHashModel:
    """Test HashModel functionality."""
    
    def setup_method(self):
        """Set up test environment."""
        # Reset backend for each test
        TestHashModel._backend = None
    
    def test_objects_returns_manager(self):
        """Test that objects() returns a RedisManager instance."""
        manager = TestHashModel.objects()
        assert isinstance(manager, RedisManager)
        assert manager.model_class == TestHashModel
    
    def test_set_backend(self):
        """Test setting backend."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestHashModel.set_backend(backend)
        assert TestHashModel._backend == backend
    
    def test_get_key_with_primary_key(self):
        """Test getting Redis key when primary key is set."""
        instance = TestHashModel(id="123", name="John")
        key = instance.get_key()
        assert key == "user:123"
    
    def test_get_key_no_primary_key_field(self):
        """Test getting key when no primary key field is defined."""
        # Create a model class without primary key field
        class NoPKModel(HashModel):
            _key_prefix = "nopk:"
            name = StringField()  # No primary key field
        
        instance = NoPKModel(name="Test")
        
        with pytest.raises(ValueError, match="Cannot generate key without primary key field"):
            instance.get_key()
    
    def test_get_key_none_primary_key_value(self):
        """Test getting key when primary key value is None."""
        instance = TestHashModel(name="John")
        # Primary key field should exist from metaclass, but set id to None
        instance.id = None
        
        with pytest.raises(ValueError, match="Cannot generate key with None primary key"):
            instance.get_key()
    
    def test_save_with_backend_and_ttl(self):
        """Test saving with backend configured and custom TTL."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestHashModel.set_backend(backend)
        
        instance = TestHashModel(id="123", name="John", age=30)
        instance.save(ttl=1800)
        
        backend.save.assert_called_once_with(instance, 1800)
    
    def test_save_with_default_ttl(self):
        """Test saving with default TTL."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestHashModel.set_backend(backend)
        
        instance = TestHashModel(id="123", name="John")
        instance.save()
        
        backend.save.assert_called_once_with(instance, 3600)  # default TTL
    
    def test_save_no_backend_configured(self):
        """Test saving when no backend is configured."""
        instance = TestHashModel(id="123", name="John")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestHashModel"):
            instance.save()
    
    def test_delete_with_backend(self):
        """Test deleting with backend configured."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestHashModel.set_backend(backend)
        
        instance = TestHashModel(id="123", name="John")
        instance.delete()
        
        backend.delete.assert_called_once_with(instance)
    
    def test_delete_no_backend_configured(self):
        """Test deleting when no backend is configured."""
        instance = TestHashModel(id="123", name="John")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestHashModel"):
            instance.delete()
    
    def test_get_field_with_backend(self):
        """Test getting field value with backend configured."""
        backend = MagicMock(spec=RedisSyncBackend)
        backend.get_field.return_value = "John"
        TestHashModel.set_backend(backend)
        
        instance = TestHashModel(id="123")
        result = instance.get_field("name")
        
        backend.get_field.assert_called_once_with(instance, "name")
        assert result == "John"
    
    def test_get_field_no_backend_configured(self):
        """Test getting field when no backend is configured."""
        instance = TestHashModel(id="123")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestHashModel"):
            instance.get_field("name")
    
    def test_set_field_with_backend(self):
        """Test setting field value with backend configured."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestHashModel.set_backend(backend)
        
        instance = TestHashModel(id="123", name="Old Name")
        instance.set_field("name", "New Name", ttl=1800)
        
        backend.set_field.assert_called_once_with(instance, "name", "New Name", 1800)
        assert instance.name == "New Name"  # Local instance should be updated
    
    def test_set_field_no_backend_configured(self):
        """Test setting field when no backend is configured."""
        instance = TestHashModel(id="123")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestHashModel"):
            instance.set_field("name", "New Name")
    
    def test_get_class_method_with_backend(self):
        """Test getting instance by primary key using class method."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_instance = TestHashModel(id="123", name="John")
        backend.get_by_key.return_value = test_instance
        TestHashModel.set_backend(backend)
        
        result = TestHashModel.get("123")
        
        backend.get_by_key.assert_called_once_with(TestHashModel, "123")
        assert result == test_instance
    
    def test_get_class_method_not_found(self):
        """Test getting instance when not found."""
        backend = MagicMock(spec=RedisSyncBackend)
        backend.get_by_key.return_value = None
        TestHashModel.set_backend(backend)
        
        result = TestHashModel.get("nonexistent")
        assert result is None
    
    def test_get_class_method_no_backend_configured(self):
        """Test getting instance when no backend is configured."""
        with pytest.raises(RuntimeError, match="No backend configured for TestHashModel"):
            TestHashModel.get("123")
    
    def test_exists_with_backend(self):
        """Test checking existence with backend configured."""
        backend = MagicMock(spec=RedisSyncBackend)
        backend.exists.return_value = True
        TestHashModel.set_backend(backend)
        
        instance = TestHashModel(id="123", name="John")
        result = instance.exists()
        
        backend.exists.assert_called_once_with(instance)
        assert result is True
    
    def test_exists_no_backend_configured(self):
        """Test checking existence when no backend is configured."""
        instance = TestHashModel(id="123")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestHashModel"):
            instance.exists()
    
    def test_default_ttl_none(self):
        """Test model with no default TTL."""
        class NoTTLModel(HashModel):
            _key_prefix = "test:"
            _default_ttl = None
            id = StringField(primary_key=True)
        
        backend = MagicMock(spec=RedisSyncBackend)
        NoTTLModel.set_backend(backend)
        
        instance = NoTTLModel(id="123")
        instance.save()
        
        backend.save.assert_called_once_with(instance, None)


class TestRedisJSONModel:
    """Test JSONModel functionality."""
    
    def setup_method(self):
        """Set up test environment."""
        # Reset backend for each test
        TestJSONModel._backend = None
    
    def test_objects_returns_manager(self):
        """Test that objects() returns a RedisManager instance."""
        manager = TestJSONModel.objects()
        assert isinstance(manager, RedisManager)
        assert manager.model_class == TestJSONModel
    
    def test_set_backend(self):
        """Test setting backend."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestJSONModel.set_backend(backend)
        assert TestJSONModel._backend == backend
    
    def test_get_key_with_primary_key(self):
        """Test getting Redis key when primary key is set."""
        instance = TestJSONModel(id="456", name="Jane")
        key = instance.get_key()
        assert key == "profile:456"
    
    def test_save_with_backend_and_ttl(self):
        """Test saving with backend configured and custom TTL."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestJSONModel.set_backend(backend)
        
        instance = TestJSONModel(id="456", name="Jane", age=25)
        instance.save(ttl=3600)
        
        backend.save.assert_called_once_with(instance, 3600)
    
    def test_save_with_default_ttl(self):
        """Test saving with default TTL."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestJSONModel.set_backend(backend)
        
        instance = TestJSONModel(id="456", name="Jane")
        instance.save()
        
        backend.save.assert_called_once_with(instance, 7200)  # default TTL
    
    def test_delete_with_backend(self):
        """Test deleting with backend configured."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestJSONModel.set_backend(backend)
        
        instance = TestJSONModel(id="456", name="Jane")
        instance.delete()
        
        backend.delete.assert_called_once_with(instance)
    
    def test_get_class_method_with_backend(self):
        """Test getting instance by primary key using class method."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_instance = TestJSONModel(id="456", name="Jane")
        backend.get_by_key.return_value = test_instance
        TestJSONModel.set_backend(backend)
        
        result = TestJSONModel.get("456")
        
        backend.get_by_key.assert_called_once_with(TestJSONModel, "456")
        assert result == test_instance
    
    def test_exists_with_backend(self):
        """Test checking existence with backend configured."""
        backend = MagicMock(spec=RedisSyncBackend)
        backend.exists.return_value = False
        TestJSONModel.set_backend(backend)
        
        instance = TestJSONModel(id="456", name="Jane")
        result = instance.exists()
        
        backend.exists.assert_called_once_with(instance)
        assert result is False
    
    def test_get_key_with_primary_key_json(self):
        """Test getting Redis key when primary key is set for JSONModel."""
        instance = TestJSONModel(id="789", name="Bob")
        key = instance.get_key()
        assert key == "profile:789"
    
    def test_get_key_no_primary_key_field_json(self):
        """Test getting key when no primary key field is defined for JSONModel."""
        class NoPKJSONModel(JSONModel):
            _key_prefix = "nopk:"
            name = StringField()  # No primary key field
        
        instance = NoPKJSONModel(name="Test")
        
        with pytest.raises(ValueError, match="Cannot generate key without primary key field"):
            instance.get_key()
    
    def test_get_key_none_primary_key_value_json(self):
        """Test getting key when primary key value is None for JSONModel."""
        instance = TestJSONModel(name="Jane")
        instance.id = None
        
        with pytest.raises(ValueError, match="Cannot generate key with None primary key"):
            instance.get_key()
    
    def test_save_no_backend_configured_json(self):
        """Test saving JSONModel when no backend is configured."""
        instance = TestJSONModel(id="456", name="Jane")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestJSONModel"):
            instance.save()
    
    def test_delete_no_backend_configured_json(self):
        """Test deleting JSONModel when no backend is configured."""
        instance = TestJSONModel(id="456", name="Jane")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestJSONModel"):
            instance.delete()
    
    def test_update_path_with_backend(self):
        """Test updating JSONPath with backend configured."""
        backend = MagicMock()
        TestJSONModel.set_backend(backend)
        
        instance = TestJSONModel(id="456", name="Jane")
        instance.update_path("$.name", "Updated Jane")
        
        backend.update_json_path.assert_called_once_with(instance, "$.name", "Updated Jane")
    
    def test_update_path_no_backend_configured(self):
        """Test updating JSONPath when no backend is configured."""
        instance = TestJSONModel(id="456", name="Jane")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestJSONModel"):
            instance.update_path("$.name", "Updated Jane")
    
    def test_get_path_with_backend(self):
        """Test getting JSONPath value with backend configured."""
        backend = MagicMock()
        backend.get_json_path.return_value = "Jane"
        TestJSONModel.set_backend(backend)
        
        instance = TestJSONModel(id="456", name="Jane")
        result = instance.get_path("$.name")
        
        backend.get_json_path.assert_called_once_with(instance, "$.name")
        assert result == "Jane"
    
    def test_get_path_no_backend_configured(self):
        """Test getting JSONPath when no backend is configured."""
        instance = TestJSONModel(id="456", name="Jane")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestJSONModel"):
            instance.get_path("$.name")
    
    def test_get_class_method_no_backend_configured_json(self):
        """Test getting JSONModel instance when no backend is configured."""
        with pytest.raises(RuntimeError, match="No backend configured for TestJSONModel"):
            TestJSONModel.get("456")
    
    def test_find_class_method_with_backend(self):
        """Test finding JSONModel instances with backend configured."""
        backend = MagicMock()
        test_instances = [
            TestJSONModel(id="456", name="Jane"),
            TestJSONModel(id="457", name="John")
        ]
        backend.find_by_json_filter.return_value = test_instances
        TestJSONModel.set_backend(backend)
        
        json_filters = {"$.status": "active"}
        result = TestJSONModel.find(json_filters)
        
        backend.find_by_json_filter.assert_called_once_with(TestJSONModel, json_filters)
        assert result == test_instances
    
    def test_find_class_method_no_backend_configured(self):
        """Test finding JSONModel instances when no backend is configured."""
        with pytest.raises(RuntimeError, match="No backend configured for TestJSONModel"):
            TestJSONModel.find({"$.status": "active"})
    
    def test_exists_no_backend_configured_json(self):
        """Test checking existence of JSONModel when no backend is configured."""
        instance = TestJSONModel(id="456", name="Jane")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestJSONModel"):
            instance.exists()
    
    def test_json_model_inheritance_from_base_model(self):
        """Test that JSONModel properly inherits from BaseModel."""
        instance = TestJSONModel(id="456", name="Jane", age=25)
        
        # Test to_dict functionality
        data = instance.to_dict()
        assert data["id"] == "456"
        assert data["name"] == "Jane" 
        assert data["age"] == 25
        
        # Test from_dict functionality
        new_instance = TestJSONModel.from_dict(data)
        assert new_instance.id == "456"
        assert new_instance.name == "Jane"
        assert new_instance.age == 25
    
    def test_hash_model_inheritance_from_base_model(self):
        """Test that HashModel properly inherits from BaseModel."""
        instance = TestHashModel(id="123", name="John", age=30)
        
        # Test to_dict functionality
        data = instance.to_dict()
        assert data["id"] == "123"
        assert data["name"] == "John"
        assert data["age"] == 30
        
        # Test from_dict functionality
        new_instance = TestHashModel.from_dict(data)
        assert new_instance.id == "123"
        assert new_instance.name == "John"
        assert new_instance.age == 30


class TestRedisModelsEdgeCases:
    """Test edge cases and error conditions for Redis models."""
    
    def test_model_without_primary_key_field(self):
        """Test model that doesn't have a primary key field."""
        class NoPKModel(HashModel):
            _key_prefix = "nopk:"
            name = StringField()  # No primary key field
        
        instance = NoPKModel(name="Test")
        
        with pytest.raises(ValueError, match="Cannot generate key without primary key field"):
            instance.get_key()
    
    def test_multiple_inheritance_edge_case(self):
        """Test that models can be extended properly."""
        class ExtendedHashModel(TestHashModel):
            email = StringField()
        
        backend = MagicMock(spec=RedisSyncBackend)
        ExtendedHashModel.set_backend(backend)
        
        instance = ExtendedHashModel(id="123", name="John", email="john@example.com")
        key = instance.get_key()
        
        # Should still use the base class prefix
        assert key == "user:123"
        
        # Should work with save
        instance.save()
        backend.save.assert_called_once_with(instance, 3600)
    
    def test_concurrent_backend_setting(self):
        """Test that backend setting affects the correct model class."""
        backend1 = MagicMock(spec=RedisSyncBackend)
        backend2 = MagicMock(spec=RedisSyncBackend)
        
        TestHashModel.set_backend(backend1)
        TestJSONModel.set_backend(backend2)
        
        # Each model should have its own backend
        assert TestHashModel._backend == backend1
        assert TestJSONModel._backend == backend2
        
        # Test that operations use the correct backend
        hash_instance = TestHashModel(id="123")
        json_instance = TestJSONModel(id="456")
        
        hash_instance.save()
        json_instance.save()
        
        backend1.save.assert_called_once_with(hash_instance, 3600)
        backend2.save.assert_called_once_with(json_instance, 7200)