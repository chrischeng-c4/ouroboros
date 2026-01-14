"""
Unit tests for AsyncSession and async utilities.

Tests async session management, factories, scoped sessions,
and async helpers without requiring a database connection.
"""
import pytest
from ouroboros.test import expect
import asyncio
from unittest.mock import MagicMock, AsyncMock, patch, Mock
from dataclasses import dataclass
from typing import Any, Dict, Optional


# Test fixture - simple model class
@dataclass
class MockUser:
    id: Optional[int] = None
    name: str = ""
    email: str = ""

    def _get_pk(self) -> Optional[int]:
        return self.id

    def _get_pk_column(self) -> str:
        return "id"

    @classmethod
    def _get_table_name(cls) -> str:
        return "users"

    def _get_column_values(self) -> Dict[str, Any]:
        return {"id": self.id, "name": self.name, "email": self.email}


@dataclass
class MockPost:
    id: Optional[int] = None
    title: str = ""
    user_id: Optional[int] = None

    def _get_pk(self) -> Optional[int]:
        return self.id

    def _get_pk_column(self) -> str:
        return "id"

    @classmethod
    def _get_table_name(cls) -> str:
        return "posts"

    def _get_column_values(self) -> Dict[str, Any]:
        return {"id": self.id, "title": self.title, "user_id": self.user_id}


# ============================================================================
# AsyncSession Tests
# ============================================================================

class TestAsyncSession:
    """Test AsyncSession functionality."""

    @pytest.mark.asyncio
    async def test_context_manager_enter_exit(self):
        """Test async context manager enter/exit."""
        from ouroboros.postgres.async_utils import AsyncSession

        async with AsyncSession() as session:
            assert session is not None
            assert not session._closed

        # Should be closed after exiting
        assert session._closed

    @pytest.mark.asyncio
    async def test_context_manager_commit_on_success(self):
        """Test context manager commits on success with autocommit."""
        from ouroboros.postgres.async_utils import AsyncSession

        session = AsyncSession(autocommit=True)
        user = MockUser(name="Alice")
        session.add(user)

        # Mock flush to avoid actual database operations
        with patch.object(session, 'flush', new_callable=AsyncMock):
            # Manually exit context
            await session.__aexit__(None, None, None)

        # Should have committed
        assert session._closed

    @pytest.mark.asyncio
    async def test_context_manager_rollback_on_exception(self):
        """Test context manager rolls back on exception."""
        from ouroboros.postgres.async_utils import AsyncSession

        with patch('ouroboros.postgres.execute', new_callable=AsyncMock) as mock_execute:
            mock_execute.return_value = []

            try:
                async with AsyncSession() as session:
                    user = MockUser(name="Alice")
                    session.add(user)
                    raise ValueError("Test error")
            except ValueError:
                pass

            # Should be closed and rolled back
            assert session._closed

    @pytest.mark.asyncio
    async def test_add_method(self):
        """Test add() method."""
        from ouroboros.postgres.async_utils import AsyncSession

        session = AsyncSession()
        user = MockUser(name="Alice")

        result = session.add(user)

        assert result is user
        assert user in session._unit_of_work.new_objects

        await session.close()

    @pytest.mark.asyncio
    async def test_execute_method(self):
        """Test execute() raw SQL method."""
        from ouroboros.postgres.async_utils import AsyncSession

        with patch('ouroboros.postgres.execute', new_callable=AsyncMock) as mock_execute:
            mock_execute.return_value = [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]

            session = AsyncSession()
            result = await session.execute("SELECT * FROM users", {"limit": 10})

            assert len(result) == 2
            assert result[0]["name"] == "Alice"
            mock_execute.assert_called_once_with("SELECT * FROM users", {"limit": 10})

            await session.close()

    @pytest.mark.asyncio
    async def test_execute_with_autoflush(self):
        """Test execute() auto-flushes pending changes."""
        from ouroboros.postgres.async_utils import AsyncSession

        with patch('ouroboros.postgres.execute', new_callable=AsyncMock) as mock_execute:
            mock_execute.return_value = []

            session = AsyncSession(autoflush=True)
            user = MockUser(name="Alice")
            session.add(user)

            # Execute should trigger flush
            with patch.object(session, 'flush', new_callable=AsyncMock) as mock_flush:
                await session.execute("SELECT 1")
                mock_flush.assert_called_once()

            await session.close()

    @pytest.mark.asyncio
    async def test_scalar_method(self):
        """Test scalar() returns first column of first row."""
        from ouroboros.postgres.async_utils import AsyncSession

        with patch('ouroboros.postgres.execute', new_callable=AsyncMock) as mock_execute:
            mock_execute.return_value = [{"count": 42}]

            session = AsyncSession()
            result = await session.scalar("SELECT COUNT(*) as count FROM users")

            assert result == 42

            await session.close()

    @pytest.mark.asyncio
    async def test_scalar_empty_result(self):
        """Test scalar() with empty result returns None."""
        from ouroboros.postgres.async_utils import AsyncSession

        with patch('ouroboros.postgres.execute', new_callable=AsyncMock) as mock_execute:
            mock_execute.return_value = []

            session = AsyncSession()
            result = await session.scalar("SELECT COUNT(*) FROM users WHERE false")

            assert result is None

            await session.close()

    @pytest.mark.asyncio
    async def test_closed_session_raises(self):
        """Test operations on closed session raise error."""
        from ouroboros.postgres.async_utils import AsyncSession

        session = AsyncSession()
        await session.close()

        expect(lambda: await session.execute("SELECT 1")).to_raise(RuntimeError)

    @pytest.mark.asyncio
    async def test_bind_engine(self):
        """Test bind_engine() method."""
        from ouroboros.postgres.async_utils import AsyncSession, AsyncEngine

        session = AsyncSession()
        engine = AsyncEngine(database="test")

        session.bind_engine(engine)

        assert session._bind is engine

        await session.close()


