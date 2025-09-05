"""Tests for Redis synchronous manager."""

from unittest.mock import MagicMock, patch

import pytest

from data_bridge.base.fields import StringField, IntField
from data_bridge.redis.sync.hash_model import HashModel
from data_bridge.redis.sync.json_model import JSONModel
from data_bridge.redis.sync.backend import RedisSyncBackend
from data_bridge.redis.sync.manager import RedisManager


class TestUser(HashModel):
    """Test user model for manager tests."""
    _key_prefix = "user:"
    _default_ttl = 3600
    
    id = StringField(primary_key=True, required=False)
    name = StringField(required=False)
    age = IntField(required=False)


class TestProfile(JSONModel):
    """Test profile model for manager tests."""
    _key_prefix = "profile:"
    _default_ttl = 7200
    
    id = StringField(primary_key=True, required=False)
    name = StringField(required=False)
    age = IntField(required=False)


class TestRedisManager:
    """Test RedisManager functionality."""
    
    def setup_method(self):
        """Set up test environment."""
        # Reset backend for each test
        TestUser._backend = None
        TestProfile._backend = None
    
    def test_manager_initialization(self):
        """Test manager initialization."""
        manager = RedisManager(TestUser)
        assert manager.model_class == TestUser
    
    def test_find_with_backend_configured(self):
        """Test find method with backend configured."""
        backend = MagicMock()
        test_users = [
            TestUser(id="1", name="John", age=30),
            TestUser(id="2", name="Jane", age=25)
        ]
        backend.find_with_expressions.return_value = test_users
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        expressions = [MagicMock()]  # Mock query expressions
        result = manager.find(*expressions)
        
        backend.find_with_expressions.assert_called_once_with(TestUser, expressions)
        assert result == test_users
    
    def test_find_no_backend_configured(self):
        """Test find method when no backend is configured."""
        manager = RedisManager(TestUser)
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            manager.find()
    
    def test_all_with_backend_configured(self):
        """Test all method with backend configured."""
        backend = MagicMock()
        test_users = [
            TestUser(id="1", name="John", age=30),
            TestUser(id="2", name="Jane", age=25),
            TestUser(id="3", name="Bob", age=35)
        ]
        backend.get_all.return_value = test_users
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        result = manager.all()
        
        backend.get_all.assert_called_once_with(TestUser)
        assert result == test_users
    
    def test_all_no_backend_configured(self):
        """Test all method when no backend is configured."""
        manager = RedisManager(TestUser)
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            manager.all()
    
    def test_create_with_ttl(self):
        """Test create method with custom TTL."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        # Mock the save method on the created instance
        with patch.object(TestUser, 'save') as mock_save:
            result = manager.create(ttl=1800, id="123", name="John", age=30)
            
            assert isinstance(result, TestUser)
            assert result.id == "123"
            assert result.name == "John"
            assert result.age == 30
            mock_save.assert_called_once_with(ttl=1800)
    
    def test_create_without_ttl(self):
        """Test create method without TTL (uses default)."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(TestUser, 'save') as mock_save:
            result = manager.create(id="123", name="John", age=30)
            
            assert isinstance(result, TestUser)
            mock_save.assert_called_once_with(ttl=None)
    
    def test_get_by_primary_key(self):
        """Test get method with primary key lookup."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_user = TestUser(id="123", name="John", age=30)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(TestUser, 'get', return_value=test_user) as mock_get:
            result = manager.get(id="123")
            
            mock_get.assert_called_once_with("123")
            assert result == test_user
    
    def test_get_by_primary_key_not_found(self):
        """Test get method with primary key lookup when not found."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(TestUser, 'get', return_value=None):
            with pytest.raises(ValueError, match="No TestUser found with"):
                manager.get(id="nonexistent")
    
    def test_get_by_non_primary_key_field(self):
        """Test get method with non-primary key field."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_user = TestUser(id="123", name="John", age=30)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        # Mock the find method to return results
        with patch.object(manager, 'find', return_value=[test_user]):
            result = manager.get(name="John")
            
            assert result == test_user
    
    def test_get_by_non_primary_key_multiple_results(self):
        """Test get method when multiple results are found."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_users = [
            TestUser(id="1", name="John", age=30),
            TestUser(id="2", name="John", age=25)  # Same name, different age/id
        ]
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(manager, 'find', return_value=test_users):
            with pytest.raises(ValueError, match="Multiple TestUser found with"):
                manager.get(name="John")
    
    def test_get_by_non_primary_key_not_found(self):
        """Test get method with non-primary key when not found."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(manager, 'find', return_value=[]):
            with pytest.raises(ValueError, match="No TestUser found with"):
                manager.get(name="NonExistent")
    
    def test_get_or_create_existing(self):
        """Test get_or_create when object exists."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_user = TestUser(id="123", name="John", age=30)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(manager, 'get', return_value=test_user):
            result, created = manager.get_or_create(id="123")
            
            assert result == test_user
            assert created is False
    
    def test_get_or_create_new_without_defaults(self):
        """Test get_or_create when creating new object without defaults."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        # Mock get to raise ValueError (not found)
        with patch.object(manager, 'get', side_effect=ValueError("Not found")):
            with patch.object(manager, 'create') as mock_create:
                test_user = TestUser(id="123", name="John")
                mock_create.return_value = test_user
                
                result, created = manager.get_or_create(id="123", name="John")
                
                mock_create.assert_called_once_with(ttl=None, id="123", name="John")
                assert result == test_user
                assert created is True
    
    def test_get_or_create_new_with_defaults(self):
        """Test get_or_create when creating new object with defaults."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(manager, 'get', side_effect=ValueError("Not found")):
            with patch.object(manager, 'create') as mock_create:
                test_user = TestUser(id="123", name="John", age=30)
                mock_create.return_value = test_user
                
                result, created = manager.get_or_create(
                    defaults={"age": 30},
                    ttl=1800,
                    id="123",
                    name="John"
                )
                
                mock_create.assert_called_once_with(
                    ttl=1800,
                    id="123",
                    name="John",
                    age=30
                )
                assert result == test_user
                assert created is True
    
    def test_count(self):
        """Test count method."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_users = [
            TestUser(id="1", name="John", age=30),
            TestUser(id="2", name="Jane", age=25),
            TestUser(id="3", name="Bob", age=35)
        ]
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(manager, 'all', return_value=test_users):
            result = manager.count()
            assert result == 3
    
    def test_count_empty(self):
        """Test count method when no objects exist."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        with patch.object(manager, 'all', return_value=[]):
            result = manager.count()
            assert result == 0
    
    def test_exists_with_backend_configured(self):
        """Test exists method with backend configured."""
        backend = MagicMock()
        backend.any_exists.return_value = True
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        result = manager.exists()
        
        backend.any_exists.assert_called_once_with(TestUser)
        assert result is True
    
    def test_exists_no_backend_configured(self):
        """Test exists method when no backend is configured."""
        manager = RedisManager(TestUser)
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            manager.exists()
    
    def test_delete_all_with_backend_configured(self):
        """Test delete_all method with backend configured."""
        backend = MagicMock()
        backend.delete_all.return_value = 5  # 5 objects deleted
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        result = manager.delete_all()
        
        backend.delete_all.assert_called_once_with(TestUser)
        assert result == 5
    
    def test_delete_all_no_backend_configured(self):
        """Test delete_all method when no backend is configured."""
        manager = RedisManager(TestUser)
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            manager.delete_all()
    
    def test_manager_with_json_model(self):
        """Test manager works with JSONModel as well."""
        backend = MagicMock(spec=RedisSyncBackend)
        TestProfile.set_backend(backend)
        
        manager = RedisManager(TestProfile)
        assert manager.model_class == TestProfile
        
        # Test create with JSONModel
        with patch.object(TestProfile, 'save') as mock_save:
            result = manager.create(id="prof1", name="Jane", age=25)
            
            assert isinstance(result, TestProfile)
            assert result.id == "prof1"
            mock_save.assert_called_once_with(ttl=None)
    
    def test_manager_create_field_expressions_method(self):
        """Test the _create_field_expressions method (inherited from base)."""
        manager = RedisManager(TestUser)
        
        # Test that the method exists and can be called
        # The actual implementation is in the base manager
        expressions = manager._create_field_expressions(name="John", age=30)
        
        # Should return a list of expressions
        assert isinstance(expressions, list)
        assert len(expressions) == 2
    
    def test_manager_edge_case_multiple_kwargs_get(self):
        """Test get method with multiple non-primary-key arguments."""
        backend = MagicMock(spec=RedisSyncBackend)
        test_user = TestUser(id="123", name="John", age=30)
        TestUser.set_backend(backend)
        
        manager = RedisManager(TestUser)
        
        # When multiple kwargs are provided, should use expression-based search
        with patch.object(manager, 'find', return_value=[test_user]):
            result = manager.get(name="John", age=30)
            assert result == test_user
    
    def test_manager_inheritance_and_polymorphism(self):
        """Test that manager works correctly with model inheritance."""
        class ExtendedUser(TestUser):
            email = StringField()
        
        backend = MagicMock(spec=RedisSyncBackend)
        ExtendedUser.set_backend(backend)
        
        manager = RedisManager(ExtendedUser)
        assert manager.model_class == ExtendedUser
        
        # Test that create works with extended model
        with patch.object(ExtendedUser, 'save') as mock_save:
            result = manager.create(id="123", name="John", email="john@example.com")
            
            assert isinstance(result, ExtendedUser)
            assert result.email == "john@example.com"
            mock_save.assert_called_once()


