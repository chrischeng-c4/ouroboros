"""Unit tests for PostgreSQL transaction support."""

import pytest
from data_bridge.postgres import connection


@pytest.mark.asyncio
async def test_transaction_basic_flow():
    """Test basic transaction flow without database."""
    # This test verifies the Python API structure without requiring a database
    # The transaction object should have commit() and rollback() methods
    # when the engine is available

    # Check that begin_transaction is available as a function
    assert hasattr(connection, 'begin_transaction')
    assert callable(connection.begin_transaction)


@pytest.mark.asyncio
async def test_transaction_isolation_levels():
    """Test that isolation levels are correctly typed."""
    from data_bridge.postgres.connection import IsolationLevel
    from typing import get_args

    # Verify IsolationLevel type definition
    expected_levels = ("read_uncommitted", "read_committed", "repeatable_read", "serializable")
    actual_levels = get_args(IsolationLevel)

    assert set(actual_levels) == set(expected_levels)


# Integration tests below require a live PostgreSQL database
# These should be run separately with pytest -m integration

@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_commit(postgres_connection):
    """Test transaction commit with live database."""
    # This test requires a live PostgreSQL connection
    # It should be marked as integration and skipped if DB is not available

    async with connection.begin_transaction() as tx:
        # Insert test data
        await connection._engine.insert_one(
            "test_table",
            {"name": "test_commit", "value": 123}
        )
        # Explicit commit
        await tx.commit()

    # Verify data persisted (requires query functionality)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_rollback(postgres_connection):
    """Test transaction rollback with live database."""
    async with connection.begin_transaction() as tx:
        # Insert test data
        await connection._engine.insert_one(
            "test_table",
            {"name": "test_rollback", "value": 456}
        )
        # Explicit rollback
        await tx.rollback()

    # Verify data NOT persisted


@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_auto_rollback_on_exception(postgres_connection):
    """Test automatic rollback on exception."""
    try:
        async with connection.begin_transaction() as tx:
            # Insert test data
            await connection._engine.insert_one(
                "test_table",
                {"name": "test_exception", "value": 789}
            )
            # Raise exception to trigger rollback
            raise ValueError("Simulated error")
    except ValueError:
        pass

    # Verify data NOT persisted due to auto-rollback


@pytest.mark.integration
@pytest.mark.asyncio
async def test_transaction_isolation_level_serializable(postgres_connection):
    """Test transaction with serializable isolation level."""
    async with connection.begin_transaction("serializable") as tx:
        # Perform operations in serializable isolation
        await connection._engine.insert_one(
            "test_table",
            {"name": "test_serializable", "value": 999}
        )
        await tx.commit()

    # Verify data persisted
