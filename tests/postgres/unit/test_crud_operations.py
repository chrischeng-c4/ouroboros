"""
Unit tests for CRUD operations.

Tests save(), delete(), refresh(), and other instance methods without
requiring a real database connection.
"""
import pytest
from unittest.mock import AsyncMock, patch
from data_bridge.postgres import Table, Column


@pytest.fixture
def User():
    """Sample User table for CRUD tests."""

    class User(Table):
        name: str
        email: str
        age: int = 0

        class Settings:
            table_name = "users"
            schema = "public"
            primary_key = "id"

    return User


class TestSaveOperation:
    """Test save() method."""

    @pytest.mark.asyncio
    async def test_save_raises_without_engine(self, User):
        """Test save() raises RuntimeError when engine not available."""
        user = User(name="Alice", email="alice@example.com")

        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await user.save()

    @pytest.mark.asyncio
    async def test_save_insert_new(self, User):
        """Test save() performs insert for new records."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.insert_one = AsyncMock(return_value=1)

            user = User(name="Alice", email="alice@example.com")
            result_id = await user.save()

            # Should call insert_one
            mock_engine.insert_one.assert_called_once()
            # Should set the id
            assert user.id == 1
            assert result_id == 1

    @pytest.mark.asyncio
    async def test_save_update_existing(self, User):
        """Test save() performs update for existing records."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.update_one = AsyncMock(return_value=1)

            user = User(id=5, name="Alice", email="alice@example.com")
            result_id = await user.save()

            # Should call update_one
            mock_engine.update_one.assert_called_once()
            assert result_id == 1

    @pytest.mark.asyncio
    async def test_save_insert_includes_data(self, User):
        """Test save() passes data to insert_one."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.insert_one = AsyncMock(return_value=1)

            user = User(name="Alice", email="alice@example.com", age=30)
            await user.save()

            # Check what data was passed
            call_args = mock_engine.insert_one.call_args[0]
            data = call_args[1]

            assert data["name"] == "Alice"
            assert data["email"] == "alice@example.com"
            assert data["age"] == 30

    @pytest.mark.asyncio
    async def test_save_update_excludes_id_from_data(self, User):
        """Test save() doesn't include id in update data."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.update_one = AsyncMock(return_value=1)

            user = User(id=5, name="Alice", email="alice@example.com")
            await user.save()

            # Check what data was passed
            call_args = mock_engine.update_one.call_args[0]
            data = call_args[3]

            # id should not be in the update data
            assert "id" not in data


class TestDeleteOperation:
    """Test delete() method."""

    @pytest.mark.asyncio
    async def test_delete_raises_without_engine(self, User):
        """Test delete() raises RuntimeError when engine not available."""
        user = User(id=1, name="Alice", email="alice@example.com")

        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await user.delete()

    @pytest.mark.asyncio
    async def test_delete_with_id(self, User):
        """Test delete() calls engine.delete_one."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.delete_one = AsyncMock(return_value=1)

            user = User(id=5, name="Alice", email="alice@example.com")
            result = await user.delete()

            # Should call delete_one
            mock_engine.delete_one.assert_called_once()
            assert result is True

    @pytest.mark.asyncio
    async def test_delete_without_id(self, User):
        """Test delete() returns False when no id."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            user = User(name="Alice", email="alice@example.com")
            result = await user.delete()

            # Should not call engine
            mock_engine.delete_one.assert_not_called()
            assert result is False

    @pytest.mark.asyncio
    async def test_delete_not_found(self, User):
        """Test delete() returns False when row not found."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.delete_one = AsyncMock(return_value=0)

            user = User(id=999, name="Alice", email="alice@example.com")
            result = await user.delete()

            assert result is False


class TestRefreshOperation:
    """Test refresh() method."""

    @pytest.mark.asyncio
    async def test_refresh_raises_without_engine(self, User):
        """Test refresh() raises RuntimeError when engine not available."""
        user = User(id=1, name="Alice", email="alice@example.com")

        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await user.refresh()

    @pytest.mark.asyncio
    async def test_refresh_raises_without_id(self, User):
        """Test refresh() raises ValueError when no id."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            user = User(name="Alice", email="alice@example.com")

            with pytest.raises(ValueError, match="Cannot refresh a row without an id"):
                await user.refresh()

    @pytest.mark.asyncio
    async def test_refresh_updates_data(self, User):
        """Test refresh() updates instance data from database."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.find_one = AsyncMock(return_value={
                "id": 5,
                "name": "Alice Updated",
                "email": "alice.new@example.com",
                "age": 31,
            })

            user = User(id=5, name="Alice", email="alice@example.com", age=30)
            await user.refresh()

            # Data should be updated
            assert user.name == "Alice Updated"
            assert user.email == "alice.new@example.com"
            assert user.age == 31

    @pytest.mark.asyncio
    async def test_refresh_not_found(self, User):
        """Test refresh() raises ValueError when row not found."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.find_one = AsyncMock(return_value=None)

            user = User(id=999, name="Alice", email="alice@example.com")

            with pytest.raises(ValueError, match="Row with id 999 not found"):
                await user.refresh()


