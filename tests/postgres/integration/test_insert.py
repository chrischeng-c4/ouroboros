"""
Integration tests for PostgreSQL insert operations.

Tests cover:
- Single document insert (insert_one)
- Bulk insert (insert_many)
- Auto-generated IDs
- Error handling for duplicate keys
"""

import pytest
from data_bridge.postgres import init, close, execute, insert_one, insert_many


@pytest.mark.integration
@pytest.mark.asyncio
class TestInsertOne:
    """Test single document insert operations."""

    async def test_insert_one_basic(self):
        """
        Test insert_one inserting a new row with auto-generated ID.

        Verifies that insert_one correctly inserts a document and returns
        the inserted row with the auto-generated ID.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_insert_users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                age INTEGER
            )
        """)

        # Insert new user
        result = await insert_one(
            "test_insert_users",
            {"name": "Alice", "age": 30}
        )

        # Verify insert happened
        assert result["name"] == "Alice"
        assert result["age"] == 30
        assert result["id"] is not None
        assert isinstance(result["id"], int)

        # Verify data in database
        rows = await execute("SELECT * FROM test_insert_users")
        assert len(rows) == 1
        assert rows[0]["name"] == "Alice"
        assert rows[0]["age"] == 30
        assert rows[0]["id"] == result["id"]

    async def test_insert_one_with_unique_constraint(self):
        """
        Test insert_one with unique constraint violation.

        Verifies that insert_one raises appropriate error when trying to
        insert a duplicate value for a unique column.
        """
        # Create test table with unique constraint
        await execute("""
            CREATE TABLE test_insert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT
            )
        """)

        # Insert first user
        result1 = await insert_one(
            "test_insert_users",
            {"email": "alice@example.com", "name": "Alice"}
        )
        assert result1["email"] == "alice@example.com"

        # Try to insert duplicate email (should fail)
        with pytest.raises(Exception) as exc_info:
            await insert_one(
                "test_insert_users",
                {"email": "alice@example.com", "name": "Alice Duplicate"}
            )

        # Verify error indicates unique constraint violation
        error_msg = str(exc_info.value).lower()
        assert "unique" in error_msg or "duplicate" in error_msg or "constraint" in error_msg

    async def test_insert_one_nullable_columns(self):
        """
        Test insert_one with nullable columns.

        Verifies that insert_one correctly handles NULL values for nullable columns.
        """
        # Create test table with nullable columns
        await execute("""
            CREATE TABLE test_insert_nullable (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT,
                age INTEGER
            )
        """)

        # Insert user with some NULL values
        result = await insert_one(
            "test_insert_nullable",
            {"name": "Bob"}  # email and age are NULL
        )

        # Verify insert happened
        assert result["name"] == "Bob"
        assert result["email"] is None
        assert result["age"] is None
        assert result["id"] is not None

        # Verify data in database
        rows = await execute("SELECT * FROM test_insert_nullable")
        assert len(rows) == 1
        assert rows[0]["name"] == "Bob"
        assert rows[0]["email"] is None
        assert rows[0]["age"] is None


@pytest.mark.integration
@pytest.mark.asyncio
class TestInsertMany:
    """Test bulk insert operations."""

    async def test_insert_many_basic(self):
        """
        Test insert_many inserting multiple new rows.

        Verifies that insert_many can efficiently insert multiple documents
        in a single operation.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_insert_users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                age INTEGER
            )
        """)

        # Insert multiple users
        users = [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25},
            {"name": "Charlie", "age": 35},
        ]

        results = await insert_many("test_insert_users", users)

        # Verify all inserts happened
        assert len(results) == 3
        assert all(r["id"] is not None for r in results)
        assert {r["name"] for r in results} == {"Alice", "Bob", "Charlie"}

        # Verify all IDs are unique
        ids = [r["id"] for r in results]
        assert len(set(ids)) == 3

        # Verify data in database
        rows = await execute("SELECT * FROM test_insert_users ORDER BY name")
        assert len(rows) == 3
        assert rows[0]["name"] == "Alice"
        assert rows[1]["name"] == "Bob"
        assert rows[2]["name"] == "Charlie"

    async def test_insert_many_large_batch(self):
        """
        Test insert_many with large batch (100+ rows).

        Verifies that bulk inserts work correctly with large batches.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_insert_users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                age INTEGER
            )
        """)

        # Generate large batch (100 users)
        large_batch = [
            {"name": f"User {i}", "age": 20 + (i % 50)}
            for i in range(100)
        ]

        # Insert large batch
        results = await insert_many("test_insert_users", large_batch)

        # Verify all inserts happened
        assert len(results) == 100
        assert all(r["id"] is not None for r in results)

        # Verify all IDs are unique
        ids = [r["id"] for r in results]
        assert len(set(ids)) == 100

        # Verify data in database
        rows = await execute("SELECT COUNT(*) as count FROM test_insert_users")
        assert rows[0]["count"] == 100

    async def test_insert_many_empty_list(self):
        """
        Test insert_many with empty document list.

        Verifies that insert_many handles empty input gracefully.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_insert_users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                age INTEGER
            )
        """)

        # Insert empty list
        results = await insert_many("test_insert_users", [])

        # Should return empty list
        assert results == []

        # Verify no rows in database
        rows = await execute("SELECT * FROM test_insert_users")
        assert len(rows) == 0

    async def test_insert_many_with_unique_constraint_violation(self):
        """
        Test insert_many with unique constraint violation.

        Verifies that insert_many raises appropriate error when batch
        contains duplicate values for a unique column.
        """
        # Create test table with unique constraint
        await execute("""
            CREATE TABLE test_insert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT
            )
        """)

        # Try to insert batch with duplicate emails
        users = [
            {"email": "alice@example.com", "name": "Alice"},
            {"email": "bob@example.com", "name": "Bob"},
            {"email": "alice@example.com", "name": "Alice Duplicate"},  # Duplicate!
        ]

        # Should fail due to duplicate email in batch
        with pytest.raises(Exception) as exc_info:
            await insert_many("test_insert_users", users)

        # Verify error indicates unique constraint violation
        error_msg = str(exc_info.value).lower()
        assert "unique" in error_msg or "duplicate" in error_msg or "constraint" in error_msg


@pytest.mark.integration
@pytest.mark.asyncio
class TestInsertErrors:
    """Test error handling for insert operations."""

    async def test_insert_table_not_exists(self):
        """
        Test insert operations on non-existent table.

        Verifies that appropriate error is raised when table doesn't exist.
        """
        # Try to insert to non-existent table
        with pytest.raises(Exception) as exc_info:
            await insert_one(
                "nonexistent_table",
                {"name": "Alice"}
            )

        # Verify error indicates table doesn't exist
        error_msg = str(exc_info.value).lower()
        assert "table" in error_msg or "relation" in error_msg or "not" in error_msg

    async def test_insert_missing_required_column(self):
        """
        Test insert_one with missing required (NOT NULL) column.

        Verifies that appropriate error is raised when a required column
        is not provided in the document.
        """
        # Create test table with NOT NULL constraint
        await execute("""
            CREATE TABLE test_insert_users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL
            )
        """)

        # Try to insert without required email column
        with pytest.raises(Exception) as exc_info:
            await insert_one(
                "test_insert_users",
                {"name": "Alice"}  # Missing required 'email'
            )

        # Verify error indicates NULL constraint violation
        error_msg = str(exc_info.value).lower()
        assert "null" in error_msg or "not" in error_msg or "constraint" in error_msg
