"""Tests for base backend modules."""

import pytest
from abc import ABC

from data_bridge.base.backends.async_ import AsyncBackend
from data_bridge.base.backends.sync import SyncBackend
from data_bridge.base.model import BaseModel
from data_bridge.base.query import BaseQuery


class MockModel(BaseModel):
    """Mock model for backend testing."""
    pass


class MockQuery(BaseQuery):
    """Mock query for backend testing."""
    
    def filter(self, *expressions):
        return self
    
    def limit(self, n: int):
        return self
    
    def skip(self, n: int):
        return self
    
    def sort(self, *fields):
        return self
    
    def select(self, *fields):
        return self


# Test concrete implementations for abstract backends
class MockSyncBackend(SyncBackend):
    """Mock synchronous backend for testing."""
    
    def save(self, instance: BaseModel) -> None:
        """Mock save implementation."""
        pass
    
    def delete(self, instance: BaseModel) -> None:
        """Mock delete implementation."""
        pass
    
    def execute_query(self, query: BaseQuery) -> list:
        """Mock execute_query implementation."""
        return []
    
    def count_query(self, query: BaseQuery) -> int:
        """Mock count_query implementation."""
        return 0
    
    def delete_query(self, query: BaseQuery) -> int:
        """Mock delete_query implementation."""
        return 0
    
    def update_query(self, query: BaseQuery, updates: dict) -> int:
        """Mock update_query implementation."""
        return 0


class MockAsyncBackend(AsyncBackend):
    """Mock asynchronous backend for testing."""
    
    async def save(self, instance: BaseModel) -> None:
        """Mock async save implementation."""
        pass
    
    async def delete(self, instance: BaseModel) -> None:
        """Mock async delete implementation."""
        pass
    
    async def execute_query(self, query: BaseQuery) -> list:
        """Mock async execute_query implementation."""
        return []
    
    async def count_query(self, query: BaseQuery) -> int:
        """Mock async count_query implementation."""
        return 0
    
    async def delete_query(self, query: BaseQuery) -> int:
        """Mock async delete_query implementation."""
        return 0
    
    async def update_query(self, query: BaseQuery, updates: dict) -> int:
        """Mock async update_query implementation."""
        return 0


class TestSyncBackend:
    """Test SyncBackend abstract class."""
    
    def test_sync_backend_is_abstract(self) -> None:
        """Test that SyncBackend is abstract and cannot be instantiated."""
        assert issubclass(SyncBackend, ABC)
        
        # Cannot instantiate abstract SyncBackend directly
        with pytest.raises(TypeError):
            SyncBackend()  # type: ignore
    
    def test_sync_backend_concrete_implementation(self) -> None:
        """Test that concrete SyncBackend implementation works."""
        backend = MockSyncBackend()
        
        # Should be able to instantiate concrete implementation
        assert isinstance(backend, SyncBackend)
    
    def test_sync_backend_method_signatures(self) -> None:
        """Test SyncBackend abstract method signatures."""
        backend = MockSyncBackend()
        instance = MockModel()
        query = MockQuery(MockModel, [])
        updates = {"field": "value"}
        
        # Test all abstract methods can be called
        backend.save(instance)
        backend.delete(instance)
        
        result = backend.execute_query(query)
        assert isinstance(result, list)
        
        count = backend.count_query(query)
        assert isinstance(count, int)
        
        deleted = backend.delete_query(query)
        assert isinstance(deleted, int)
        
        updated = backend.update_query(query, updates)
        assert isinstance(updated, int)


class TestAsyncBackend:
    """Test AsyncBackend abstract class."""
    
    def test_async_backend_is_abstract(self) -> None:
        """Test that AsyncBackend is abstract and cannot be instantiated."""
        assert issubclass(AsyncBackend, ABC)
        
        # Cannot instantiate abstract AsyncBackend directly
        with pytest.raises(TypeError):
            AsyncBackend()  # type: ignore
    
    def test_async_backend_concrete_implementation(self) -> None:
        """Test that concrete AsyncBackend implementation works."""
        backend = MockAsyncBackend()
        
        # Should be able to instantiate concrete implementation
        assert isinstance(backend, AsyncBackend)
    
    @pytest.mark.asyncio
    async def test_async_backend_method_signatures(self) -> None:
        """Test AsyncBackend abstract method signatures."""
        backend = MockAsyncBackend()
        instance = MockModel()
        query = MockQuery(MockModel, [])
        updates = {"field": "value"}
        
        # Test all abstract async methods can be called
        await backend.save(instance)
        await backend.delete(instance)
        
        result = await backend.execute_query(query)
        assert isinstance(result, list)
        
        count = await backend.count_query(query)
        assert isinstance(count, int)
        
        deleted = await backend.delete_query(query)
        assert isinstance(deleted, int)
        
        updated = await backend.update_query(query, updates)
        assert isinstance(updated, int)
    
    def test_async_methods_are_coroutines(self) -> None:
        """Test that async backend methods are coroutines."""
        backend = MockAsyncBackend()
        instance = MockModel()
        query = MockQuery(MockModel, [])
        updates = {"field": "value"}
        
        # Check that methods return coroutines (not executed yet)
        import asyncio
        
        save_coro = backend.save(instance)
        assert asyncio.iscoroutine(save_coro)
        save_coro.close()  # Clean up coroutine
        
        delete_coro = backend.delete(instance)
        assert asyncio.iscoroutine(delete_coro)
        delete_coro.close()
        
        execute_coro = backend.execute_query(query)
        assert asyncio.iscoroutine(execute_coro)
        execute_coro.close()
        
        count_coro = backend.count_query(query)
        assert asyncio.iscoroutine(count_coro)
        count_coro.close()
        
        delete_query_coro = backend.delete_query(query)
        assert asyncio.iscoroutine(delete_query_coro)
        delete_query_coro.close()
        
        update_coro = backend.update_query(query, updates)
        assert asyncio.iscoroutine(update_coro)
        update_coro.close()


class TestBackendIntegration:
    """Test backend integration with models and queries."""
    
    def test_sync_backend_with_model(self) -> None:
        """Test SyncBackend integration with model."""
        backend = MockSyncBackend()
        
        # Test that backend can work with model instances
        instance = MockModel()
        
        # These should not raise errors
        backend.save(instance)
        backend.delete(instance)
    
    def test_sync_backend_with_query(self) -> None:
        """Test SyncBackend integration with query."""
        backend = MockSyncBackend()
        query = MockQuery(MockModel, [])
        
        # Test that backend can work with queries
        results = backend.execute_query(query)
        assert results == []
        
        count = backend.count_query(query)
        assert count == 0
        
        deleted = backend.delete_query(query)
        assert deleted == 0
        
        updated = backend.update_query(query, {"field": "value"})
        assert updated == 0
    
    @pytest.mark.asyncio
    async def test_async_backend_with_model(self) -> None:
        """Test AsyncBackend integration with model."""
        backend = MockAsyncBackend()
        
        # Test that backend can work with model instances
        instance = MockModel()
        
        # These should not raise errors
        await backend.save(instance)
        await backend.delete(instance)
    
    @pytest.mark.asyncio 
    async def test_async_backend_with_query(self) -> None:
        """Test AsyncBackend integration with query."""
        backend = MockAsyncBackend()
        query = MockQuery(MockModel, [])
        
        # Test that backend can work with queries
        results = await backend.execute_query(query)
        assert results == []
        
        count = await backend.count_query(query)
        assert count == 0
        
        deleted = await backend.delete_query(query)
        assert deleted == 0
        
        updated = await backend.update_query(query, {"field": "value"})
        assert updated == 0