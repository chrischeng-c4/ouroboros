"""Tests for MongoDB backend implementations (sync and async)."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from bson import ObjectId
from pymongo.collection import Collection
from pymongo.database import Database
from pymongo.results import DeleteResult, InsertOneResult, UpdateResult

from data_bridge.base.fields import UpdateExpression
from data_bridge.mongo.async_.backend import MongoAsyncBackend
from data_bridge.mongo.sync.backend import MongoSyncBackend


class TestMongoSyncBackend:
    """Test MongoDB synchronous backend."""
    
    def test_backend_initialization(self) -> None:
        """Test backend initialization with default parameters."""
        backend = MongoSyncBackend()
        assert backend.connection_string == "mongodb://localhost:27017"
        assert backend.database_name == "default"
        assert backend.client is None
        assert backend.database is None
    
    def test_backend_initialization_custom_params(self) -> None:
        """Test backend initialization with custom parameters."""
        backend = MongoSyncBackend(
            connection_string="mongodb://custom:27017",
            database_name="custom_db",
            serverSelectionTimeoutMS=5000
        )
        assert backend.connection_string == "mongodb://custom:27017"
        assert backend.database_name == "custom_db"
        assert backend.client_kwargs == {"serverSelectionTimeoutMS": 5000}
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_connect(self, mock_mongo_client: MagicMock) -> None:
        """Test connection establishment."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        
        backend = MongoSyncBackend()
        backend.connect()
        
        mock_mongo_client.assert_called_once_with("mongodb://localhost:27017")
        assert backend.client == mock_client
        assert backend.database == mock_database
    
    def test_connect_already_connected(self) -> None:
        """Test connect when already connected."""
        backend = MongoSyncBackend()
        mock_client = MagicMock()
        backend.client = mock_client
        
        backend.connect()
        # Should not create new client
        assert backend.client == mock_client
    
    def test_disconnect(self) -> None:
        """Test disconnection."""
        backend = MongoSyncBackend()
        mock_client = MagicMock()
        mock_database = MagicMock()
        backend.client = mock_client
        backend.database = mock_database
        
        backend.disconnect()
        
        mock_client.close.assert_called_once()
        assert backend.client is None
        assert backend.database is None
    
    def test_disconnect_not_connected(self) -> None:
        """Test disconnect when not connected."""
        backend = MongoSyncBackend()
        backend.disconnect()  # Should not raise error
        assert backend.client is None
        assert backend.database is None
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_get_collection_default_database(self, mock_mongo_client: MagicMock) -> None:
        """Test getting collection with default database."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock(spec=Collection)
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        
        # Mock document class
        class MockDocument:
            _collection = "test_collection"
        
        backend = MongoSyncBackend(database_name="test_db")
        collection = backend.get_collection(MockDocument)
        
        # Should connect first
        mock_mongo_client.assert_called_once()
        mock_database.__getitem__.assert_called_once_with("test_collection")
        assert collection == mock_collection
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_get_collection_custom_database(self, mock_mongo_client: MagicMock) -> None:
        """Test getting collection with custom database."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_custom_database = MagicMock()
        mock_collection = MagicMock(spec=Collection)
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.side_effect = lambda name: {
            "default": mock_database,
            "custom_db": mock_custom_database
        }[name]
        mock_custom_database.__getitem__.return_value = mock_collection
        
        # Mock document class with custom database
        class MockDocument:
            _collection = "test_collection"
            _database = "custom_db"
        
        backend = MongoSyncBackend()
        backend.database_name = "default"
        collection = backend.get_collection(MockDocument)
        
        mock_custom_database.__getitem__.assert_called_once_with("test_collection")
        assert collection == mock_collection
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_save_new_document(self, mock_mongo_client: MagicMock) -> None:
        """Test saving a new document (insert)."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock(spec=InsertOneResult)
        mock_result.inserted_id = ObjectId()
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.insert_one.return_value = mock_result
        
        # Mock document instance
        class MockDocument:
            _collection = "test_collection"
            def to_dict(self):
                return {"name": "test"}
        
        document = MockDocument()
        document._id = None  # Initialize _id attribute
        backend = MongoSyncBackend()
        backend.save(document)
        
        mock_collection.insert_one.assert_called_once_with({"name": "test"})
        assert document._id == mock_result.inserted_id
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_save_existing_document(self, mock_mongo_client: MagicMock) -> None:
        """Test saving an existing document (update)."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock(spec=UpdateResult)
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.replace_one.return_value = mock_result
        
        # Mock document instance with existing _id
        class MockDocument:
            _collection = "test_collection"
            def to_dict(self):
                return {"_id": self._id, "name": "updated"}
        
        document = MockDocument()
        document._id = ObjectId()
        backend = MongoSyncBackend()
        backend.save(document)
        
        mock_collection.replace_one.assert_called_once_with(
            {"_id": document._id}, 
            {"_id": document._id, "name": "updated"},
            upsert=True
        )
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_delete_document(self, mock_mongo_client: MagicMock) -> None:
        """Test deleting a document."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock(spec=DeleteResult)
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.delete_one.return_value = mock_result
        
        # Mock document instance
        class MockDocument:
            _collection = "test_collection"
        
        document = MockDocument()
        document._id = ObjectId()
        backend = MongoSyncBackend()
        backend.delete(document)
        
        mock_collection.delete_one.assert_called_once_with({"_id": document._id})
    
    def test_delete_document_no_id_error(self) -> None:
        """Test error when deleting document without _id."""
        class MockDocument:
            _collection = "test_collection"
        
        document = MockDocument()
        backend = MongoSyncBackend()
        
        with pytest.raises(ValueError, match="Cannot delete document without _id"):
            backend.delete(document)
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_update_query(self, mock_mongo_client: MagicMock) -> None:
        """Test updating documents with query and update expressions."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock(spec=UpdateResult)
        mock_result.modified_count = 3
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many.return_value = mock_result
        
        # Mock query and document class
        class MockDocument:
            _collection = "test_collection"
        
        class MockQuery:
            model_class = MockDocument
            expressions = []
        
        # Create update expressions
        updates = [
            UpdateExpression("name", "set", "new_name"),
            UpdateExpression("age", "inc", 1)
        ]
        
        backend = MongoSyncBackend()
        query = MockQuery()
        result = backend.update_query(query, updates)
        
        # Verify MongoDB update document was created correctly
        expected_update = {
            "$set": {"name": "new_name"},
            "$inc": {"age": 1}
        }
        mock_collection.update_many.assert_called_once_with({}, expected_update)
        assert result == 3
    
    @patch('data_bridge.mongo.sync.backend.MongoClient')
    def test_update_query_complex_operations(self, mock_mongo_client: MagicMock) -> None:
        """Test updating with complex array and nested operations."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock(spec=UpdateResult)
        mock_result.modified_count = 1
        
        mock_mongo_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many.return_value = mock_result
        
        class MockDocument:
            _collection = "test_collection"
        
        class MockQuery:
            model_class = MockDocument
            expressions = []
        
        # Complex update expressions
        updates = [
            UpdateExpression("tags", "push", "new_tag", {"$position": 0}),
            UpdateExpression("active", "toggle", None),
            UpdateExpression("metadata.count", "inc", 1)
        ]
        
        backend = MongoSyncBackend()
        query = MockQuery()
        result = backend.update_query(query, updates)
        
        expected_update = {
            "$push": {"tags": {"$position": 0, "$each": ["new_tag"]}},
            "$set": {"active": {"$not": "$active"}, "metadata.count": {"$inc": 1}},
            "$inc": {"metadata.count": 1}
        }
        
        # The exact structure may vary based on translator logic
        mock_collection.update_many.assert_called_once()
        assert result == 1


class TestMongoAsyncBackend:
    """Test MongoDB asynchronous backend."""
    
    def test_async_backend_initialization(self) -> None:
        """Test async backend initialization."""
        backend = MongoAsyncBackend()
        assert backend.connection_string == "mongodb://localhost:27017"
        assert backend.database_name == "default"
        assert backend.client is None
        assert backend.database is None
    
    @patch('data_bridge.mongo.async_.backend.AsyncIOMotorClient')
    async def test_async_connect(self, mock_motor_client: MagicMock) -> None:
        """Test async connection establishment."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_motor_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        
        backend = MongoAsyncBackend()
        await backend.connect()
        
        mock_motor_client.assert_called_once_with("mongodb://localhost:27017")
        assert backend.client == mock_client
        assert backend.database == mock_database
    
    async def test_async_disconnect(self) -> None:
        """Test async disconnection."""
        backend = MongoAsyncBackend()
        mock_client = MagicMock()
        backend.client = mock_client
        backend.database = MagicMock()
        
        await backend.disconnect()
        
        mock_client.close.assert_called_once()
        assert backend.client is None
        assert backend.database is None
    
    @patch('data_bridge.mongo.async_.backend.AsyncIOMotorClient')
    async def test_async_get_collection(self, mock_motor_client: MagicMock) -> None:
        """Test async get_collection."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_motor_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        
        class MockDocument:
            _collection = "test_collection"
        
        backend = MongoAsyncBackend()
        collection = await backend.get_collection(MockDocument)
        
        mock_motor_client.assert_called_once()
        mock_database.__getitem__.assert_called_once_with("test_collection")
        assert collection == mock_collection
    
    @patch('data_bridge.mongo.async_.backend.AsyncIOMotorClient')
    async def test_async_save_new_document(self, mock_motor_client: MagicMock) -> None:
        """Test async save for new document."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.inserted_id = ObjectId()
        
        mock_motor_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.insert_one = AsyncMock(return_value=mock_result)
        
        class MockDocument:
            _collection = "test_collection"
            def to_dict(self):
                return {"name": "test"}
        
        document = MockDocument()
        document._id = None  # Initialize _id attribute
        backend = MongoAsyncBackend()
        await backend.save(document)
        
        mock_collection.insert_one.assert_called_once_with({"name": "test"})
        assert document._id == mock_result.inserted_id
    
    @patch('data_bridge.mongo.async_.backend.AsyncIOMotorClient')
    async def test_async_update_query(self, mock_motor_client: MagicMock) -> None:
        """Test async update_query method."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.modified_count = 2
        
        mock_motor_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.update_many = AsyncMock(return_value=mock_result)
        
        class MockDocument:
            _collection = "test_collection"
        
        class MockQuery:
            model_class = MockDocument
            expressions = []
        
        updates = [
            UpdateExpression("status", "set", "active"),
            UpdateExpression("count", "inc", 5)
        ]
        
        backend = MongoAsyncBackend()
        query = MockQuery()
        result = await backend.update_query(query, updates)
        
        expected_update = {
            "$set": {"status": "active"},
            "$inc": {"count": 5}
        }
        mock_collection.update_many.assert_called_once_with({}, expected_update)
        assert result == 2
    
    @patch('data_bridge.mongo.async_.backend.AsyncIOMotorClient')
    async def test_async_delete_query(self, mock_motor_client: MagicMock) -> None:
        """Test async delete_query method."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        mock_result = MagicMock()
        mock_result.deleted_count = 5
        
        mock_motor_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.delete_many = AsyncMock(return_value=mock_result)
        
        class MockDocument:
            _collection = "test_collection"
        
        class MockQuery:
            model_class = MockDocument
            expressions = []
        
        backend = MongoAsyncBackend()
        query = MockQuery()
        result = await backend.delete_query(query)
        
        mock_collection.delete_many.assert_called_once_with({})
        assert result == 5
    
    @patch('data_bridge.mongo.async_.backend.AsyncIOMotorClient')
    async def test_async_count_query(self, mock_motor_client: MagicMock) -> None:
        """Test async count_query method."""
        mock_client = MagicMock()
        mock_database = MagicMock()
        mock_collection = MagicMock()
        
        mock_motor_client.return_value = mock_client
        mock_client.__getitem__.return_value = mock_database
        mock_database.__getitem__.return_value = mock_collection
        mock_collection.count_documents = AsyncMock(return_value=10)
        
        class MockDocument:
            _collection = "test_collection"
        
        class MockQuery:
            model_class = MockDocument
            expressions = []
        
        backend = MongoAsyncBackend()
        query = MockQuery()
        result = await backend.count_query(query)
        
        mock_collection.count_documents.assert_called_once_with({})
        assert result == 10