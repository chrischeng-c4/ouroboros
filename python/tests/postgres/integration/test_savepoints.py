"""Integration tests for PostgreSQL savepoint functionality."""

import pytest
from ouroboros.postgres import connection, execute
from ouroboros.qc import expect


@pytest.fixture
async def test_table():
    """
    Create a simple test table for savepoint testing.

    Returns:
        str: The table name ('savepoint_test')
    """
    await execute("""
        CREATE TABLE IF NOT EXISTS savepoint_test (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            value INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)

    yield "savepoint_test"

    # Table will be cleaned up by cleanup_tables fixture in conftest.py


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_create(test_table):
    """Test creating a savepoint within a transaction."""
    async with connection.begin_transaction() as tx:
        # Insert initial data
        await connection.insert_one(test_table, {"name": "initial", "value": 1})

        # Create savepoint
        await tx._tx.savepoint("sp1")

        # Insert data after savepoint
        await connection.insert_one(test_table, {"name": "after_savepoint", "value": 2})

        # Release savepoint explicitly
        await tx._tx.release_savepoint("sp1")

        # Transaction commits
        await tx.commit()

    # Verify both records exist
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(2)
    expect(result[0]["name"]).to_equal("initial")
    expect(result[1]["name"]).to_equal("after_savepoint")


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_rollback(test_table):
    """Test rolling back to a savepoint undoes changes after it."""
    async with connection.begin_transaction() as tx:
        # Insert data before savepoint
        await connection.insert_one(test_table, {"name": "before", "value": 1})

        # Create savepoint
        await tx._tx.savepoint("sp1")

        # Insert data after savepoint
        await connection.insert_one(test_table, {"name": "after", "value": 2})

        # Verify data exists before rollback (within transaction)
        result = await tx.execute(f"SELECT * FROM {test_table} WHERE name = $1", ["after"])
        expect(len(result)).to_equal(1)

        # Rollback to savepoint
        await tx._tx.rollback_to_savepoint("sp1")

        # Transaction commits (keeps "before", removes "after")
        await tx.commit()

    # Verify only "before" exists
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(1)
    expect(result[0]["name"]).to_equal("before")

    # Verify "after" does not exist
    result = await execute(f"SELECT * FROM {test_table} WHERE name = $1", ["after"])
    expect(len(result)).to_equal(0)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_release(test_table):
    """Test releasing a savepoint keeps changes."""
    async with connection.begin_transaction() as tx:
        # Insert data before savepoint
        await connection.insert_one(test_table, {"name": "before", "value": 1})

        # Create savepoint
        await tx._tx.savepoint("sp1")

        # Insert data after savepoint
        await connection.insert_one(test_table, {"name": "after", "value": 2})

        # Release savepoint (keeps changes)
        await tx._tx.release_savepoint("sp1")

        # Transaction commits
        await tx.commit()

    # Verify both records exist
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(2)
    expect(result[0]["name"]).to_equal("before")
    expect(result[1]["name"]).to_equal("after")


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_nested(test_table):
    """Test multiple nested savepoints."""
    async with connection.begin_transaction() as tx:
        # Insert initial data
        await connection.insert_one(test_table, {"name": "initial", "value": 0})

        # Create first savepoint
        await tx._tx.savepoint("sp1")
        await connection.insert_one(test_table, {"name": "sp1_data", "value": 1})

        # Create second savepoint
        await tx._tx.savepoint("sp2")
        await connection.insert_one(test_table, {"name": "sp2_data", "value": 2})

        # Rollback to sp1 (removes sp2_data and sp1_data)
        await tx._tx.rollback_to_savepoint("sp1")

        # Commit transaction
        await tx.commit()

    # Verify only initial data exists (sp1_data and sp2_data rolled back)
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(1)
    expect(result[0]["name"]).to_equal("initial")

    # Verify sp1_data does not exist
    result = await execute(f"SELECT * FROM {test_table} WHERE name = $1", ["sp1_data"])
    expect(len(result)).to_equal(0)

    # Verify sp2_data does not exist
    result = await execute(f"SELECT * FROM {test_table} WHERE name = $1", ["sp2_data"])
    expect(len(result)).to_equal(0)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_commit_after_rollback(test_table):
    """Test transaction commits successfully after savepoint rollback."""
    async with connection.begin_transaction() as tx:
        # Insert initial data
        await connection.insert_one(test_table, {"name": "initial", "value": 1})

        # Create savepoint
        await tx._tx.savepoint("sp1")

        # Insert data that will be rolled back
        await connection.insert_one(test_table, {"name": "rollback_me", "value": 2})

        # Rollback savepoint
        await tx._tx.rollback_to_savepoint("sp1")

        # Insert more data after rollback
        await connection.insert_one(test_table, {"name": "after_rollback", "value": 3})

        # Commit transaction
        await tx.commit()

    # Verify initial and after_rollback exist, but not rollback_me
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(2)
    expect(result[0]["name"]).to_equal("initial")
    expect(result[1]["name"]).to_equal("after_rollback")

    # Verify rollback_me does not exist
    result = await execute(f"SELECT * FROM {test_table} WHERE name = $1", ["rollback_me"])
    expect(len(result)).to_equal(0)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_multiple_sequential(test_table):
    """Test creating multiple savepoints sequentially."""
    async with connection.begin_transaction() as tx:
        # Insert initial data
        await connection.insert_one(test_table, {"name": "initial", "value": 0})

        # First savepoint
        await tx._tx.savepoint("sp1")
        await connection.insert_one(test_table, {"name": "sp1_data", "value": 1})
        await tx._tx.release_savepoint("sp1")

        # Second savepoint (after first is released)
        await tx._tx.savepoint("sp2")
        await connection.insert_one(test_table, {"name": "sp2_data", "value": 2})
        await tx._tx.rollback_to_savepoint("sp2")

        # Third savepoint
        await tx._tx.savepoint("sp3")
        await connection.insert_one(test_table, {"name": "sp3_data", "value": 3})
        await tx._tx.release_savepoint("sp3")

        # Commit transaction
        await tx.commit()

    # Verify initial, sp1_data, and sp3_data exist (sp2_data rolled back)
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(3)
    expect(result[0]["name"]).to_equal("initial")
    expect(result[1]["name"]).to_equal("sp1_data")
    expect(result[2]["name"]).to_equal("sp3_data")

    # Verify sp2_data does not exist
    result = await execute(f"SELECT * FROM {test_table} WHERE name = $1", ["sp2_data"])
    expect(len(result)).to_equal(0)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_complex_workflow(test_table):
    """Test complex workflow with multiple savepoints and operations."""
    async with connection.begin_transaction() as tx:
        # Phase 1: Initial data
        await connection.insert_one(test_table, {"name": "user1", "value": 100})

        # Phase 2: Try risky operation with savepoint
        await tx._tx.savepoint("risky_operation")
        await connection.insert_one(test_table, {"name": "risky1", "value": 200})
        await connection.insert_one(test_table, {"name": "risky2", "value": 300})
        # Simulate failure - rollback
        await tx._tx.rollback_to_savepoint("risky_operation")

        # Phase 3: Another operation with savepoint
        await tx._tx.savepoint("safe_operation")
        await connection.insert_one(test_table, {"name": "safe1", "value": 400})
        await connection.insert_one(test_table, {"name": "safe2", "value": 500})
        await tx._tx.release_savepoint("safe_operation")  # Keep these changes

        # Phase 4: Final operation
        await connection.insert_one(test_table, {"name": "final", "value": 600})

        # Commit transaction
        await tx.commit()

    # Verify only user1, safe1, safe2, and final exist (risky1/risky2 rolled back)
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(4)
    expect(result[0]["name"]).to_equal("user1")
    expect(result[1]["name"]).to_equal("safe1")
    expect(result[2]["name"]).to_equal("safe2")
    expect(result[3]["name"]).to_equal("final")

    # Verify risky operations were rolled back
    result = await execute(f"SELECT * FROM {test_table} WHERE name LIKE 'risky%'")
    expect(len(result)).to_equal(0)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_error_handling(test_table):
    """Test that savepoint errors are properly handled."""
    async with connection.begin_transaction() as tx:
        # Create savepoint
        await tx._tx.savepoint("sp1")

        # Try to rollback to non-existent savepoint
        expect(lambda: await tx._tx.rollback_to_savepoint("non_existent")).to_raise(Exception)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_transaction_isolation(test_table):
    """Test that savepoint changes are isolated within transaction."""
    async with connection.begin_transaction() as tx:
        # Insert data
        await connection.insert_one(test_table, {"name": "data1", "value": 1})

        # Create savepoint
        await tx._tx.savepoint("sp1")
        await connection.insert_one(test_table, {"name": "data2", "value": 2})

        # Data should be visible within transaction
        result = await tx.execute(f"SELECT * FROM {test_table} WHERE name = $1", ["data2"])
        expect(len(result)).to_equal(1)

        # Rollback savepoint
        await tx._tx.rollback_to_savepoint("sp1")

        # Data2 should not be visible anymore
        result = await tx.execute(f"SELECT * FROM {test_table} WHERE name = $1", ["data2"])
        expect(len(result)).to_equal(0)

        # Commit transaction
        await tx.commit()

    # Verify only data1 exists
    result = await execute(f"SELECT * FROM {test_table}")
    expect(len(result)).to_equal(1)
    expect(result[0]["name"]).to_equal("data1")


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_release_after_release(test_table):
    """Test that releasing a savepoint twice causes an error."""
    async with connection.begin_transaction() as tx:
        # Create and release savepoint
        await tx._tx.savepoint("sp1")
        await tx._tx.release_savepoint("sp1")

        # Try to release again - should fail
        expect(lambda: await tx._tx.release_savepoint("sp1")).to_raise(Exception)


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_with_py_transaction_api(test_table):
    """Test using savepoint with the higher-level pg_transaction API."""
    from ouroboros.postgres.transactions import pg_transaction

    async with pg_transaction() as tx:
        # Insert initial data - must use transaction object methods
        await tx._tx.insert_one(test_table, {"name": "initial", "value": 1})

        # Create savepoint using the Savepoint wrapper
        sp = await tx.savepoint("sp1")

        # Insert data after savepoint
        await tx._tx.insert_one(test_table, {"name": "after", "value": 2})

        # Release savepoint
        await sp.release()

        # Commit transaction
        await tx.commit()

    # Verify both records exist
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(2)
    expect(result[0]["name"]).to_equal("initial")
    expect(result[1]["name"]).to_equal("after")


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_context_manager_with_pg_transaction(test_table):
    """Test using savepoint as async context manager with pg_transaction."""
    from ouroboros.postgres.transactions import pg_transaction

    async with pg_transaction() as tx:
        # Insert data before savepoint
        await tx._tx.insert_one(test_table, {"name": "before", "value": 1})

        # Use savepoint as context manager (auto-releases on success)
        async with await tx.savepoint("sp1"):
            await tx._tx.insert_one(test_table, {"name": "in_savepoint", "value": 2})

        # Transaction commits
        await tx.commit()

    # Verify both records exist (savepoint was auto-released)
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(2)
    expect(result[0]["name"]).to_equal("before")
    expect(result[1]["name"]).to_equal("in_savepoint")


@pytest.mark.integration
@pytest.mark.asyncio
async def test_savepoint_context_manager_exception_with_pg_transaction(test_table):
    """Test savepoint context manager auto-rollback on exception with pg_transaction."""
    from ouroboros.postgres.transactions import pg_transaction

    async with pg_transaction() as tx:
        # Insert data before savepoint
        await tx._tx.insert_one(test_table, {"name": "before", "value": 1})

        # Use savepoint as context manager with exception
        try:
            async with await tx.savepoint("sp1"):
                await tx._tx.insert_one(test_table, {"name": "exception_data", "value": 2})
                # Raise exception to trigger auto-rollback
                raise ValueError("Simulated error")
        except ValueError:
            pass  # Expected exception

        # Transaction commits (keeps "before", rolls back "exception_data")
        await tx.commit()

    # Verify only "before" exists
    result = await execute(f"SELECT * FROM {test_table} ORDER BY id")
    expect(len(result)).to_equal(1)
    expect(result[0]["name"]).to_equal("before")

    # Verify exception_data was rolled back
    result = await execute(f"SELECT * FROM {test_table} WHERE name = $1", ["exception_data"])
    expect(len(result)).to_equal(0)
