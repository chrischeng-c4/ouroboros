"""
Smoke tests to verify basic PostgreSQL connectivity and functionality.

These tests ensure that the test infrastructure is working correctly
before running more complex integration tests.
"""

import pytest
from ouroboros.postgres import execute
from ouroboros.test import expect


@pytest.mark.integration
@pytest.mark.asyncio
async def test_database_connection():
    """Test basic database connectivity with simple query."""
    result = await execute("SELECT 1 as num")
    expect(len(result)).to_equal(1)
    expect(result[0]["num"]).to_equal(1)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_database_version():
    """Test database version query."""
    result = await execute("SELECT version()")
    expect(len(result)).to_equal(1)
    expect("PostgreSQL" in result[0]["version"]).to_be_true()


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

    expect(len(result)).to_equal(1)
    expect(result[0]["name"]).to_equal("Test User")
    expect(result[0]["email"]).to_equal("test@example.com")
    expect(result[0]["age"]).to_equal(25)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_sample_data_fixture(test_table, sample_data):
    """Test that sample_data fixture works correctly."""
    # Query all data
    result = await execute(f"SELECT * FROM {test_table} ORDER BY name")

    # Should have 3 users from sample_data
    expect(len(result)).to_equal(3)
    expect(result[0]["name"]).to_equal("Alice")
    expect(result[1]["name"]).to_equal("Bob")
    expect(result[2]["name"]).to_equal("Charlie")

    # Verify sample_data matches
    expect(len(sample_data)).to_equal(3)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_table_isolation_between_tests(test_table):
    """
    Test that tables are cleaned up between tests.

    This test should see an empty table, proving that the cleanup
    from previous tests worked.
    """
    result = await execute(f"SELECT COUNT(*) as count FROM {test_table}")
    expect(result[0]["count"]).to_equal(0)


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

    expect(len(result)).to_equal(1)
    expect(result[0]["name"]).to_equal("User2")


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

    expect(len(result)).to_equal(1)
    expect(result[0]["age"]).to_equal(40)