class TestGetOperation:
    """Test get() class method."""

    @pytest.mark.asyncio
    async def test_get_raises_without_engine(self, User):
        """Test get() raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await User.get(1)

    @pytest.mark.asyncio
    async def test_get_returns_instance(self, User):
        """Test get() returns Table instance."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.find_one = AsyncMock(return_value={
                "id": 1,
                "name": "Alice",
                "email": "alice@example.com",
                "age": 30,
            })

            user = await User.get(1)

            assert isinstance(user, User)
            assert user.id == 1
            assert user.name == "Alice"
            assert user.email == "alice@example.com"

    @pytest.mark.asyncio
    async def test_get_not_found(self, User):
        """Test get() returns None when row not found."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.find_one = AsyncMock(return_value=None)

            user = await User.get(999)

            assert user is None

    @pytest.mark.asyncio
    async def test_get_calls_engine_with_pk(self, User):
        """Test get() calls engine with primary key."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.find_one = AsyncMock(return_value=None)

            await User.get(5)

            # Should call find_one with table name, pk column, and value
            mock_engine.find_one.assert_called_once()
            call_args = mock_engine.find_one.call_args[0]
            assert "public.users" in call_args[0]
            assert call_args[1] == "id"
            assert call_args[2] == 5


class TestInsertMany:
    """Test insert_many() class method."""

    @pytest.mark.asyncio
    async def test_insert_many_raises_without_engine(self, User):
        """Test insert_many() raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await User.insert_many([{"name": "Alice", "email": "alice@example.com"}])

    @pytest.mark.asyncio
    async def test_insert_many_with_dicts(self, User):
        """Test insert_many() with list of dictionaries."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.insert_many = AsyncMock(return_value=[1, 2, 3])

            rows = [
                {"name": "Alice", "email": "alice@example.com"},
                {"name": "Bob", "email": "bob@example.com"},
                {"name": "Charlie", "email": "charlie@example.com"},
            ]

            ids = await User.insert_many(rows)

            assert ids == [1, 2, 3]
            mock_engine.insert_many.assert_called_once()

    @pytest.mark.asyncio
    async def test_insert_many_with_instances(self, User):
        """Test insert_many() with list of Table instances."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.insert_many = AsyncMock(return_value=[1, 2])

            rows = [
                User(name="Alice", email="alice@example.com"),
                User(name="Bob", email="bob@example.com"),
            ]

            ids = await User.insert_many(rows)

            assert ids == [1, 2]
            # Should convert instances to dicts
            call_args = mock_engine.insert_many.call_args[0]
            data = call_args[1]
            assert data[0]["name"] == "Alice"
            assert data[1]["name"] == "Bob"

    @pytest.mark.asyncio
    async def test_insert_many_mixed_types_raises(self, User):
        """Test insert_many() raises TypeError for invalid types."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            rows = [
                {"name": "Alice", "email": "alice@example.com"},
                "invalid",  # Invalid type
            ]

            with pytest.raises(TypeError, match="Expected dict or User instance"):
                await User.insert_many(rows)


class TestDeleteMany:
    """Test delete_many() class method."""

    @pytest.mark.asyncio
    async def test_delete_many_raises_without_engine(self, User):
        """Test delete_many() raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await User.delete_many(User.age < 18)

    @pytest.mark.asyncio
    async def test_delete_many_not_implemented(self, User):
        """Test delete_many() with filters raises NotImplementedError."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            # Filter conversion not yet implemented
            with pytest.raises(NotImplementedError):
                await User.delete_many(User.age < 18)


class TestUpdateMany:
    """Test update_many() class method."""

    @pytest.mark.asyncio
    async def test_update_many_raises_without_engine(self, User):
        """Test update_many() raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await User.update_many({"status": "active"}, User.age >= 18)

    @pytest.mark.asyncio
    async def test_update_many_not_implemented(self, User):
        """Test update_many() with filters raises NotImplementedError."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            # Filter conversion not yet implemented
            with pytest.raises(NotImplementedError):
                await User.update_many({"status": "active"}, User.age >= 18)


class TestFindOne:
    """Test find_one() class method."""

    @pytest.mark.asyncio
    async def test_find_one_delegates_to_find(self, User):
        """Test find_one() delegates to find().first()."""
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.find_many = AsyncMock(return_value=[
                {"id": 1, "name": "Alice", "email": "alice@example.com", "age": 30}
            ])

            user = await User.find_one(User.email == "alice@example.com")

            # Should return an instance
            assert isinstance(user, User)
            assert user.email == "alice@example.com"

    @pytest.mark.asyncio
    async def test_find_one_returns_none(self, User):
        """Test find_one() returns None when not found."""
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.find_many = AsyncMock(return_value=[])

            user = await User.find_one(User.email == "notfound@example.com")

            assert user is None


class TestCount:
    """Test count() class method."""

    @pytest.mark.asyncio
    async def test_count_delegates_to_find(self, User):
        """Test count() delegates to find().count()."""
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.count = AsyncMock(return_value=42)

            count = await User.count(User.age > 18)

            assert count == 42

    @pytest.mark.asyncio
    async def test_count_all(self, User):
        """Test count() with no filters."""
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.count = AsyncMock(return_value=100)

            count = await User.count()

            assert count == 100
