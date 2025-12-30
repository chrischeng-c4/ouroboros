"""
Unit tests for CRUD operations.

Tests save(), delete(), refresh(), and other instance methods without
requiring a real database connection.
"""
import pytest
from unittest.mock import AsyncMock, patch
from data_bridge.postgres import Table, Column
from data_bridge.test import expect


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
            expect(user.id).to_equal(1)
            expect(result_id).to_equal(1)

    @pytest.mark.asyncio
    async def test_save_update_existing(self, User):
        """Test save() performs update for existing records."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.update_one = AsyncMock(return_value=1)

            user = User(id=5, name="Alice", email="alice@example.com")
            result_id = await user.save()

            # Should call update_one
            mock_engine.update_one.assert_called_once()
            expect(result_id).to_equal(1)

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

            expect(data["name"]).to_equal("Alice")
            expect(data["email"]).to_equal("alice@example.com")
            expect(data["age"]).to_equal(30)

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
            expect("id" not in data).to_be_true()


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
            expect(result).to_be_true()

    @pytest.mark.asyncio
    async def test_delete_without_id(self, User):
        """Test delete() returns False when no id."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            user = User(name="Alice", email="alice@example.com")
            result = await user.delete()

            # Should not call engine
            mock_engine.delete_one.assert_not_called()
            expect(result).to_be_false()

    @pytest.mark.asyncio
    async def test_delete_not_found(self, User):
        """Test delete() returns False when row not found."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.delete_one = AsyncMock(return_value=0)

            user = User(id=999, name="Alice", email="alice@example.com")
            result = await user.delete()

            expect(result).to_be_false()


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
            expect(user.name).to_equal("Alice Updated")
            expect(user.email).to_equal("alice.new@example.com")
            expect(user.age).to_equal(31)

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

            expect(isinstance(user, User)).to_be_true()
            expect(user.id).to_equal(1)
            expect(user.name).to_equal("Alice")
            expect(user.email).to_equal("alice@example.com")

    @pytest.mark.asyncio
    async def test_get_not_found(self, User):
        """Test get() returns None when row not found."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.find_one = AsyncMock(return_value=None)

            user = await User.get(999)

            expect(user).to_be_none()

    @pytest.mark.asyncio
    async def test_get_calls_engine_with_pk(self, User):
        """Test get() calls engine with primary key."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.find_one = AsyncMock(return_value=None)

            await User.get(5)

            # Should call find_one with table name, pk column, and value
            mock_engine.find_one.assert_called_once()
            call_args = mock_engine.find_one.call_args[0]
            expect("public.users" in call_args[0]).to_be_true()
            expect(call_args[1]).to_equal("id")
            expect(call_args[2]).to_equal(5)


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

            expect(ids).to_equal([1, 2, 3])
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

            expect(ids).to_equal([1, 2])
            # Should convert instances to dicts
            call_args = mock_engine.insert_many.call_args[0]
            data = call_args[1]
            expect(data[0]["name"]).to_equal("Alice")
            expect(data[1]["name"]).to_equal("Bob")

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
    async def test_delete_many_with_filters(self, User):
        """Test delete_many() with filters works correctly."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.delete_many = AsyncMock(return_value=5)
            result = await User.delete_many(User.age < 18)
            expect(result).to_equal(5)
            # Verify delete_many was called with correct arguments
            mock_engine.delete_many.assert_called_once()
            args = mock_engine.delete_many.call_args[0]
            expect(args[0]).to_equal("public.users")  # table name
            expect("age < $1" in args[1]).to_be_true()  # WHERE clause
            expect(args[2]).to_equal([18])  # parameters

    @pytest.mark.asyncio
    async def test_delete_many_no_filters(self, User):
        """Test delete_many() without filters (delete all)."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.delete_many = AsyncMock(return_value=10)
            result = await User.delete_many()
            expect(result).to_equal(10)
            # Verify delete_many was called with empty WHERE clause
            mock_engine.delete_many.assert_called_once()
            args = mock_engine.delete_many.call_args[0]
            expect(args[0]).to_equal("public.users")  # table name
            expect(args[1]).to_equal("")  # Empty WHERE clause
            expect(args[2]).to_equal([])  # No parameters

    @pytest.mark.asyncio
    async def test_delete_many_with_dict_filter(self, User):
        """Test delete_many() with dict filter."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.delete_many = AsyncMock(return_value=3)
            result = await User.delete_many({"name": "Alice"})
            expect(result).to_equal(3)
            # Verify delete_many was called with correct arguments
            mock_engine.delete_many.assert_called_once()
            args = mock_engine.delete_many.call_args[0]
            expect(args[0]).to_equal("public.users")  # table name
            expect("name = $1" in args[1]).to_be_true()  # WHERE clause
            expect(args[2]).to_equal(["Alice"])  # parameters


