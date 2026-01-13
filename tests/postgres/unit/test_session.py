"""
Unit tests for Session and Unit of Work functionality.

Tests the ORM session pattern without requiring a database connection.
"""
import pytest
from data_bridge.test import expect
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


class TestIdentityMap:
    """Test IdentityMap functionality."""

    def test_add_and_get(self):
        """Test adding and retrieving objects."""
        from data_bridge.postgres.session import IdentityMap

        imap = IdentityMap(use_weak_refs=False)
        user = MockUser(id=1, name="Alice")

        imap.add("users", 1, user)
        retrieved = imap.get("users", 1)

        assert retrieved is user

    def test_get_nonexistent(self):
        """Test getting nonexistent object returns None."""
        from data_bridge.postgres.session import IdentityMap

        imap = IdentityMap(use_weak_refs=False)
        assert imap.get("users", 999) is None

    def test_contains(self):
        """Test contains check."""
        from data_bridge.postgres.session import IdentityMap

        imap = IdentityMap(use_weak_refs=False)
        user = MockUser(id=1, name="Alice")

        assert not imap.contains("users", 1)
        imap.add("users", 1, user)
        assert imap.contains("users", 1)

    def test_remove(self):
        """Test removing objects."""
        from data_bridge.postgres.session import IdentityMap

        imap = IdentityMap(use_weak_refs=False)
        user = MockUser(id=1, name="Alice")

        imap.add("users", 1, user)
        imap.remove("users", 1)

        assert imap.get("users", 1) is None

    def test_clear(self):
        """Test clearing all entries."""
        from data_bridge.postgres.session import IdentityMap

        imap = IdentityMap(use_weak_refs=False)
        imap.add("users", 1, MockUser(id=1))
        imap.add("users", 2, MockUser(id=2))

        imap.clear()

        assert len(imap) == 0

    def test_length(self):
        """Test length property."""
        from data_bridge.postgres.session import IdentityMap

        imap = IdentityMap(use_weak_refs=False)
        assert len(imap) == 0

        imap.add("users", 1, MockUser(id=1))
        assert len(imap) == 1

        imap.add("users", 2, MockUser(id=2))
        assert len(imap) == 2


class TestDirtyTracker:
    """Test DirtyTracker functionality."""

    def test_take_snapshot(self):
        """Test taking snapshot."""
        from data_bridge.postgres.session import DirtyTracker

        tracker = DirtyTracker()
        user = MockUser(id=1, name="Alice", email="alice@test.com")

        tracker.take_snapshot(user)
        # Should not be dirty immediately after snapshot
        assert not tracker.is_dirty(user)

    def test_detect_dirty_field(self):
        """Test detecting dirty fields."""
        from data_bridge.postgres.session import DirtyTracker

        tracker = DirtyTracker()
        user = MockUser(id=1, name="Alice", email="alice@test.com")

        tracker.take_snapshot(user)
        user.name = "Bob"

        assert tracker.is_dirty(user)
        dirty = tracker.get_dirty_fields(user)
        assert "name" in dirty
        assert dirty["name"] == ("Alice", "Bob")

    def test_detect_multiple_dirty_fields(self):
        """Test detecting multiple dirty fields."""
        from data_bridge.postgres.session import DirtyTracker

        tracker = DirtyTracker()
        user = MockUser(id=1, name="Alice", email="alice@test.com")

        tracker.take_snapshot(user)
        user.name = "Bob"
        user.email = "bob@test.com"

        dirty = tracker.get_dirty_fields(user)
        assert len(dirty) == 2
        assert "name" in dirty
        assert "email" in dirty

    def test_refresh_snapshot(self):
        """Test refreshing snapshot clears dirty state."""
        from data_bridge.postgres.session import DirtyTracker

        tracker = DirtyTracker()
        user = MockUser(id=1, name="Alice")

        tracker.take_snapshot(user)
        user.name = "Bob"
        assert tracker.is_dirty(user)

        tracker.refresh_snapshot(user)
        assert not tracker.is_dirty(user)

    def test_clear_snapshot(self):
        """Test clearing snapshot for specific object."""
        from data_bridge.postgres.session import DirtyTracker

        tracker = DirtyTracker()
        user = MockUser(id=1, name="Alice")

        tracker.take_snapshot(user)
        tracker.clear_snapshot(user)

        # After clearing, get_dirty_fields returns empty
        assert tracker.get_dirty_fields(user) == {}