class TestRedisManagerErrorHandling:
    """Test error handling and edge cases for RedisManager."""
    
    def test_manager_with_model_without_backend_methods(self):
        """Test manager behavior when backend doesn't have expected methods."""
        backend = MagicMock()
        # Remove some expected methods to test error handling
        del backend.find_with_expressions
        
        TestUser.set_backend(backend)
        manager = RedisManager(TestUser)
        
        # Should raise AttributeError when backend doesn't have expected method
        with pytest.raises(AttributeError):
            manager.find()
    
    def test_manager_concurrent_operations(self):
        """Test that manager operations don't interfere with each other."""
        backend1 = MagicMock()
        backend2 = MagicMock()
        
        TestUser.set_backend(backend1)
        TestProfile.set_backend(backend2)
        
        user_manager = RedisManager(TestUser)
        profile_manager = RedisManager(TestProfile)
        
        # Operations on different managers should use different backends
        backend1.get_all.return_value = []
        backend2.get_all.return_value = []
        
        user_manager.all()
        profile_manager.all()
        
        backend1.get_all.assert_called_once_with(TestUser)
        backend2.get_all.assert_called_once_with(TestProfile)
    
    def test_manager_with_none_backend(self):
        """Test manager behavior when backend is explicitly set to None."""
        TestUser._backend = None
        manager = RedisManager(TestUser)
        
        with pytest.raises(RuntimeError, match="No backend configured"):
            manager.find()
            
        with pytest.raises(RuntimeError, match="No backend configured"):
            manager.all()
            
        with pytest.raises(RuntimeError, match="No backend configured"):
            manager.exists()
            
        with pytest.raises(RuntimeError, match="No backend configured"):
            manager.delete_all()