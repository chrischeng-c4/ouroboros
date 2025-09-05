"""Tests for MongoDB synchronous collection functionality."""

from unittest.mock import MagicMock, patch
import pytest

from data_bridge.base.fields import StringField, IntField, QueryExpression, CompoundExpression
from data_bridge.mongo.sync.document import Document
from data_bridge.mongo.sync.collection import MongoCollection
from data_bridge.mongo.sync.query import MongoQuery
from data_bridge.mongo.sync.backend import MongoSyncBackend


class TestUser(Document):
    """Test user document for collection tests."""
    _collection = "users"
    _database = "test_db"
    
    id = StringField(primary_key=True, required=False)
    name = StringField(required=False)
    age = IntField(required=False)
    email = StringField(required=False)
    active = StringField(required=False)


class TestMongoCollection:
    """Test MongoCollection functionality."""
    
    def setup_method(self):
        """Set up test environment."""
        # Reset backend for each test
        TestUser._backend = None
    
    def test_collection_initialization(self):
        """Test collection manager initialization."""
        collection = MongoCollection(TestUser)
        assert collection.model_class == TestUser
    
    def test_find_with_expressions(self):
        """Test find method with query expressions."""
        collection = MongoCollection(TestUser)
        
        expr1 = MagicMock(spec=QueryExpression)
        expr2 = MagicMock(spec=QueryExpression)
        
        query = collection.find(expr1, expr2)
        
        assert isinstance(query, MongoQuery)
        assert query.model_class == TestUser
        assert expr1 in query.expressions
        assert expr2 in query.expressions
    
    def test_find_with_compound_expressions(self):
        """Test find method with compound expressions."""
        collection = MongoCollection(TestUser)
        
        compound_expr = MagicMock(spec=CompoundExpression)
        
        query = collection.find(compound_expr)
        
        assert isinstance(query, MongoQuery)
        assert compound_expr in query.expressions
    
    def test_find_no_expressions(self):
        """Test find method with no expressions."""
        collection = MongoCollection(TestUser)
        
        query = collection.find()
        
        assert isinstance(query, MongoQuery)
        assert len(query.expressions) == 0
    
    def test_find_one_with_expressions(self):
        """Test find_one method with expressions."""
        collection = MongoCollection(TestUser)
        
        expr = MagicMock(spec=QueryExpression)
        
        with patch.object(MongoQuery, 'first') as mock_first:
            test_user = TestUser(id="123", name="John")
            mock_first.return_value = test_user
            
            result = collection.find_one(expr)
            
            assert result == test_user
            mock_first.assert_called_once()
    
    def test_find_one_no_results(self):
        """Test find_one method when no results found."""
        collection = MongoCollection(TestUser)
        
        expr = MagicMock(spec=QueryExpression)
        
        with patch.object(MongoQuery, 'first') as mock_first:
            mock_first.return_value = None
            
            result = collection.find_one(expr)
            
            assert result is None
    
    def test_all(self):
        """Test all method returns query for all documents."""
        collection = MongoCollection(TestUser)
        
        query = collection.all()
        
        assert isinstance(query, MongoQuery)
        assert query.model_class == TestUser
        assert len(query.expressions) == 0
    
    def test_create_with_backend(self):
        """Test create method with backend configured."""
        backend = MagicMock(spec=MongoSyncBackend)
        TestUser.set_backend(backend)
        
        collection = MongoCollection(TestUser)
        
        with patch.object(TestUser, 'save') as mock_save:
            result = collection.create(id="123", name="John", age=30)
            
            assert isinstance(result, TestUser)
            assert result.id == "123"
            assert result.name == "John"
            assert result.age == 30
            mock_save.assert_called_once()
    
    def test_get_with_single_field(self):
        """Test get method with single field lookup."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, '_create_field_expressions') as mock_create_expr:
            with patch.object(MongoQuery, 'first') as mock_first:
                test_user = TestUser(id="123", name="John")
                mock_expr = MagicMock(spec=QueryExpression)
                mock_create_expr.return_value = [mock_expr]
                mock_first.return_value = test_user
                
                result = collection.get(name="John")
                
                assert result == test_user
                mock_create_expr.assert_called_once_with(name="John")
                mock_first.assert_called_once()
    
    def test_get_with_multiple_fields(self):
        """Test get method with multiple field lookup."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, '_create_field_expressions') as mock_create_expr:
            with patch.object(MongoQuery, 'first') as mock_first:
                test_user = TestUser(id="123", name="John", age=30)
                mock_expr1 = MagicMock(spec=QueryExpression)
                mock_expr2 = MagicMock(spec=QueryExpression)
                mock_create_expr.return_value = [mock_expr1, mock_expr2]
                mock_first.return_value = test_user
                
                result = collection.get(name="John", age=30)
                
                assert result == test_user
                mock_create_expr.assert_called_once_with(name="John", age=30)
    
    def test_get_not_found(self):
        """Test get method when document not found."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, '_create_field_expressions') as mock_create_expr:
            with patch.object(MongoQuery, 'first') as mock_first:
                mock_expr = MagicMock(spec=QueryExpression)
                mock_create_expr.return_value = [mock_expr]
                mock_first.return_value = None
                
                with pytest.raises(ValueError, match="No TestUser found with"):
                    collection.get(name="NonExistent")
    
    def test_get_or_create_existing(self):
        """Test get_or_create when document already exists."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'get') as mock_get:
            test_user = TestUser(id="123", name="John")
            mock_get.return_value = test_user
            
            result, created = collection.get_or_create(name="John")
            
            assert result == test_user
            assert created is False
            mock_get.assert_called_once_with(name="John")
    
    def test_get_or_create_new_without_defaults(self):
        """Test get_or_create when creating new document without defaults."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'get') as mock_get:
            with patch.object(collection, 'create') as mock_create:
                test_user = TestUser(id="123", name="John")
                mock_get.side_effect = ValueError("Not found")
                mock_create.return_value = test_user
                
                result, created = collection.get_or_create(name="John")
                
                assert result == test_user
                assert created is True
                mock_get.assert_called_once_with(name="John")
                mock_create.assert_called_once_with(name="John")
    
    def test_get_or_create_new_with_defaults(self):
        """Test get_or_create when creating new document with defaults."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'get') as mock_get:
            with patch.object(collection, 'create') as mock_create:
                test_user = TestUser(id="123", name="John", age=30, active="true")
                mock_get.side_effect = ValueError("Not found")
                mock_create.return_value = test_user
                
                result, created = collection.get_or_create(
                    defaults={"age": 30, "active": "true"},
                    name="John"
                )
                
                assert result == test_user
                assert created is True
                mock_get.assert_called_once_with(name="John")
                mock_create.assert_called_once_with(name="John", age=30, active="true")
    
    def test_get_or_create_defaults_override(self):
        """Test get_or_create where defaults override lookup fields."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'get') as mock_get:
            with patch.object(collection, 'create') as mock_create:
                test_user = TestUser(id="123", name="John", age=25)  # age from defaults
                mock_get.side_effect = ValueError("Not found")
                mock_create.return_value = test_user
                
                result, created = collection.get_or_create(
                    defaults={"age": 25},  # This should override age=30 in kwargs
                    name="John",
                    age=30
                )
                
                assert result == test_user
                assert created is True
                # Should be called with defaults merged (defaults override kwargs)
                mock_create.assert_called_once_with(name="John", age=25)
    
    def test_count(self):
        """Test count method."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'all') as mock_all:
            mock_query = MagicMock(spec=MongoQuery)
            mock_query.count.return_value = 5
            mock_all.return_value = mock_query
            
            result = collection.count()
            
            assert result == 5
            mock_all.assert_called_once()
            mock_query.count.assert_called_once()
    
    def test_exists_with_documents(self):
        """Test exists method when documents exist."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'count') as mock_count:
            mock_count.return_value = 3
            
            result = collection.exists()
            
            assert result is True
            mock_count.assert_called_once()
    
    def test_exists_no_documents(self):
        """Test exists method when no documents exist."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'count') as mock_count:
            mock_count.return_value = 0
            
            result = collection.exists()
            
            assert result is False
            mock_count.assert_called_once()