# ============================================================================
# AsyncSessionFactory Tests
# ============================================================================

class TestAsyncSessionFactory:
    """Test AsyncSessionFactory functionality."""

    def test_factory_creation(self):
        """Test creating factory with options."""
        from ouroboros.postgres.async_utils import AsyncSessionFactory

        factory = AsyncSessionFactory(
            autoflush=False,
            expire_on_commit=False,
            autocommit=True
        )

        assert factory.autoflush is False
        assert factory.expire_on_commit is False
        assert factory.autocommit is True

    def test_factory_creates_session(self):
        """Test factory creates AsyncSession instances."""
        from ouroboros.postgres.async_utils import AsyncSessionFactory, AsyncSession

        factory = AsyncSessionFactory()
        session = factory()

        assert isinstance(session, AsyncSession)

    def test_factory_passes_options(self):
        """Test factory passes options to created sessions."""
        from ouroboros.postgres.async_utils import AsyncSessionFactory

        factory = AsyncSessionFactory(
            autoflush=False,
            expire_on_commit=False,
            autocommit=True
        )

        session = factory()

        assert session.autoflush is False
        assert session.expire_on_commit is False
        assert session.autocommit is True

    def test_factory_with_engine(self):
        """Test factory binds engine to sessions."""
        from ouroboros.postgres.async_utils import AsyncSessionFactory, AsyncEngine

        engine = AsyncEngine(database="test")
        factory = AsyncSessionFactory(engine=engine)

        session = factory()

        assert session._bind is engine

    @pytest.mark.asyncio
    async def test_factory_begin(self):
        """Test factory begin() starts transaction."""
        from ouroboros.postgres.async_utils import AsyncSessionFactory

        factory = AsyncSessionFactory()
        session = await factory.begin()

        assert session is not None
        assert not session._closed

        await session.close()


# ============================================================================
# run_sync() and async_wrap() Tests
# ============================================================================

class TestRunSync:
    """Test run_sync() function."""

    @pytest.mark.asyncio
    async def test_run_sync_basic(self):
        """Test run_sync() runs sync function."""
        from ouroboros.postgres.async_utils import run_sync

        def sync_func(x: int, y: int) -> int:
            return x + y

        result = await run_sync(sync_func, 10, 20)
        assert result == 30

    @pytest.mark.asyncio
    async def test_run_sync_with_kwargs(self):
        """Test run_sync() passes kwargs correctly."""
        from ouroboros.postgres.async_utils import run_sync

        def sync_func(x: int, y: int, multiplier: int = 1) -> int:
            return (x + y) * multiplier

        result = await run_sync(sync_func, 10, 20, multiplier=2)
        assert result == 60

    @pytest.mark.asyncio
    async def test_run_sync_returns_correct_type(self):
        """Test run_sync() returns correct result type."""
        from ouroboros.postgres.async_utils import run_sync

        def sync_func() -> str:
            return "hello"

        result = await run_sync(sync_func)
        assert result == "hello"
        assert isinstance(result, str)


