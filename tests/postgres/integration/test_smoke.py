"""
Smoke tests to verify basic PostgreSQL connectivity and functionality.

These tests ensure that the test infrastructure is working correctly
before running more complex integration tests.
"""

import pytest
from data_bridge.postgres import execute


@pytest.mark.integration
@pytest.mark.asyncio
async def test_database_connection():
    """Test basic database connectivity with simple query."""
    result = await execute("SELECT 1 as num")
    assert len(result) == 1
    assert result[0]["num"] == 1


@pytest.mark.integration
@pytest.mark.asyncio
async def test_database_version():
    """Test database version query."""
    result = await execute("SELECT version()")
    assert len(result) == 1
    assert "PostgreSQL" in result[0]["version"]


@pytest.mark.integration
@pytest.mark.asyncio
async def test_create_and_query_table(test_table):
    """Test table creation and basic CRUD operations."""
    # Insert data
    await execute(
        f"INSERT INTO {test_table} (name, email, age) VALUES ($1, $2, $3)",
        ["Test User", "test@example.com", 25]
    )

    # Query data
    result = await execute(
        f"SELECT * FROM {test_table} WHERE name = $1",
        ["Test User"]
    )

    assert len(result) == 1
    assert result[0]["name"] == "Test User"
    assert result[0]["email"] == "test@example.com"
    assert result[0]["age"] == 25


@pytest.mark.integration
@pytest.mark.asyncio
async def test_sample_data_fixture(test_table, sample_data):
    """Test that sample_data fixture works correctly."""
    # Query all data
    result = await execute(f"SELECT * FROM {test_table} ORDER BY name")

    # Should have 3 users from sample_data
    assert len(result) == 3
    assert result[0]["name"] == "Alice"
    assert result[1]["name"] == "Bob"
    assert result[2]["name"] == "Charlie"

    # Verify sample_data matches
    assert len(sample_data) == 3


@pytest.mark.integration
@pytest.mark.asyncio
async def test_table_isolation_between_tests(test_table):
    """
    Test that tables are cleaned up between tests.

    This test should see an empty table, proving that the cleanup
    from previous tests worked.
    """
    result = await execute(f"SELECT COUNT(*) as count FROM {test_table}")
    assert result[0]["count"] == 0


@pytest.mark.integration
@pytest.mark.asyncio
async def test_parameterized_query(test_table):
    """Test parameterized queries work correctly."""
    # Insert multiple rows
    await execute(
        f"INSERT INTO {test_table} (name, age) VALUES ($1, $2), ($3, $4)",
        ["User1", 20, "User2", 30]
    )

    # Query with parameter
    result = await execute(
        f"SELECT * FROM {test_table} WHERE age > $1",
        [25]
    )

    assert len(result) == 1
    assert result[0]["name"] == "User2"


@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_support(test_table):
    """Test basic transaction support (implicitly via execute)."""
    # Each execute should be in its own transaction
    await execute(
        f"INSERT INTO {test_table} (name, age) VALUES ($1, $2)",
        ["TransactionTest", 40]
    )

    # Should be immediately visible in next query
    result = await execute(
        f"SELECT * FROM {test_table} WHERE name = $1",
        ["TransactionTest"]
    )

    assert len(result) == 1
    assert result[0]["age"] == 40
