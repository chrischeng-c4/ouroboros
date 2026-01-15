"""
Integration tests for raw SQL execution.

These tests require a running PostgreSQL instance.

Set POSTGRES_URI environment variable to override default:
    export POSTGRES_URI="postgresql://user:pass@localhost:5432/ouroboros_test"

Run with:
    bash scripts/run_integration_tests.sh
    # or
    POSTGRES_URI="postgresql://rstn:rstn@localhost:5432/ouroboros_test" \
        pytest tests/postgres/integration/test_execute_integration.py -v -m integration
"""

import pytest
from ouroboros.postgres import execute
from ouroboros.test import expect


# Note: Connection setup is handled by conftest.py fixtures


@pytest.fixture
async def test_table():
    """Create a test table for execute integration tests."""
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

    # Table will be cleaned up by cleanup_tables fixture from conftest.py


@pytest.mark.asyncio
@pytest.mark.integration
class TestExecuteIntegration:
    """Integration tests for execute function with real PostgreSQL."""

    async def test_execute_select_empty(self, test_table):
        """Test SELECT on empty table returns empty list."""
        results = await execute("SELECT * FROM test_execute_users")
        expect(results).to_equal([])

    async def test_execute_insert_and_select(self, test_table):
        """Test INSERT followed by SELECT."""
        # Insert a row
        count = await execute(
            "INSERT INTO test_execute_users (name, email, age) VALUES ($1, $2, $3)",
            ["Alice", "alice@example.com", 30]
        )
        expect(count).to_equal(1)

        # Select all rows
        results = await execute("SELECT * FROM test_execute_users")
        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Alice")
        expect(results[0]["email"]).to_equal("alice@example.com")
        expect(results[0]["age"]).to_equal(30)

    async def test_execute_insert_with_returning(self, test_table):
        """Test INSERT with RETURNING clause."""
        results = await execute(
            "INSERT INTO test_execute_users (name, email, age) VALUES ($1, $2, $3) RETURNING id, name",
            ["Bob", "bob@example.com", 25]
        )
        # INSERT with RETURNING is treated as SELECT
        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Bob")
        expect("id" in results[0]).to_be_true()

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
        expect(count).to_equal(1)

        # Verify update
        results = await execute(
            "SELECT age FROM test_execute_users WHERE name = $1",
            ["Charlie"]
        )
        expect(results[0]["age"]).to_equal(21)

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
        expect(count).to_equal(1)

        # Verify remaining rows
        results = await execute("SELECT * FROM test_execute_users")
        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Eve")

    async def test_execute_with_null_parameter(self, test_table):
        """Test query with NULL parameter."""
        # Insert with NULL email
        count = await execute(
            "INSERT INTO test_execute_users (name, email, age) VALUES ($1, $2, $3)",
            ["Frank", None, 35]
        )
        expect(count).to_equal(1)

        # Verify NULL was stored
        results = await execute(
            "SELECT * FROM test_execute_users WHERE email IS NULL"
        )
        expect(len(results)).to_equal(1)
        expect(results[0]["name"]).to_equal("Frank")
        expect(results[0]["email"]).to_be_none()

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
            expect(count).to_equal(1)

            # Verify types
            results = await execute("SELECT * FROM test_types")
            expect(len(results)).to_equal(1)
            row = results[0]
            expect(row["text_col"]).to_equal("test")
            expect(row["int_col"]).to_equal(42)
            expect(row["bigint_col"]).to_equal(9999999999)
            expect(abs(row["float_col"] - 3.14) < 0.01).to_be_true()
            expect(abs(row["double_col"] - 2.718281828) < 0.000001).to_be_true()
            expect(row["bool_col"]).to_equal(True)

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

        expect(len(results)).to_equal(1)
        stats = results[0]
        expect(stats["total"]).to_equal(3)
        expect(stats["avg_age"]).to_equal(30)
        expect(stats["min_age"]).to_equal(20)
        expect(stats["max_age"]).to_equal(40)

    async def test_execute_ddl_create_index(self, test_table):
        """Test DDL operation (CREATE INDEX)."""
        result = await execute(
            "CREATE INDEX IF NOT EXISTS idx_test_execute_users_age ON test_execute_users(age)"
        )
        # DDL operations return None
        expect(result).to_be_none()

        # Verify index was created by querying system catalog
        results = await execute(
            """
            SELECT indexname FROM pg_indexes
            WHERE tablename = 'test_execute_users' AND indexname = $1
            """,
            ["idx_test_execute_users_age"]
        )
        expect(len(results)).to_equal(1)

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

        expect(len(results)).to_equal(1)
        expect(results[0]["count"]).to_equal(2)

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
        expect(len(results)).to_equal(0)

        # Verify table still exists and has data
        all_results = await execute("SELECT * FROM test_execute_users")
        expect(len(all_results)).to_equal(1)
        expect(all_results[0]["name"]).to_equal("Victim")