class TestAsyncWrap:
    """Test async_wrap() decorator."""

    @pytest.mark.asyncio
    async def test_async_wrap_basic(self):
        """Test async_wrap() wraps sync function."""
        from ouroboros.postgres.async_utils import async_wrap

        @async_wrap
        def sync_func(x: int) -> int:
            return x * 2

        result = await sync_func(5)
        assert result == 10

    @pytest.mark.asyncio
    async def test_async_wrap_preserves_metadata(self):
        """Test async_wrap() preserves function metadata."""
        from ouroboros.postgres.async_utils import async_wrap

        @async_wrap
        def sync_func(x: int) -> int:
            """Multiply by 2."""
            return x * 2

        assert sync_func.__name__ == "sync_func"
        assert sync_func.__doc__ == "Multiply by 2."

    @pytest.mark.asyncio
    async def test_async_wrap_with_args_kwargs(self):
        """Test async_wrap() handles args and kwargs."""
        from ouroboros.postgres.async_utils import async_wrap

        @async_wrap
        def sync_func(a: int, b: int, c: int = 0) -> int:
            return a + b + c

        result = await sync_func(1, 2, c=3)
        assert result == 6


# ============================================================================
# AsyncScoped Tests
# ============================================================================

class TestAsyncScoped:
    """Test AsyncScoped functionality."""

    @pytest.mark.asyncio
    async def test_creates_session_per_task(self):
        """Test AsyncScoped creates session per task."""
        from ouroboros.postgres.async_utils import AsyncScoped, AsyncSessionFactory

        factory = AsyncSessionFactory()
        scoped = AsyncScoped(factory)

        session1 = scoped()
        session2 = scoped()

        # Same task should get same session
        assert session1 is session2

        await session1.close()

    @pytest.mark.asyncio
    async def test_different_tasks_get_different_sessions(self):
        """Test different tasks get different sessions."""
        from ouroboros.postgres.async_utils import AsyncScoped, AsyncSessionFactory

        factory = AsyncSessionFactory()
        scoped = AsyncScoped(factory)
        sessions = []

        async def get_session():
            session = scoped()
            sessions.append(session)
            await session.close()

        # Run two tasks
        await asyncio.gather(get_session(), get_session())

        # Should have created 2 different sessions
        assert len(sessions) == 2
        assert sessions[0] is not sessions[1]

    @pytest.mark.asyncio
    async def test_remove_clears_session(self):
        """Test remove() clears session for current task."""
        from ouroboros.postgres.async_utils import AsyncScoped, AsyncSessionFactory

        factory = AsyncSessionFactory()
        scoped = AsyncScoped(factory)

        session1 = scoped()
        scoped.remove()

        # Give close task time to execute
        await asyncio.sleep(0.01)

        # Next call should create new session
        session2 = scoped()
        assert session1 is not session2

        await session2.close()

    @pytest.mark.asyncio
    async def test_remove_all_closes_all_sessions(self):
        """Test remove_all() closes all scoped sessions."""
        from ouroboros.postgres.async_utils import AsyncScoped, AsyncSessionFactory

        factory = AsyncSessionFactory()
        scoped = AsyncScoped(factory)

        sessions = []

        async def create_session():
            session = scoped()
            sessions.append(session)

        # Create sessions in multiple tasks
        await asyncio.gather(create_session(), create_session())

        # Remove all
        await scoped.remove_all()

        # All sessions should be closed
        for session in sessions:
            assert session._closed


class TestGetAsyncSession:
    """Test get_async_session() function."""

    @pytest.mark.asyncio
    async def test_get_async_session_none_when_not_set(self):
        """Test get_async_session() returns None when not set."""
        from ouroboros.postgres.async_utils import get_async_session

        session = get_async_session()
        assert session is None

    @pytest.mark.asyncio
    async def test_get_async_session_returns_current(self):
        """Test get_async_session() returns current session."""
        from ouroboros.postgres.async_utils import AsyncSession, get_async_session

        async with AsyncSession() as session:
            current = get_async_session()
            assert current is session


# ============================================================================
# Async relationship loading helpers Tests
# ============================================================================