class TestUpdateMany:
    """Test update_many() class method."""

    @pytest.mark.asyncio
    async def test_update_many_raises_without_engine(self, User):
        """Test update_many() raises RuntimeError when engine not available."""
        with patch('data_bridge.postgres.table._engine', None):
            with pytest.raises(RuntimeError, match="PostgreSQL engine not available"):
                await User.update_many({"status": "active"}, User.age >= 18)

    @pytest.mark.asyncio
    async def test_update_many_with_filters(self, User):
        """Test update_many() with filters works correctly."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.update_many = AsyncMock(return_value=5)
            result = await User.update_many({"status": "active"}, User.age >= 18)
            expect(result).to_equal(5)
            # Verify update_many was called with correct arguments
            mock_engine.update_many.assert_called_once()
            args = mock_engine.update_many.call_args[0]
            expect(args[0]).to_equal("public.users")  # table name
            expect(args[1]).to_equal({"status": "active"})  # updates
            expect("age >= $1" in args[2]).to_be_true()  # WHERE clause
            expect(args[3]).to_equal([18])  # parameters

    @pytest.mark.asyncio
    async def test_update_many_no_filters(self, User):
        """Test update_many() without filters (update all)."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.update_many = AsyncMock(return_value=10)
            result = await User.update_many({"status": "inactive"})
            expect(result).to_equal(10)
            # Verify update_many was called with empty WHERE clause
            mock_engine.update_many.assert_called_once()
            args = mock_engine.update_many.call_args[0]
            expect(args[0]).to_equal("public.users")  # table name
            expect(args[1]).to_equal({"status": "inactive"})  # updates
            expect(args[2]).to_equal("")  # Empty WHERE clause
            expect(args[3]).to_equal([])  # No parameters

    @pytest.mark.asyncio
    async def test_update_many_with_dict_filter(self, User):
        """Test update_many() with dict filter."""
        with patch('data_bridge.postgres.table._engine') as mock_engine:
            mock_engine.update_many = AsyncMock(return_value=3)
            result = await User.update_many({"age": 31}, {"name": "Alice"})
            expect(result).to_equal(3)
            # Verify update_many was called with correct arguments
            mock_engine.update_many.assert_called_once()
            args = mock_engine.update_many.call_args[0]
            expect(args[0]).to_equal("public.users")  # table name
            expect(args[1]).to_equal({"age": 31})  # updates
            expect("name = $1" in args[2]).to_be_true()  # WHERE clause
            expect(args[3]).to_equal(["Alice"])  # parameters


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
            expect(isinstance(user, User)).to_be_true()
            expect(user.email).to_equal("alice@example.com")

    @pytest.mark.asyncio
    async def test_find_one_returns_none(self, User):
        """Test find_one() returns None when not found."""
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.find_many = AsyncMock(return_value=[])

            user = await User.find_one(User.email == "notfound@example.com")

            expect(user).to_be_none()


class TestCount:
    """Test count() class method."""

    @pytest.mark.asyncio
    async def test_count_delegates_to_find(self, User):
        """Test count() delegates to find().count()."""
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.count = AsyncMock(return_value=42)

            count = await User.count(User.age > 18)

            expect(count).to_equal(42)

    @pytest.mark.asyncio
    async def test_count_all(self, User):
        """Test count() with no filters."""
        with patch('data_bridge.postgres.query._engine') as mock_engine:
            mock_engine.count = AsyncMock(return_value=100)

            count = await User.count()

            expect(count).to_equal(100)
