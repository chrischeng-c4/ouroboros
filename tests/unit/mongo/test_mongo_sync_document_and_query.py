"""Tests for MongoDB synchronous document and query functionality."""

from unittest.mock import MagicMock
import pytest

from data_bridge.base.fields import StringField, IntField, QueryExpression, CompoundExpression
from data_bridge.mongo.sync.document import Document
from data_bridge.mongo.sync.query import MongoQuery
from data_bridge.mongo.sync.backend import MongoSyncBackend
from data_bridge.mongo.sync.collection import MongoCollection


class TestUser(Document):
    """Test user document for unit tests."""
    _collection = "users"
    _database = "test_db"
    
    id = StringField(primary_key=True, required=False)
    name = StringField(required=False)
    age = IntField(required=False)
    email = StringField(required=False)


class TestMongoDocument:
    """Test MongoDB Document functionality."""
    
    def setup_method(self):
        """Set up test environment."""
        # Reset backend for each test
        TestUser._backend = None
    
    def test_objects_returns_collection_manager(self):
        """Test that objects() returns a MongoCollection instance."""
        collection = TestUser.objects()
        assert isinstance(collection, MongoCollection)
        assert collection.model_class == TestUser
    
    def test_set_backend(self):
        """Test setting backend."""
        backend = MagicMock(spec=MongoSyncBackend)
        TestUser.set_backend(backend)
        assert TestUser._backend == backend
    
    def test_save_with_backend(self):
        """Test saving with backend configured."""
        backend = MagicMock(spec=MongoSyncBackend)
        TestUser.set_backend(backend)
        
        user = TestUser(id="123", name="John", age=30)
        user.save()
        
        backend.save.assert_called_once_with(user)
    
    def test_save_no_backend_configured(self):
        """Test saving when no backend is configured."""
        user = TestUser(id="123", name="John")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            user.save()
    
    def test_delete_with_backend(self):
        """Test deleting with backend configured."""
        backend = MagicMock(spec=MongoSyncBackend)
        TestUser.set_backend(backend)
        
        user = TestUser(id="123", name="John")
        user.delete()
        
        backend.delete.assert_called_once_with(user)
    
    def test_delete_no_backend_configured(self):
        """Test deleting when no backend is configured."""
        user = TestUser(id="123", name="John")
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            user.delete()
    
    def test_document_inheritance_from_base_model(self):
        """Test that Document properly inherits from BaseModel."""
        user = TestUser(id="123", name="John", age=30)
        
        # Test to_dict functionality
        data = user.to_dict()
        assert data["id"] == "123"
        assert data["name"] == "John"
        assert data["age"] == 30
        
        # Test from_dict functionality
        new_user = TestUser.from_dict(data)
        assert new_user.id == "123"
        assert new_user.name == "John"
        assert new_user.age == 30


