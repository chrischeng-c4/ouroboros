"""
Integration tests for raw SQL execution.

These tests require a running PostgreSQL instance.

Set POSTGRES_URI environment variable to override default:
    export POSTGRES_URI="postgresql://user:pass@localhost:5432/test_db"

Run with:
    pytest tests/postgres/integration/test_execute_integration.py -v
"""

import os
import pytest
from data_bridge.postgres import init, close, execute


# Get PostgreSQL URI from environment or use default
POSTGRES_URI = os.getenv("POSTGRES_URI", "postgresql://localhost:5432/test_db")


@pytest.fixture(scope="module")
async def postgres_connection():
    """Initialize PostgreSQL connection for integration tests."""
    await init(POSTGRES_URI)
    yield
    await close()


@pytest.fixture
async def test_table(postgres_connection):
    """Create a test table for integration tests."""
    # Create test table
    await execute("""
        CREATE TABLE IF NOT EXISTS test_execute_users (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT,
            age INTEGER,
            created_at TIMESTAMP DEFAULT NOW()
        )
    """)

    # Clean up any existing data
    await execute("TRUNCATE TABLE test_execute_users RESTART IDENTITY")

    yield

    # Cleanup after tests
    await execute("DROP TABLE IF EXISTS test_execute_users")


@pytest.mark.asyncio
@pytest.mark.integration
class TestExecuteIntegration:
    """Integration tests for execute function with real PostgreSQL."""

    async def test_execute_select_empty(self, test_table):
        """Test SELECT on empty table returns empty list."""
        results = await execute("SELECT * FROM test_execute_users")
        assert results == []

    async def test_execute_insert_and_select(self, test_table):
        """Test INSERT followed by SELECT."""
        # Insert a row
        count = await execute(
            "INSERT INTO test_execute_users (name, email, age) VALUES ($1, $2, $3)",
            ["Alice", "alice@example.com", 30]
        )
        assert count == 1

        # Select all rows
        results = await execute("SELECT * FROM test_execute_users")
        assert len(results) == 1
        assert results[0]["name"] == "Alice"
        assert results[0]["email"] == "alice@example.com"
        assert results[0]["age"] == 30

    async def test_execute_insert_with_returning(self, test_table):
        """Test INSERT with RETURNING clause."""
        results = await execute(
            "INSERT INTO test_execute_users (name, email, age) VALUES ($1, $2, $3) RETURNING id, name",
            ["Bob", "bob@example.com", 25]
        )
        # INSERT with RETURNING is treated as SELECT
        assert len(results) == 1
        assert results[0]["name"] == "Bob"
        assert "id" in results[0]

    async def test_execute_update(self, test_table):
        """Test UPDATE query."""
        # Insert a row
        await execute(
            "INSERT INTO test_execute_users (name, age) VALUES ($1, $2)",
            ["Charlie", 20]
        )

        # Update the row
        count = await execute(
            "UPDATE test_execute_users SET age = $1 WHERE name = $2",
            [21, "Charlie"]
        )
        assert count == 1

        # Verify update
        results = await execute(
            "SELECT age FROM test_execute_users WHERE name = $1",
            ["Charlie"]
        )
        assert results[0]["age"] == 21

    async def test_execute_delete(self, test_table):
        """Test DELETE query."""
        # Insert rows
        await execute(
            "INSERT INTO test_execute_users (name, age) VALUES ($1, $2), ($3, $4)",
            ["Dave", 15, "Eve", 25]
        )

        # Delete rows where age < 18
        count = await execute(
            "DELETE FROM test_execute_users WHERE age < $1",
            [18]
        )
        assert count == 1

        # Verify remaining rows
        results = await execute("SELECT * FROM test_execute_users")
        assert len(results) == 1
        assert results[0]["name"] == "Eve"

    async def test_execute_with_null_parameter(self, test_table):
        """Test query with NULL parameter."""
        # Insert with NULL email
        count = await execute(
            "INSERT INTO test_execute_users (name, email, age) VALUES ($1, $2, $3)",
            ["Frank", None, 35]
        )
        assert count == 1

        # Verify NULL was stored
        results = await execute(
            "SELECT * FROM test_execute_users WHERE email IS NULL"
        )
        assert len(results) == 1
        assert results[0]["name"] == "Frank"
        assert results[0]["email"] is None

    async def test_execute_with_multiple_types(self, test_table):
        """Test query with various parameter types."""
        # Create a more complex table
        await execute("""
            CREATE TABLE IF NOT EXISTS test_types (
                id SERIAL PRIMARY KEY,
                text_col TEXT,
                int_col INTEGER,
                bigint_col BIGINT,
                float_col REAL,
                double_col DOUBLE PRECISION,
                bool_col BOOLEAN
            )
        """)

        try:
            count = await execute(
                """
                INSERT INTO test_types (text_col, int_col, bigint_col, float_col, double_col, bool_col)
                VALUES ($1, $2, $3, $4, $5, $6)
                """,
                ["test", 42, 9999999999, 3.14, 2.718281828, True]
            )
            assert count == 1

            # Verify types
            results = await execute("SELECT * FROM test_types")
            assert len(results) == 1
            row = results[0]
            assert row["text_col"] == "test"
            assert row["int_col"] == 42
            assert row["bigint_col"] == 9999999999
            assert abs(row["float_col"] - 3.14) < 0.01
            assert abs(row["double_col"] - 2.718281828) < 0.000001
            assert row["bool_col"] is True

        finally:
            await execute("DROP TABLE IF EXISTS test_types")

    async def test_execute_aggregate_query(self, test_table):
        """Test aggregate query."""
        # Insert test data
        await execute(
            """
            INSERT INTO test_execute_users (name, age)
            VALUES ($1, $2), ($3, $4), ($5, $6)
            """,
            ["User1", 20, "User2", 30, "User3", 40]
        )

        # Execute aggregate query
        results = await execute(
            """
            SELECT
                COUNT(*) as total,
                AVG(age) as avg_age,
                MIN(age) as min_age,
                MAX(age) as max_age
            FROM test_execute_users
            WHERE age > $1
            """,
            [0]
        )

        assert len(results) == 1
        stats = results[0]
        assert stats["total"] == 3
        assert stats["avg_age"] == 30
        assert stats["min_age"] == 20
        assert stats["max_age"] == 40

    async def test_execute_ddl_create_index(self, test_table):
        """Test DDL operation (CREATE INDEX)."""
        result = await execute(
            "CREATE INDEX IF NOT EXISTS idx_test_execute_users_age ON test_execute_users(age)"
        )
        # DDL operations return None
        assert result is None

        # Verify index was created by querying system catalog
        results = await execute(
            """
            SELECT indexname FROM pg_indexes
            WHERE tablename = 'test_execute_users' AND indexname = $1
            """,
            ["idx_test_execute_users_age"]
        )
        assert len(results) == 1

    async def test_execute_with_query(self, test_table):
        """Test WITH (CTE) query."""
        # Insert test data
        await execute(
            """
            INSERT INTO test_execute_users (name, age)
            VALUES ($1, $2), ($3, $4), ($5, $6)
            """,
            ["Young1", 18, "Young2", 19, "Old1", 50]
        )

        # Execute CTE query
        results = await execute(
            """
            WITH young_users AS (
                SELECT * FROM test_execute_users WHERE age < $1
            )
            SELECT COUNT(*) as count FROM young_users
            """,
            [20]
        )

        assert len(results) == 1
        assert results[0]["count"] == 2

    async def test_execute_parameterized_prevents_injection(self, test_table):
        """Test that parameterized queries prevent SQL injection."""
        # Insert a row
        await execute(
            "INSERT INTO test_execute_users (name, age) VALUES ($1, $2)",
            ["Victim", 25]
        )

        # Attempt SQL injection via parameter
        malicious_input = "'; DROP TABLE test_execute_users; --"

        # This should safely search for the literal string, not execute the DROP
        results = await execute(
            "SELECT * FROM test_execute_users WHERE name = $1",
            [malicious_input]
        )

        # Should find no results (the literal string doesn't exist)
        assert len(results) == 0

        # Verify table still exists and has data
        all_results = await execute("SELECT * FROM test_execute_users")
        assert len(all_results) == 1
        assert all_results[0]["name"] == "Victim"
