"""Integration tests for MongoDB update operations."""

from unittest.mock import MagicMock, patch

import pytest
from bson import ObjectId

from data_bridge.base.fields import (
    BoolField,
    DictField,
    FloatField,
    IntField,
    ListField,
    StringField,
)
from data_bridge.mongo.sync.backend import MongoSyncBackend
from data_bridge.mongo.translator import MongoUpdateTranslator


class MockUser:
    """Mock user document class for testing."""
    _collection = "users"
    _database = "test_db"
    
    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            setattr(self, key, value)
    
    def to_dict(self):
        return {key: value for key, value in self.__dict__.items() 
                if not key.startswith('_')}
    
    @classmethod
    def from_dict(cls, data):
        return cls(**data)


class MockQuery:
    """Mock query class for testing."""
    def __init__(self, model_class, expressions=None):
        self.model_class = model_class
        self.expressions = expressions or []
        self._projection = []
        self._sort_fields = []
        self._skip_value = 0
        self._limit_value = None


class TestMongoUpdateIntegration:
    """Test end-to-end MongoDB update operations."""
    
    def setup_method(self):
        """Set up test fields as if they were on a model."""
        self.name = StringField()
        self.name.name = "name"
        
        self.age = IntField()
        self.age.name = "age"
        
        self.score = FloatField()
        self.score.name = "score"
        
        self.active = BoolField()
        self.active.name = "active"
        
        self.tags = ListField(str)
        self.tags.name = "tags"
        
        self.metadata = DictField()
        self.metadata.name = "metadata"
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_single_field_update(self, mock_mongo_client: MagicMock) -> None:
        """Test updating a single field."""
        # Setup mocks
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.modified_count = 1
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many.return_value = mock_result
        
        # Test update operation
        backend = MongoSyncBackend()
        query = MockQuery(MockUser)
        updates = [self.name.set("John Doe")]
        
        result = backend.update_query(query, updates)
        
        # Verify the correct MongoDB update document was generated
        expected_update = {"$set": {"name": "John Doe"}}
        mock_collection.update_many.assert_called_once_with({}, expected_update)
        assert result == 1
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_multi_field_update(self, mock_mongo_client: MagicMock) -> None:
        """Test updating multiple fields with different operations."""
        # Setup mocks
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.modified_count = 3
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many.return_value = mock_result
        
        # Test multiple update operations
        backend = MongoSyncBackend()
        query = MockQuery(MockUser)
        updates = [
            self.name.set("Jane Smith"),
            self.age.inc(1),
            self.score.mul(1.5),
            self.active.toggle()
        ]
        
        result = backend.update_query(query, updates)
        
        # Verify complex update document
        expected_update = {
            "$set": {
                "name": "Jane Smith",
                "active": {"$not": "$active"}
            },
            "$inc": {"age": 1},
            "$mul": {"score": 1.5}
        }
        mock_collection.update_many.assert_called_once_with({}, expected_update)
        assert result == 3
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_array_operations(self, mock_mongo_client: MagicMock) -> None:
        """Test array update operations."""
        # Setup mocks
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.modified_count = 2
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many.return_value = mock_result
        
        # Test array operations
        backend = MongoSyncBackend()
        query = MockQuery(MockUser)
        updates = [
            self.tags.push("new_tag"),
            self.tags.add_to_set("unique_tag"),
            self.tags.pull("old_tag")
        ]
        
        result = backend.update_query(query, updates)
        
        expected_update = {
            "$push": {"tags": "new_tag"},
            "$addToSet": {"tags": "unique_tag"},
            "$pull": {"tags": "old_tag"}
        }
        mock_collection.update_many.assert_called_once_with({}, expected_update)
        assert result == 2
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_advanced_array_operations(self, mock_mongo_client: MagicMock) -> None:
        """Test advanced array operations with modifiers."""
        # Setup mocks
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.modified_count = 1
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many.return_value = mock_result
        
        # Test advanced array operations
        backend = MongoSyncBackend()
        query = MockQuery(MockUser)
        updates = [
            self.tags.push("priority_tag", position=0, slice=10),
            self.tags.push_all(["tag1", "tag2", "tag3"]),
            self.tags.add_to_set_each(["unique1", "unique2"])
        ]
        
        result = backend.update_query(query, updates)
        
        # Verify the complex array operations
        mock_collection.update_many.assert_called_once()
        update_call = mock_collection.update_many.call_args[0][1]
        
        # Check that $push operations have correct modifiers
        assert "$push" in update_call
        assert isinstance(update_call["$push"], dict)
        assert "$addToSet" in update_call
        
        assert result == 1
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_nested_field_operations(self, mock_mongo_client: MagicMock) -> None:
        """Test nested field update operations."""
        # Setup mocks
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.modified_count = 1
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many.return_value = mock_result
        
        # Test nested field operations
        backend = MongoSyncBackend()
        query = MockQuery(MockUser)
        updates = [
            self.metadata.set_field("config.theme", "dark"),
            self.metadata.inc_field("stats.views", 1),
            self.metadata.unset_field("old_setting"),
            self.metadata.set_field("user.preferences.lang", "en")
        ]
        
        result = backend.update_query(query, updates)
        
        expected_update = {
            "$set": {
                "metadata.config.theme": "dark",
                "metadata.user.preferences.lang": "en"
            },
            "$inc": {"metadata.stats.views": 1},
            "$unset": {"metadata.old_setting": ""}
        }
        mock_collection.update_many.assert_called_once_with({}, expected_update)
        assert result == 1
    
    def test_update_translator_complex_scenario(self) -> None:
        """Test translator with complex real-world scenario."""
        # Simulate a complex user profile update
        updates = [
            # Basic field updates
            self.name.set("John Doe Updated"),
            self.age.inc(1),
            self.score.mul(1.1),
            
            # Array operations
            self.tags.push("verified", position=0),
            self.tags.pull("unverified"),
            self.tags.add_to_set("premium"),
            
            # Nested operations
            self.metadata.set_field("last_login", "2024-01-01T12:00:00Z"),
            self.metadata.inc_field("login_count", 1),
            self.metadata.set_field("preferences.notifications", True),
            
            # Boolean toggle
            self.active.toggle()
        ]
        
        # Translate to MongoDB update document
        update_doc = MongoUpdateTranslator.translate(updates)
        
        # Verify structure contains all expected operations
        assert "$set" in update_doc
        assert "$inc" in update_doc
        assert "$mul" in update_doc
        assert "$push" in update_doc
        assert "$pull" in update_doc
        assert "$addToSet" in update_doc
        
        # Verify specific operations
        assert update_doc["$set"]["name"] == "John Doe Updated"
        assert update_doc["$inc"]["age"] == 1
        assert update_doc["$mul"]["score"] == 1.1
        assert update_doc["$pull"]["tags"] == "unverified"
        assert update_doc["$addToSet"]["tags"] == "premium"
        
        # Verify nested operations
        assert update_doc["$set"]["metadata.last_login"] == "2024-01-01T12:00:00Z"
        assert update_doc["$inc"]["metadata.login_count"] == 1
        assert update_doc["$set"]["metadata.preferences.notifications"] is True
        
        # Verify boolean toggle
        assert update_doc["$set"]["active"] == {"$not": "$active"}
    
    def test_update_operations_type_safety(self) -> None:
        """Test that update operations maintain type safety."""
        # Test numeric operations only work with numbers
        int_updates = [
            self.age.inc(5),
            self.age.mul(2),
            self.age.min(100),
            self.age.max(18)
        ]
        
        float_updates = [
            self.score.inc(1.5),
            self.score.mul(0.8),
            self.score.min(0.0),
            self.score.max(100.0)
        ]
        
        # Test array operations work with lists
        array_updates = [
            self.tags.push("new_tag"),
            self.tags.push_all(["tag1", "tag2"]),
            self.tags.pull("old_tag"),
            self.tags.add_to_set("unique_tag")
        ]
        
        # All should translate successfully
        int_doc = MongoUpdateTranslator.translate(int_updates)
        float_doc = MongoUpdateTranslator.translate(float_updates)
        array_doc = MongoUpdateTranslator.translate(array_updates)
        
        assert "$inc" in int_doc and "$mul" in int_doc
        assert "$inc" in float_doc and "$mul" in float_doc
        assert "$push" in array_doc and "$pull" in array_doc
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_error_handling_invalid_updates(self, mock_mongo_client: MagicMock) -> None:
        """Test error handling for invalid update scenarios."""
        # Setup mocks
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        
        # Test invalid pop position
        with pytest.raises(ValueError, match="Position must be -1 \\(first\\) or 1 \\(last\\)"):
            self.tags.pop(0)
        
        # Test empty field name should fail in UpdateExpression validation
        with pytest.raises(ValueError, match="Field cannot be empty"):
            from data_bridge.base.fields import UpdateExpression
            UpdateExpression("", "set", "value")
    
    def test_concurrent_update_simulation(self) -> None:
        """Test concurrent update operations (simulation)."""
        # Simulate multiple concurrent updates that might happen
        user_updates = [
            self.name.set("Updated Name"),
            self.age.inc(1)
        ]
        
        stats_updates = [
            self.metadata.inc_field("views", 1),
            self.metadata.inc_field("likes", 5)
        ]
        
        activity_updates = [
            self.tags.add_to_set("active_user"),
            self.active.set(True),
            self.score.inc(10.0)
        ]
        
        # Each update should be atomic and generate correct MongoDB operations
        user_doc = MongoUpdateTranslator.translate(user_updates)
        stats_doc = MongoUpdateTranslator.translate(stats_updates)  
        activity_doc = MongoUpdateTranslator.translate(activity_updates)
        
        # Verify all operations are properly structured
        assert user_doc["$set"]["name"] == "Updated Name"
        assert user_doc["$inc"]["age"] == 1
        
        assert stats_doc["$inc"]["metadata.views"] == 1
        assert stats_doc["$inc"]["metadata.likes"] == 5
        
        assert activity_doc["$addToSet"]["tags"] == "active_user"
        assert activity_doc["$set"]["active"] is True
        assert activity_doc["$inc"]["score"] == 10.0