class TestMongoQuery:
    """Test MongoQuery functionality."""
    
    def setup_method(self):
        """Set up test environment."""
        # Reset backend for each test
        TestUser._backend = None
    
    def test_query_initialization(self):
        """Test query initialization."""
        expressions = [MagicMock(spec=QueryExpression)]
        query = MongoQuery(TestUser, expressions)
        
        assert query.model_class == TestUser
        assert query.expressions == expressions
        assert query._backend is None  # No backend set on TestUser
    
    def test_query_initialization_with_backend(self):
        """Test query initialization with backend on model."""
        backend = MagicMock(spec=MongoSyncBackend)
        TestUser.set_backend(backend)
        
        expressions = [MagicMock(spec=QueryExpression)]
        query = MongoQuery(TestUser, expressions)
        
        assert query._backend == backend
    
    def test_filter(self):
        """Test adding filter expressions."""
        initial_expr = MagicMock(spec=QueryExpression)
        query = MongoQuery(TestUser, [initial_expr])
        
        new_expr = MagicMock(spec=QueryExpression)
        filtered_query = query.filter(new_expr)
        
        # Should return a new query instance
        assert filtered_query is not query
        assert len(filtered_query.expressions) == 2
        assert initial_expr in filtered_query.expressions
        assert new_expr in filtered_query.expressions
    
    def test_limit(self):
        """Test setting query limit."""
        query = MongoQuery(TestUser, [])
        limited_query = query.limit(10)
        
        # Should return a new query instance
        assert limited_query is not query
        assert limited_query._limit_value == 10
        assert query._limit_value is None  # Original unchanged
    
    def test_skip(self):
        """Test setting query skip."""
        query = MongoQuery(TestUser, [])
        skipped_query = query.skip(5)
        
        # Should return a new query instance
        assert skipped_query is not query
        assert skipped_query._skip_value == 5
        assert query._skip_value == 0  # Original unchanged
    
    def test_sort_single_field(self):
        """Test sorting by single field."""
        query = MongoQuery(TestUser, [])
        sorted_query = query.sort("name")
        
        # Should return a new query instance
        assert sorted_query is not query
        assert len(sorted_query._sort_fields) == 1
        assert ("name", 1) in sorted_query._sort_fields
    
    def test_sort_field_with_direction(self):
        """Test sorting with explicit direction."""
        query = MongoQuery(TestUser, [])
        sorted_query = query.sort("-age")
        
        assert len(sorted_query._sort_fields) == 1
        assert ("age", -1) in sorted_query._sort_fields
    
    def test_sort_multiple_fields(self):
        """Test sorting by multiple fields."""
        query = MongoQuery(TestUser, [])
        sorted_query = query.sort(("name", 1), ("age", -1))
        
        assert len(sorted_query._sort_fields) == 2
        assert ("name", 1) in sorted_query._sort_fields
        assert ("age", -1) in sorted_query._sort_fields
    
    def test_select_projection(self):
        """Test selecting specific fields."""
        query = MongoQuery(TestUser, [])
        selected_query = query.select("name", "email")
        
        # Should return a new query instance
        assert selected_query is not query
        assert selected_query._projection == ["name", "email"]
        assert query._projection is None  # Original unchanged
    
    def test_execute_with_backend(self):
        """Test executing query with backend."""
        backend = MagicMock(spec=MongoSyncBackend)
        test_results = [TestUser(id="1", name="John"), TestUser(id="2", name="Jane")]
        backend.execute_query.return_value = test_results
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        results = query.execute()
        
        backend.execute_query.assert_called_once_with(query)
        assert results == test_results
    
    def test_execute_no_backend_configured(self):
        """Test executing query when no backend is configured."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            query.execute()
    
    def test_first_with_results(self):
        """Test getting first result when results exist."""
        backend = MagicMock(spec=MongoSyncBackend)
        test_user = TestUser(id="1", name="John")
        backend.execute_query.return_value = [test_user]
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        result = query.first()
        
        assert result == test_user
        # Should have called limit(1)
        backend.execute_query.assert_called_once()
        called_query = backend.execute_query.call_args[0][0]
        assert called_query._limit_value == 1
    
    def test_first_no_results(self):
        """Test getting first result when no results exist."""
        backend = MagicMock(spec=MongoSyncBackend)
        backend.execute_query.return_value = []
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        result = query.first()
        
        assert result is None
    
    def test_count_with_backend(self):
        """Test counting query results with backend."""
        backend = MagicMock(spec=MongoSyncBackend)
        backend.count_query.return_value = 5
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        result = query.count()
        
        backend.count_query.assert_called_once_with(query)
        assert result == 5
    
    def test_count_no_backend_configured(self):
        """Test counting when no backend is configured."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            query.count()
    
    def test_exists_with_results(self):
        """Test exists when documents exist."""
        backend = MagicMock(spec=MongoSyncBackend)
        backend.count_query.return_value = 3
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        result = query.exists()
        
        assert result is True
        # Should have called count with limit(1)
        backend.count_query.assert_called_once()
        called_query = backend.count_query.call_args[0][0]
        assert called_query._limit_value == 1
    
    def test_exists_no_results(self):
        """Test exists when no documents exist."""
        backend = MagicMock(spec=MongoSyncBackend)
        backend.count_query.return_value = 0
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        result = query.exists()
        
        assert result is False
    
    def test_delete_with_backend(self):
        """Test deleting matching documents with backend."""
        backend = MagicMock(spec=MongoSyncBackend)
        backend.delete_query.return_value = 3
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        result = query.delete()
        
        backend.delete_query.assert_called_once_with(query)
        assert result == 3
    
    def test_delete_no_backend_configured(self):
        """Test deleting when no backend is configured."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            query.delete()
    
    def test_update_with_backend(self):
        """Test updating matching documents with backend."""
        backend = MagicMock(spec=MongoSyncBackend)
        backend.update_query.return_value = 2
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        updates = {"name": "Updated Name", "age": 25}
        result = query.update(**updates)
        
        backend.update_query.assert_called_once_with(query, updates)
        assert result == 2
    
    def test_update_no_backend_configured(self):
        """Test updating when no backend is configured."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(RuntimeError, match="No backend configured for TestUser"):
            query.update(name="Updated")
    
    def test_iteration(self):
        """Test iterating over query results."""
        backend = MagicMock(spec=MongoSyncBackend)
        test_users = [TestUser(id="1", name="John"), TestUser(id="2", name="Jane")]
        backend.execute_query.return_value = test_users
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        results = list(query)
        
        assert results == test_users
        backend.execute_query.assert_called_once_with(query)
    
    def test_indexing_positive_index(self):
        """Test accessing query result by positive index."""
        backend = MagicMock(spec=MongoSyncBackend)
        test_user = TestUser(id="2", name="Jane")
        backend.execute_query.return_value = [test_user]
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        result = query[1]
        
        assert result == test_user
        # Should have called skip(1).limit(1)
        backend.execute_query.assert_called_once()
        called_query = backend.execute_query.call_args[0][0]
        assert called_query._skip_value == 1
        assert called_query._limit_value == 1
    
    def test_indexing_negative_index(self):
        """Test accessing query result by negative index (should raise error)."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(ValueError, match="Negative indexing not supported"):
            _ = query[-1]
    
    def test_indexing_out_of_range(self):
        """Test accessing query result with index out of range."""
        backend = MagicMock(spec=MongoSyncBackend)
        backend.execute_query.return_value = []  # No results
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(IndexError, match="Query index out of range"):
            _ = query[0]
    
    def test_slicing_basic(self):
        """Test slicing query results."""
        backend = MagicMock(spec=MongoSyncBackend)
        test_users = [TestUser(id="2", name="Jane"), TestUser(id="3", name="Bob")]
        backend.execute_query.return_value = test_users
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        results = query[1:3]
        
        assert results == test_users
        # Should have called skip(1).limit(2)
        backend.execute_query.assert_called_once()
        called_query = backend.execute_query.call_args[0][0]
        assert called_query._skip_value == 1
        assert called_query._limit_value == 2
    
    def test_slicing_start_only(self):
        """Test slicing with only start index."""
        backend = MagicMock(spec=MongoSyncBackend)
        test_users = [TestUser(id="3", name="Bob")]
        backend.execute_query.return_value = test_users
        TestUser.set_backend(backend)
        
        query = MongoQuery(TestUser, [])
        results = query[2:]
        
        assert results == test_users
        # Should have called skip(2) with no limit
        backend.execute_query.assert_called_once()
        called_query = backend.execute_query.call_args[0][0]
        assert called_query._skip_value == 2
        assert called_query._limit_value is None
    
    def test_slicing_with_step(self):
        """Test slicing with step (should raise error)."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(ValueError, match="Step not supported in query slicing"):
            _ = query[1:10:2]
    
    def test_slicing_negative_indices(self):
        """Test slicing with negative indices (should raise error)."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(ValueError, match="Negative indexing not supported"):
            _ = query[-1:5]
        
        with pytest.raises(ValueError, match="Negative indexing not supported"):
            _ = query[1:-1]
    
    def test_invalid_index_type(self):
        """Test accessing with invalid index type."""
        query = MongoQuery(TestUser, [])
        
        with pytest.raises(TypeError, match="Invalid index type"):
            _ = query["invalid"]
    
    def test_clone(self):
        """Test cloning query preserves all attributes."""
        backend = MagicMock(spec=MongoSyncBackend)
        TestUser.set_backend(backend)
        
        expressions = [MagicMock(spec=QueryExpression)]
        query = MongoQuery(TestUser, expressions)
        query._limit_value = 10
        query._skip_value = 5
        query._sort_fields = [("name", 1), ("age", -1)]
        query._projection = ["name", "email"]
        
        cloned = query._clone()
        
        # Should be different objects
        assert cloned is not query
        assert cloned.expressions is not query.expressions
        
        # But have same content
        assert cloned.model_class == query.model_class
        assert cloned.expressions == query.expressions
        assert cloned._limit_value == query._limit_value
        assert cloned._skip_value == query._skip_value
        assert cloned._sort_fields == query._sort_fields
        assert cloned._projection == query._projection
        assert cloned._backend == query._backend
    
    def test_clone_with_none_projection(self):
        """Test cloning query with None projection."""
        query = MongoQuery(TestUser, [])
        query._projection = None
        
        cloned = query._clone()
        
        assert cloned._projection is None


class TestMongoQueryChaining:
    """Test MongoDB query method chaining."""
    
    def setup_method(self):
        """Set up test environment."""
        TestUser._backend = None
    
    def test_chaining_multiple_operations(self):
        """Test chaining multiple query operations."""
        backend = MagicMock(spec=MongoSyncBackend)
        test_results = [TestUser(id="1", name="John")]
        backend.execute_query.return_value = test_results
        TestUser.set_backend(backend)
        
        expr = MagicMock(spec=QueryExpression)
        query = MongoQuery(TestUser, [])
        
        result = query.filter(expr).limit(10).skip(5).sort("name").select("name", "age").execute()
        
        assert result == test_results
        backend.execute_query.assert_called_once()
        called_query = backend.execute_query.call_args[0][0]
        
        # Verify all operations were applied
        assert expr in called_query.expressions
        assert called_query._limit_value == 10
        assert called_query._skip_value == 5
        assert ("name", 1) in called_query._sort_fields
        assert called_query._projection == ["name", "age"]
    
    def test_immutability_of_chained_operations(self):
        """Test that chained operations don't modify the original query."""
        query = MongoQuery(TestUser, [])
        
        # Chain operations but don't assign to variables
        query.limit(10).skip(5).sort("name").select("name")
        
        # Original query should be unchanged
        assert query._limit_value is None
        assert query._skip_value == 0
        assert len(query._sort_fields) == 0
        assert query._projection is None


class TestMongoQueryEdgeCases:
    """Test edge cases and error conditions for MongoDB queries."""
    
    def test_query_with_compound_expressions(self):
        """Test query with compound expressions."""
        compound_expr = MagicMock(spec=CompoundExpression)
        query = MongoQuery(TestUser, [compound_expr])
        
        assert compound_expr in query.expressions
        
        # Test filtering with compound expressions
        new_compound_expr = MagicMock(spec=CompoundExpression)
        filtered_query = query.filter(new_compound_expr)
        
        assert len(filtered_query.expressions) == 2
        assert compound_expr in filtered_query.expressions
        assert new_compound_expr in filtered_query.expressions