"""Unit tests for PostgreSQL transaction support."""

import pytest
from ouroboros.postgres import connection
from ouroboros.test import expect


@pytest.mark.asyncio
async def test_transaction_basic_flow():
    """Test basic transaction flow without database."""
    # This test verifies the Python API structure without requiring a database
    # The transaction object should have commit() and rollback() methods
    # when the engine is available

    # Check that begin_transaction is available as a function
    expect(hasattr(connection, 'begin_transaction')).to_be_true()
    expect(callable(connection.begin_transaction)).to_be_true()


@pytest.mark.asyncio
async def test_transaction_isolation_levels():
    """Test that isolation levels are correctly typed."""
    from ouroboros.postgres.connection import IsolationLevel
    from typing import get_args

    # Verify IsolationLevel type definition
    expected_levels = ("read_uncommitted", "read_committed", "repeatable_read", "serializable")
    actual_levels = get_args(IsolationLevel)

    expect(set(actual_levels)).to_equal(set(expected_levels))


# Integration tests below require a live PostgreSQL database
# These should be run separately with pytest -m integration

@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_commit(test_table):
    """Test transaction commit with live database."""
    async with connection.begin_transaction() as tx:
        # Insert test data
        await connection.insert_one(
            test_table,
            {"name": "test_commit", "email": "commit@test.com", "age": 20}
        )
        # Explicit commit
        await tx.commit()

    # Verify data persisted
    result = await connection.execute(f"SELECT * FROM {test_table} WHERE name = $1", ["test_commit"])
    expect(len(result)).to_equal(1)
    expect(result[0]["name"]).to_equal("test_commit")


@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_rollback(test_table):
    """Test transaction rollback with live database."""
    async with connection.begin_transaction() as tx:
        # Insert test data
        await connection.insert_one(
            test_table,
            {"name": "test_rollback", "email": "rollback@test.com", "age": 20}
        )
        # Explicit rollback
        await tx.rollback()

    # Verify data NOT persisted
    result = await connection.execute(f"SELECT * FROM {test_table} WHERE name = $1", ["test_rollback"])
    expect(len(result)).to_equal(0)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_auto_rollback_on_exception(test_table):
    """Test automatic rollback on exception."""
    try:
        async with connection.begin_transaction() as tx:
            # Insert test data
            await connection.insert_one(
                test_table,
                {"name": "test_exception", "email": "exception@test.com", "age": 20}
            )
            # Raise exception to trigger rollback
            raise ValueError("Simulated error")
    except ValueError:
        pass

    # Verify data NOT persisted due to auto-rollback
    result = await connection.execute(f"SELECT * FROM {test_table} WHERE name = $1", ["test_exception"])
    expect(len(result)).to_equal(0)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_isolation_level_serializable(test_table):
    """Test transaction with serializable isolation level."""
    async with connection.begin_transaction("serializable") as tx:
        # Perform operations in serializable isolation
        await connection.insert_one(
            test_table,
            {"name": "test_serializable", "email": "serial@test.com", "age": 20}
        )
        await tx.commit()

    # Verify data persisted
    result = await connection.execute(f"SELECT * FROM {test_table} WHERE name = $1", ["test_serializable"])
    expect(len(result)).to_equal(1)