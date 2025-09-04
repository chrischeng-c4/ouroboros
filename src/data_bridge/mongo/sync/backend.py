from __future__ import annotations

from typing import TYPE_CHECKING, Any, TypeVar

from pymongo import MongoClient
from pymongo.collection import Collection
from pymongo.database import Database

from ...base.backends.sync import SyncBackend
from ..translator import MongoQueryTranslator

if TYPE_CHECKING:
    from .document import Document
    from .query import MongoQuery

T = TypeVar("T", bound="Document")


class MongoSyncBackend(SyncBackend):
    """Synchronous MongoDB backend implementation."""

    def __init__(
        self,
        connection_string: str = "mongodb://localhost:27017",
        database_name: str = "default",
        **client_kwargs: Any
    ) -> None:
        """Initialize the MongoDB backend.

        Args:
            connection_string: MongoDB connection string
            database_name: Default database name
            **client_kwargs: Additional arguments passed to MongoClient
        """
        self.connection_string = connection_string
        self.database_name = database_name
        self.client: MongoClient | None = None
        self.database: Database | None = None
        self.client_kwargs = client_kwargs

    def connect(self) -> None:
        """Establish connection to MongoDB."""
        if self.client is None:
            self.client = MongoClient(self.connection_string, **self.client_kwargs)
            self.database = self.client[self.database_name]

    def disconnect(self) -> None:
        """Close connection to MongoDB."""
        if self.client:
            self.client.close()
            self.client = None
            self.database = None

    def get_collection(self, document_class: type[Document]) -> Collection:
        """Get MongoDB collection for a document class."""
        if self.database is None:
            self.connect()

        database_name = getattr(document_class, '_database', None) or self.database_name
        collection_name = document_class._collection

        # Switch database if needed
        if database_name != self.database_name and self.client:
            db = self.client[database_name]
        else:
            db = self.database

        return db[collection_name]  # type: ignore[index]

    def save(self, instance: Document) -> None:
        """Save a document instance."""
        collection = self.get_collection(instance.__class__)
        data = instance.to_dict()

        # Check if this is an update (has _id) or insert
        if hasattr(instance, '_id') and instance._id is not None:
            # Update existing document
            filter_query = {"_id": instance._id}
            collection.replace_one(filter_query, data, upsert=True)
        else:
            # Insert new document
            result = collection.insert_one(data)
            # Set the _id on the instance if it has an _id field
            if hasattr(instance, '_id'):
                instance._id = result.inserted_id

    def delete(self, instance: Document) -> None:
        """Delete a document instance."""
        if not hasattr(instance, '_id') or instance._id is None:
            raise ValueError("Cannot delete document without _id")

        collection = self.get_collection(instance.__class__)
        filter_query = {"_id": instance._id}
        collection.delete_one(filter_query)

    def execute_query(self, query: MongoQuery[T]) -> list[T]:
        """Execute a query and return results."""
        collection = self.get_collection(query.model_class)

        # Translate expressions to MongoDB query
        filter_query = MongoQueryTranslator.translate(query.expressions)

        # Build the find operation
        cursor = collection.find(filter_query)

        # Apply projection
        if query._projection:
            projection = MongoQueryTranslator.translate_projection(query._projection)
            cursor = collection.find(filter_query, projection)

        # Apply sorting
        if query._sort_fields:
            sort_spec = MongoQueryTranslator.translate_sort(query._sort_fields)
            cursor = cursor.sort(sort_spec)

        # Apply skip and limit
        if query._skip_value > 0:
            cursor = cursor.skip(query._skip_value)

        if query._limit_value is not None:
            cursor = cursor.limit(query._limit_value)

        # Convert results to model instances
        results = []
        for doc in cursor:
            instance = query.model_class.from_dict(doc)
            results.append(instance)

        return results

    def count_query(self, query: MongoQuery[T]) -> int:
        """Count documents matching a query."""
        collection = self.get_collection(query.model_class)
        filter_query = MongoQueryTranslator.translate(query.expressions)
        return collection.count_documents(filter_query)

    def delete_query(self, query: MongoQuery[T]) -> int:
        """Delete documents matching a query."""
        collection = self.get_collection(query.model_class)
        filter_query = MongoQueryTranslator.translate(query.expressions)
        result = collection.delete_many(filter_query)
        return result.deleted_count

    def update_query(self, query: MongoQuery[T], updates: dict[str, Any]) -> int:
        """Update documents matching a query."""
        collection = self.get_collection(query.model_class)
        filter_query = MongoQueryTranslator.translate(query.expressions)

        # Convert updates to MongoDB update operations
        update_doc = {"$set": updates}

        result = collection.update_many(filter_query, update_doc)
        return result.modified_count