class TestMongoCollectionInheritance:
    """Test MongoDB collection inheritance from BaseManager."""
    
    def test_collection_inherits_from_base_manager(self):
        """Test that MongoCollection properly inherits from BaseManager."""
        collection = MongoCollection(TestUser)
        
        # Should have inherited _create_field_expressions method
        assert hasattr(collection, '_create_field_expressions')
        assert callable(collection._create_field_expressions)
        
        # Test the inherited method works
        expressions = collection._create_field_expressions(name="John", age=30)
        
        # Should return a list of expressions
        assert isinstance(expressions, list)
        assert len(expressions) == 2


class TestMongoCollectionEdgeCases:
    """Test edge cases and error conditions for MongoDB collection."""
    
    def test_create_with_no_args(self):
        """Test create method with no arguments."""
        backend = MagicMock(spec=MongoSyncBackend)
        TestUser.set_backend(backend)
        
        collection = MongoCollection(TestUser)
        
        with patch.object(TestUser, 'save') as mock_save:
            result = collection.create()
            
            assert isinstance(result, TestUser)
            # All fields should have default/None values
            mock_save.assert_called_once()
    
    def test_get_or_create_with_empty_defaults(self):
        """Test get_or_create with empty defaults dict."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'get') as mock_get:
            with patch.object(collection, 'create') as mock_create:
                test_user = TestUser(id="123", name="John")
                mock_get.side_effect = ValueError("Not found")
                mock_create.return_value = test_user
                
                result, created = collection.get_or_create(defaults={}, name="John")
                
                assert result == test_user
                assert created is True
                # Should just pass through the kwargs
                mock_create.assert_called_once_with(name="John")
    
    def test_get_or_create_with_none_defaults(self):
        """Test get_or_create with None defaults."""
        collection = MongoCollection(TestUser)
        
        with patch.object(collection, 'get') as mock_get:
            with patch.object(collection, 'create') as mock_create:
                test_user = TestUser(id="123", name="John")
                mock_get.side_effect = ValueError("Not found")
                mock_create.return_value = test_user
                
                result, created = collection.get_or_create(defaults=None, name="John")
                
                assert result == test_user
                assert created is True
                # Should just pass through the kwargs
                mock_create.assert_called_once_with(name="John")
    
    def test_collection_with_extended_model(self):
        """Test collection works with extended model classes."""
        class ExtendedUser(TestUser):
            email = StringField(required=False)
            active = StringField(required=False)
        
        collection = MongoCollection(ExtendedUser)
        assert collection.model_class == ExtendedUser
        
        # Test create works with extended model
        backend = MagicMock(spec=MongoSyncBackend)
        ExtendedUser.set_backend(backend)
        
        with patch.object(ExtendedUser, 'save') as mock_save:
            result = collection.create(id="123", name="John", email="john@example.com")
            
            assert isinstance(result, ExtendedUser)
            assert result.email == "john@example.com"
            mock_save.assert_called_once()
    
    def test_multiple_collections_for_same_model(self):
        """Test that multiple collection instances work independently."""
        collection1 = MongoCollection(TestUser)
        collection2 = MongoCollection(TestUser)
        
        assert collection1 is not collection2
        assert collection1.model_class == collection2.model_class
        
        # Both should work independently
        query1 = collection1.all()
        query2 = collection2.all()
        
        assert query1 is not query2
        assert query1.model_class == query2.model_class
    
    def test_find_mixed_expression_types(self):
        """Test find method with mixed query and compound expressions."""
        collection = MongoCollection(TestUser)
        
        query_expr = MagicMock(spec=QueryExpression)
        compound_expr = MagicMock(spec=CompoundExpression)
        
        query = collection.find(query_expr, compound_expr)
        
        assert isinstance(query, MongoQuery)
        assert len(query.expressions) == 2
        assert query_expr in query.expressions
        assert compound_expr in query.expressions