class TestUnitOfWork:
    """Test UnitOfWork functionality."""

    def test_register_new(self):
        """Test registering new objects."""
        from data_bridge.postgres.session import UnitOfWork

        uow = UnitOfWork()
        user = MockUser(name="Alice")

        uow.register_new(user)

        assert user in uow.new_objects
        assert uow.has_pending

    def test_register_new_idempotent(self):
        """Test registering same object twice doesn't duplicate."""
        from data_bridge.postgres.session import UnitOfWork

        uow = UnitOfWork()
        user = MockUser(name="Alice")

        uow.register_new(user)
        uow.register_new(user)

        assert len(uow.new_objects) == 1

    def test_register_deleted(self):
        """Test registering deleted objects."""
        from data_bridge.postgres.session import UnitOfWork

        uow = UnitOfWork()
        user = MockUser(id=1, name="Alice")

        uow.register_deleted(user)

        assert user in uow.deleted_objects
        assert uow.has_pending

    def test_register_deleted_removes_from_new(self):
        """Test deleting new object removes it from new list."""
        from data_bridge.postgres.session import UnitOfWork

        uow = UnitOfWork()
        user = MockUser(name="Alice")

        uow.register_new(user)
        uow.register_deleted(user)

        assert user not in uow.new_objects
        assert user in uow.deleted_objects

    def test_register_clean(self):
        """Test registering clean objects takes snapshot."""
        from data_bridge.postgres.session import UnitOfWork

        uow = UnitOfWork()
        user = MockUser(id=1, name="Alice")

        uow.register_clean(user)

        # Modify and check dirty
        user.name = "Bob"
        assert uow.is_dirty(user)

    def test_clear(self):
        """Test clearing all pending operations."""
        from data_bridge.postgres.session import UnitOfWork

        uow = UnitOfWork()
        uow.register_new(MockUser(name="Alice"))
        uow.register_deleted(MockUser(id=1))

        uow.clear()

        assert not uow.has_pending
        assert len(uow.new_objects) == 0
        assert len(uow.deleted_objects) == 0


class TestSession:
    """Test Session functionality."""

    def test_add_object(self):
        """Test adding object to session."""
        from data_bridge.postgres.session import Session

        session = Session()
        user = MockUser(name="Alice")

        result = session.add(user)

        assert result is user

    def test_add_returns_existing(self):
        """Test adding object with same PK returns existing."""
        from data_bridge.postgres.session import Session

        session = Session()
        user1 = MockUser(id=1, name="Alice")
        user2 = MockUser(id=1, name="Bob")

        session.add(user1)
        result = session.add(user2)

        # Should return the first one (identity map)
        assert result is user1

    def test_delete_object(self):
        """Test deleting object from session."""
        from data_bridge.postgres.session import Session

        session = Session()
        user = MockUser(id=1, name="Alice")

        session.add(user)
        session.delete(user)

        # Should be in deleted list
        assert user in session._unit_of_work.deleted_objects

    def test_expunge_object(self):
        """Test expunging object from session."""
        from data_bridge.postgres.session import Session

        session = Session()
        user = MockUser(id=1, name="Alice")

        session.add(user)
        session.expunge(user)

        # Should not be tracked anymore
        assert user not in session._unit_of_work.new_objects
        assert not session._identity_map.contains("users", 1)

    def test_expunge_all(self):
        """Test expunging all objects."""
        from data_bridge.postgres.session import Session

        session = Session()
        session.add(MockUser(id=1, name="Alice"))
        session.add(MockUser(id=2, name="Bob"))

        session.expunge_all()

        assert len(session._identity_map) == 0
        assert not session._unit_of_work.has_pending

    def test_is_modified(self):
        """Test checking if object is modified."""
        from data_bridge.postgres.session import Session

        session = Session()
        user = MockUser(id=1, name="Alice")

        # New object is considered modified
        session.add(user)
        assert session.is_modified(user)

    def test_closed_session_raises(self):
        """Test operations on closed session raise error."""
        from data_bridge.postgres.session import Session
        import asyncio

        async def test():
            session = Session()
            await session.close()

            expect(lambda: session.add(MockUser(name="Alice"))).to_raise(RuntimeError)

        asyncio.run(test())

    def test_get_session(self):
        """Test getting current session."""
        from data_bridge.postgres.session import Session, get_session

        # No session active
        assert get_session() is None

        # With active session - test without context manager auto-commit
        session = Session()
        Session._current = session
        assert get_session() is session

        # Clear current
        Session._current = None
        assert get_session() is None


class TestObjectState:
    """Test ObjectState enum."""

    def test_all_states_exist(self):
        """Test all expected states exist."""
        from data_bridge.postgres.session import ObjectState

        assert ObjectState.TRANSIENT
        assert ObjectState.PENDING
        assert ObjectState.PERSISTENT
        assert ObjectState.DIRTY
        assert ObjectState.DELETED
        assert ObjectState.DETACHED