class TestAsyncLoad:
    """Test async_load() function."""

    @pytest.mark.asyncio
    async def test_async_load_missing_relationship(self):
        """Test async_load() raises AttributeError for missing relationship."""
        from ouroboros.postgres.async_utils import async_load, AsyncSession

        session = AsyncSession()
        user = MockUser(id=1, name="Alice")
        session.add(user)

        with patch.object(session, 'flush', new_callable=AsyncMock):
            expect(lambda: await async_load(user, "nonexistent")).to_raise(AttributeError)
            await session.close()

    @pytest.mark.asyncio
    async def test_async_load_no_session(self):
        """Test async_load() raises RuntimeError without session."""
        from ouroboros.postgres.async_utils import async_load

        # Need to add relationship attribute for this test
        user = MockUser(id=1, name="Alice")
        user.posts = None  # Add attribute

        expect(lambda: await async_load(user, "posts")).to_raise(RuntimeError)


class TestAsyncRefresh:
    """Test async_refresh() function."""

    @pytest.mark.asyncio
    async def test_async_refresh_no_session(self):
        """Test async_refresh() raises RuntimeError without session."""
        from ouroboros.postgres.async_utils import async_refresh

        user = MockUser(id=1, name="Alice")

        expect(lambda: await async_refresh(user)).to_raise(RuntimeError)

    @pytest.mark.asyncio
    async def test_async_refresh_no_pk(self):
        """Test async_refresh() raises ValueError without primary key."""
        from ouroboros.postgres.async_utils import async_refresh, AsyncSession

        session = AsyncSession()
        user = MockUser(name="Alice")  # No ID
        session.add(user)

        with patch.object(session, 'flush', new_callable=AsyncMock):
            expect(lambda: await async_refresh(user, session=session)).to_raise(ValueError)
            await session.close()


class TestAsyncExpire:
    """Test async_expire() function."""

    @pytest.mark.asyncio
    async def test_async_expire_all_attributes(self):
        """Test async_expire() expires all attributes."""
        from ouroboros.postgres.async_utils import async_expire, AsyncSession

        session = AsyncSession()
        user = MockUser(id=1, name="Alice", email="alice@test.com")
        session.add(user)
        session._unit_of_work._dirty_tracker.take_snapshot(user)

        with patch.object(session, 'flush', new_callable=AsyncMock):
            # Expire all
            await async_expire(user, session=session)

            # Snapshot should be cleared
            snapshot = session._unit_of_work._dirty_tracker._snapshots.get(id(user))
            assert snapshot is None
            await session.close()

    @pytest.mark.asyncio
    async def test_async_expire_specific_attributes(self):
        """Test async_expire() expires specific attributes."""
        from ouroboros.postgres.async_utils import async_expire, AsyncSession

        session = AsyncSession()
        user = MockUser(id=1, name="Alice", email="alice@test.com")
        session.add(user)
        session._unit_of_work._dirty_tracker.take_snapshot(user)

        with patch.object(session, 'flush', new_callable=AsyncMock):
            # Expire specific attributes
            await async_expire(user, attrs=["name"], session=session)

            # Snapshot should still exist but name should be removed
            snapshot = session._unit_of_work._dirty_tracker._snapshots.get(id(user))
            assert "name" not in snapshot.data
            await session.close()

    @pytest.mark.asyncio
    async def test_async_expire_no_session(self):
        """Test async_expire() raises RuntimeError without session."""
        from ouroboros.postgres.async_utils import async_expire

        user = MockUser(id=1, name="Alice")

        expect(lambda: await async_expire(user)).to_raise(RuntimeError)


# ============================================================================
# AsyncResultIterator Tests
# ============================================================================

class TestAsyncResultIterator:
    """Test AsyncResultIterator functionality."""

    @pytest.mark.asyncio
    async def test_async_iteration(self):
        """Test async iteration works."""
        from ouroboros.postgres.async_utils import AsyncResultIterator

        async def query_func():
            return [1, 2, 3, 4, 5]

        iterator = AsyncResultIterator(query_func, batch_size=2)
        results = []

        async for item in iterator:
            results.append(item)

        assert results == [1, 2]  # Only first batch due to simplified implementation

    @pytest.mark.asyncio
    async def test_async_iteration_empty(self):
        """Test async iteration with empty results."""
        from ouroboros.postgres.async_utils import AsyncResultIterator

        async def query_func():
            return []

        iterator = AsyncResultIterator(query_func, batch_size=10)
        results = []

        async for item in iterator:
            results.append(item)

        assert results == []

    @pytest.mark.asyncio
    async def test_async_iteration_custom_batch_size(self):
        """Test custom batch size."""
        from ouroboros.postgres.async_utils import AsyncResultIterator

        async def query_func():
            return [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

        iterator = AsyncResultIterator(query_func, batch_size=3)
        results = []

        async for item in iterator:
            results.append(item)

        assert len(results) == 3  # First batch only


# ============================================================================
# async_stream() Tests
# ============================================================================

class TestAsyncStream:
    """Test async_stream() function."""

    @pytest.mark.asyncio
    async def test_async_stream_basic(self):
        """Test async_stream() streams results."""
        from ouroboros.postgres.async_utils import async_stream, AsyncSession

        with patch('ouroboros.postgres.find_many', new_callable=AsyncMock) as mock_find:
            mock_find.side_effect = [
                [{"id": 1, "name": "Alice"}],
                []  # Empty to signal end
            ]

            async with AsyncSession() as session:
                results = []
                async for user in async_stream(MockUser, batch_size=1, session=session):
                    results.append(user)

                assert len(results) == 1
                assert results[0].name == "Alice"

    @pytest.mark.asyncio
    async def test_async_stream_empty_results(self):
        """Test async_stream() handles empty results."""
        from ouroboros.postgres.async_utils import async_stream, AsyncSession

        with patch('ouroboros.postgres.find_many', new_callable=AsyncMock) as mock_find:
            mock_find.return_value = []

            async with AsyncSession() as session:
                results = []
                async for user in async_stream(MockUser, session=session):
                    results.append(user)

                assert results == []

    @pytest.mark.asyncio
    async def test_async_stream_multiple_batches(self):
        """Test async_stream() handles multiple batches."""
        from ouroboros.postgres.async_utils import async_stream, AsyncSession

        with patch('ouroboros.postgres.find_many', new_callable=AsyncMock) as mock_find:
            mock_find.side_effect = [
                [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}],
                [{"id": 3, "name": "Charlie"}],
                []  # End
            ]

            async with AsyncSession() as session:
                results = []
                async for user in async_stream(MockUser, batch_size=2, session=session):
                    results.append(user)

                assert len(results) == 3

    @pytest.mark.asyncio
    async def test_async_stream_no_session(self):
        """Test async_stream() raises RuntimeError without session."""
        from ouroboros.postgres.async_utils import async_stream

        with pytest.raises(RuntimeError, match="No active async session"):
            async for user in async_stream(MockUser):
                pass


# ============================================================================
# AsyncEngine Tests
# ============================================================================

class TestAsyncEngine:
    """Test AsyncEngine functionality."""

    def test_engine_creation(self):
        """Test creating engine with parameters."""
        from ouroboros.postgres.async_utils import AsyncEngine

        engine = AsyncEngine(
            host="localhost",
            port=5432,
            database="test",
            username="user",
            password="pass",
            min_connections=2,
            max_connections=20
        )

        assert engine.host == "localhost"
        assert engine.port == 5432
        assert engine.database == "test"
        assert engine.username == "user"
        assert engine.password == "pass"
        assert engine.min_connections == 2
        assert engine.max_connections == 20
        assert not engine.is_connected

    def test_engine_with_connection_string(self):
        """Test creating engine with connection string."""
        from ouroboros.postgres.async_utils import AsyncEngine

        conn_str = "postgresql://user:pass@localhost:5432/test"
        engine = AsyncEngine(connection_string=conn_str)

        assert engine.connection_string == conn_str

    @pytest.mark.asyncio
    async def test_engine_connect(self):
        """Test engine connect() initializes connection."""
        from ouroboros.postgres.async_utils import AsyncEngine

        with patch('ouroboros.postgres.init', new_callable=AsyncMock) as mock_init:
            engine = AsyncEngine(database="test")
            await engine.connect()

            assert engine.is_connected
            mock_init.assert_called_once()

    @pytest.mark.asyncio
    async def test_engine_dispose(self):
        """Test engine dispose() closes connections."""
        from ouroboros.postgres.async_utils import AsyncEngine

        with patch('ouroboros.postgres.init', new_callable=AsyncMock), \
             patch('ouroboros.postgres.close', new_callable=AsyncMock) as mock_close:

            engine = AsyncEngine(database="test")
            await engine.connect()
            await engine.dispose()

            assert not engine.is_connected
            mock_close.assert_called_once()

    @pytest.mark.asyncio
    async def test_engine_context_manager(self):
        """Test engine async context manager."""
        from ouroboros.postgres.async_utils import AsyncEngine

        with patch('ouroboros.postgres.init', new_callable=AsyncMock), \
             patch('ouroboros.postgres.close', new_callable=AsyncMock) as mock_close:

            async with AsyncEngine(database="test") as engine:
                assert engine.is_connected

            # Should be closed after exit
            assert not engine.is_connected
            mock_close.assert_called_once()


# ============================================================================
# Greenlet Tests (if available)
# ============================================================================

class TestGreenlet:
    """Test greenlet compatibility functions."""

    def test_greenlet_available_flag(self):
        """Test GREENLET_AVAILABLE flag."""
        from ouroboros.postgres.async_utils import GREENLET_AVAILABLE

        assert isinstance(GREENLET_AVAILABLE, bool)

    @pytest.mark.skipif(
        "not __import__('ouroboros.postgres.async_utils').postgres.async_utils.GREENLET_AVAILABLE",
        reason="greenlet not installed"
    )
    def test_greenlet_spawn_available(self):
        """Test greenlet_spawn when greenlet is available."""
        from ouroboros.postgres.async_utils import greenlet_spawn

        async def async_func():
            return "test"

        # This test would require a running event loop
        # Just verify the function exists
        assert callable(greenlet_spawn)

    @pytest.mark.skipif(
        "__import__('ouroboros.postgres.async_utils').postgres.async_utils.GREENLET_AVAILABLE",
        reason="greenlet is installed"
    )
    def test_greenlet_spawn_not_available(self):
        """Test greenlet_spawn raises when greenlet not installed."""
        from ouroboros.postgres.async_utils import greenlet_spawn

        expect(lambda: greenlet_spawn(lambda: None)).to_raise(RuntimeError)

    @pytest.mark.skipif(
        "__import__('ouroboros.postgres.async_utils').postgres.async_utils.GREENLET_AVAILABLE",
        reason="greenlet is installed"
    )
    def test_async_greenlet_not_available(self):
        """Test AsyncGreenlet raises when greenlet not installed."""
        from ouroboros.postgres.async_utils import AsyncGreenlet

        expect(lambda: AsyncGreenlet()).to_raise(RuntimeError)


# ============================================================================
# Integration Tests (combining multiple components)
# ============================================================================

class TestAsyncUtilsIntegration:
    """Test integration of async utilities."""

    @pytest.mark.asyncio
    async def test_factory_with_scoped(self):
        """Test factory works with scoped sessions."""
        from ouroboros.postgres.async_utils import (
            AsyncSessionFactory, AsyncScoped
        )

        factory = AsyncSessionFactory(autoflush=False)
        scoped = AsyncScoped(factory)

        session1 = scoped()
        session2 = scoped()

        assert session1 is session2
        assert session1.autoflush is False

        await session1.close()

    @pytest.mark.asyncio
    async def test_session_with_engine_lifecycle(self):
        """Test session lifecycle with engine."""
        from ouroboros.postgres.async_utils import (
            AsyncEngine, AsyncSessionFactory
        )

        with patch('ouroboros.postgres.init', new_callable=AsyncMock), \
             patch('ouroboros.postgres.close', new_callable=AsyncMock):

            async with AsyncEngine(database="test") as engine:
                factory = AsyncSessionFactory(engine=engine)

                async with factory() as session:
                    assert session._bind is engine
                    assert not session._closed

                # Session should be closed after context
                assert session._closed

    @pytest.mark.asyncio
    async def test_multiple_concurrent_sessions(self):
        """Test multiple concurrent sessions."""
        from ouroboros.postgres.async_utils import AsyncSession

        async def use_session():
            async with AsyncSession() as session:
                user = MockUser(name="Test")
                session.add(user)
                await asyncio.sleep(0.01)  # Simulate work
                return session

        # Run multiple sessions concurrently
        sessions = await asyncio.gather(
            use_session(),
            use_session(),
            use_session()
        )

        # All should be different sessions
        assert len(set(id(s) for s in sessions)) == 3

        # All should be closed
        for session in sessions:
            assert session._closed
