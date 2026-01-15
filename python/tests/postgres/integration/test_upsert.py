"""
Integration tests for PostgreSQL upsert operations.

Tests cover:
- Single document upsert (insert and update scenarios)
- Bulk upsert with mixed operations
- Composite unique constraints
- Selective column updates
- Parallel processing for large batches
- Error handling for invalid inputs
"""

import pytest
from ouroboros.postgres import init, close, execute, upsert_one, upsert_many
from ouroboros.qc import expect


@pytest.mark.integration
@pytest.mark.asyncio
class TestUpsertOne:
    """Test single document upsert operations."""

    async def test_upsert_one_insert_new(self):
        """
        Test upsert_one inserting a new row when no conflict exists.

        Verifies that upsert_one acts like a regular insert when the unique
        constraint is not violated.
        """
        # Create test table with unique email constraint
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Upsert new user (should insert)
        result = await upsert_one(
            "test_upsert_users",
            {"email": "alice@example.com", "name": "Alice", "age": 30},
            conflict_target="email"
        )

        # Verify insert happened
        expect(result["email"]).to_equal("alice@example.com")
        expect(result["name"]).to_equal("Alice")
        expect(result["age"]).to_equal(30)
        expect(result["id"]).not_to_be_none()

        # Verify data in database
        rows = await execute("SELECT * FROM test_upsert_users")
        expect(len(rows)).to_equal(1)
        expect(rows[0]["email"]).to_equal("alice@example.com")
        expect(rows[0]["name"]).to_equal("Alice")
        expect(rows[0]["age"]).to_equal(30)

    async def test_upsert_one_update_existing(self):
        """
        Test upsert_one updating an existing row on conflict.

        Verifies that when a unique constraint is violated, upsert_one
        updates the existing row instead of failing.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Insert initial user
        await execute(
            "INSERT INTO test_upsert_users (email, name, age) VALUES ($1, $2, $3)",
            ["bob@example.com", "Bob Original", 25]
        )

        # Get original ID
        original = await execute(
            "SELECT id FROM test_upsert_users WHERE email = $1",
            ["bob@example.com"]
        )
        original_id = original[0]["id"]

        # Upsert with same email (should update)
        result = await upsert_one(
            "test_upsert_users",
            {"email": "bob@example.com", "name": "Bob Updated", "age": 26},
            conflict_target="email"
        )

        # Verify update happened
        expect(result["email"]).to_equal("bob@example.com")
        expect(result["name"]).to_equal("Bob Updated")
        expect(result["age"]).to_equal(26)
        expect(result["id"]).to_equal(original_id)

        # Verify only one row exists (update, not insert)
        rows = await execute("SELECT * FROM test_upsert_users")
        expect(len(rows)).to_equal(1)
        expect(rows[0]["name"]).to_equal("Bob Updated")
        expect(rows[0]["age"]).to_equal(26)

    async def test_upsert_one_selective_update(self):
        """
        Test upsert_one with selective column updates on conflict.

        Verifies that update_columns parameter allows updating only
        specific columns while preserving others.
        """
        # Create test table with additional columns
        await execute("""
            CREATE TABLE test_upsert_selective (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER,
                status TEXT DEFAULT 'active',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # Insert initial user
        await execute(
            "INSERT INTO test_upsert_selective (email, name, age, status) VALUES ($1, $2, $3, $4)",
            ["charlie@example.com", "Charlie", 35, "premium"]
        )

        # Get original values
        original = await execute(
            "SELECT id, status, created_at FROM test_upsert_selective WHERE email = $1",
            ["charlie@example.com"]
        )
        original_id = original[0]["id"]
        original_status = original[0]["status"]
        original_created_at = original[0]["created_at"]

        # Upsert with selective update (only update name and age)
        result = await upsert_one(
            "test_upsert_selective",
            {"email": "charlie@example.com", "name": "Charlie Updated", "age": 36, "status": "inactive"},
            conflict_target="email",
            update_columns=["name", "age"]  # Don't update status
        )

        # Verify selective update
        expect(result["email"]).to_equal("charlie@example.com")
        expect(result["name"]).to_equal("Charlie Updated")
        expect(result["age"]).to_equal(36)
        expect(result["status"]).to_equal(original_status)
        expect(result["id"]).to_equal(original_id)

        # Verify in database
        rows = await execute("SELECT * FROM test_upsert_selective WHERE email = $1", ["charlie@example.com"])
        expect(len(rows)).to_equal(1)
        expect(rows[0]["name"]).to_equal("Charlie Updated")
        expect(rows[0]["age"]).to_equal(36)
        expect(rows[0]["status"]).to_equal("premium")

    async def test_upsert_one_composite_key(self):
        """
        Test upsert_one with composite unique constraint.

        Verifies that upsert works correctly when the unique constraint
        involves multiple columns (e.g., tenant_id + user_id).
        """
        # Create test table with composite unique constraint
        await execute("""
            CREATE TABLE test_upsert_composite (
                id SERIAL PRIMARY KEY,
                tenant_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                name TEXT,
                metadata TEXT,
                UNIQUE(tenant_id, user_id)
            )
        """)

        # Insert initial row
        await execute(
            "INSERT INTO test_upsert_composite (tenant_id, user_id, name, metadata) VALUES ($1, $2, $3, $4)",
            [1, 100, "User 100", "initial"]
        )

        # Upsert with same composite key (should update)
        result = await upsert_one(
            "test_upsert_composite",
            {"tenant_id": 1, "user_id": 100, "name": "User 100 Updated", "metadata": "updated"},
            conflict_target=["tenant_id", "user_id"]  # Composite key
        )

        # Verify update happened
        expect(result["tenant_id"]).to_equal(1)
        expect(result["user_id"]).to_equal(100)
        expect(result["name"]).to_equal("User 100 Updated")
        expect(result["metadata"]).to_equal("updated")

        # Verify only one row exists
        rows = await execute("SELECT * FROM test_upsert_composite")
        expect(len(rows)).to_equal(1)
        expect(rows[0]["name"]).to_equal("User 100 Updated")

        # Upsert with different composite key (should insert)
        result2 = await upsert_one(
            "test_upsert_composite",
            {"tenant_id": 1, "user_id": 101, "name": "User 101", "metadata": "new"},
            conflict_target=["tenant_id", "user_id"]
        )

        # Verify insert happened
        expect(result2["tenant_id"]).to_equal(1)
        expect(result2["user_id"]).to_equal(101)
        expect(result2["name"]).to_equal("User 101")

        # Verify two rows exist now
        rows = await execute("SELECT * FROM test_upsert_composite ORDER BY user_id")
        expect(len(rows)).to_equal(2)
        expect(rows[0]["user_id"]).to_equal(100)
        expect(rows[1]["user_id"]).to_equal(101)


@pytest.mark.integration
@pytest.mark.asyncio
class TestUpsertMany:
    """Test bulk upsert operations."""

    async def test_upsert_many_insert_all_new(self):
        """
        Test upsert_many inserting all new rows (no conflicts).

        Verifies that upsert_many can efficiently insert multiple rows
        when none of them conflict with existing data.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Upsert multiple new users
        users = [
            {"email": "alice@example.com", "name": "Alice", "age": 30},
            {"email": "bob@example.com", "name": "Bob", "age": 25},
            {"email": "charlie@example.com", "name": "Charlie", "age": 35},
        ]

        results = await upsert_many(
            "test_upsert_users",
            users,
            conflict_target="email"
        )

        # Verify all inserts happened
        expect(len(results)).to_equal(3)
        expect(all(r["id"] is not None for r in results)).to_be_true()
        expect({r["email"] for r in results}).to_equal({"alice@example.com", "bob@example.com", "charlie@example.com"})

        # Verify data in database
        rows = await execute("SELECT * FROM test_upsert_users ORDER BY email")
        expect(len(rows)).to_equal(3)
        expect(rows[0]["email"]).to_equal("alice@example.com")
        expect(rows[1]["email"]).to_equal("bob@example.com")
        expect(rows[2]["email"]).to_equal("charlie@example.com")

    async def test_upsert_many_update_all_existing(self):
        """
        Test upsert_many updating all existing rows (all conflicts).

        Verifies that upsert_many correctly updates all rows when every
        document conflicts with an existing row.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Insert initial users
        initial_users = [
            {"email": "alice@example.com", "name": "Alice Original", "age": 30},
            {"email": "bob@example.com", "name": "Bob Original", "age": 25},
            {"email": "charlie@example.com", "name": "Charlie Original", "age": 35},
        ]

        for user in initial_users:
            await execute(
                "INSERT INTO test_upsert_users (email, name, age) VALUES ($1, $2, $3)",
                [user["email"], user["name"], user["age"]]
            )

        # Get original IDs
        original_rows = await execute("SELECT id, email FROM test_upsert_users ORDER BY email")
        original_ids = {row["email"]: row["id"] for row in original_rows}

        # Upsert with same emails (should all update)
        updated_users = [
            {"email": "alice@example.com", "name": "Alice Updated", "age": 31},
            {"email": "bob@example.com", "name": "Bob Updated", "age": 26},
            {"email": "charlie@example.com", "name": "Charlie Updated", "age": 36},
        ]

        results = await upsert_many(
            "test_upsert_users",
            updated_users,
            conflict_target="email"
        )

        # Verify all updates happened
        expect(len(results)).to_equal(3)
        for result in results:
            expect(result["name"].endswith("Updated")).to_be_true()
            expect(result["id"]).to_equal(original_ids[result["email"]])

        # Verify still only 3 rows (updates, not inserts)
        rows = await execute("SELECT * FROM test_upsert_users ORDER BY email")
        expect(len(rows)).to_equal(3)
        expect(all(row["name"].endswith("Updated") for row in rows)).to_be_true()

    async def test_upsert_many_mixed(self):
        """
        Test upsert_many with mixed inserts and updates.

        Verifies that upsert_many correctly handles a batch where some
        documents conflict (update) and others don't (insert).
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Insert some initial users
        await execute(
            "INSERT INTO test_upsert_users (email, name, age) VALUES ($1, $2, $3), ($4, $5, $6)",
            ["alice@example.com", "Alice Original", 30, "bob@example.com", "Bob Original", 25]
        )

        # Upsert batch with mix of existing and new
        mixed_users = [
            {"email": "alice@example.com", "name": "Alice Updated", "age": 31},  # Update
            {"email": "bob@example.com", "name": "Bob Updated", "age": 26},      # Update
            {"email": "charlie@example.com", "name": "Charlie New", "age": 35},  # Insert
            {"email": "diana@example.com", "name": "Diana New", "age": 28},      # Insert
        ]

        results = await upsert_many(
            "test_upsert_users",
            mixed_users,
            conflict_target="email"
        )

        # Verify all operations succeeded
        expect(len(results)).to_equal(4)

        # Verify data in database
        rows = await execute("SELECT * FROM test_upsert_users ORDER BY email")
        expect(len(rows)).to_equal(4)

        # Check updated rows
        alice = next(r for r in rows if r["email"] == "alice@example.com")
        expect(alice["name"]).to_equal("Alice Updated")
        expect(alice["age"]).to_equal(31)

        bob = next(r for r in rows if r["email"] == "bob@example.com")
        expect(bob["name"]).to_equal("Bob Updated")
        expect(bob["age"]).to_equal(26)

        # Check inserted rows
        charlie = next(r for r in rows if r["email"] == "charlie@example.com")
        expect(charlie["name"]).to_equal("Charlie New")
        expect(charlie["age"]).to_equal(35)

        diana = next(r for r in rows if r["email"] == "diana@example.com")
        expect(diana["name"]).to_equal("Diana New")
        expect(diana["age"]).to_equal(28)

    async def test_upsert_many_parallel_threshold(self):
        """
        Test upsert_many with large batch (50+ rows) to trigger parallel processing.

        Verifies that bulk upserts work correctly with large batches and that
        parallel processing threshold (≥50 documents) is handled properly.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Generate large batch (100 users)
        large_batch = [
            {"email": f"user{i}@example.com", "name": f"User {i}", "age": 20 + (i % 50)}
            for i in range(100)
        ]

        # Upsert large batch
        results = await upsert_many(
            "test_upsert_users",
            large_batch,
            conflict_target="email"
        )

        # Verify all inserts happened
        expect(len(results)).to_equal(100)
        expect(all(r["id"] is not None for r in results)).to_be_true()

        # Verify data in database
        rows = await execute("SELECT COUNT(*) as count FROM test_upsert_users")
        expect(rows[0]["count"]).to_equal(100)

        # Update half of them (mixed batch with 50 updates + 50 new inserts)
        update_batch = [
            {"email": f"user{i}@example.com", "name": f"User {i} Updated", "age": 25 + (i % 50)}
            for i in range(50)
        ]
        insert_batch = [
            {"email": f"newuser{i}@example.com", "name": f"New User {i}", "age": 30 + (i % 50)}
            for i in range(50)
        ]
        mixed_batch = update_batch + insert_batch

        # Upsert mixed large batch
        results2 = await upsert_many(
            "test_upsert_users",
            mixed_batch,
            conflict_target="email"
        )

        # Verify all operations succeeded
        expect(len(results2)).to_equal(100)

        # Verify total count (100 original + 50 new = 150)
        rows = await execute("SELECT COUNT(*) as count FROM test_upsert_users")
        expect(rows[0]["count"]).to_equal(150)

        # Verify updates happened
        updated_rows = await execute(
            "SELECT * FROM test_upsert_users WHERE name LIKE '%Updated%'"
        )
        expect(len(updated_rows)).to_equal(50)

    async def test_upsert_many_selective_update(self):
        """
        Test upsert_many with selective column updates on conflicts.

        Verifies that update_columns parameter works correctly with bulk
        operations, preserving specified columns on update.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_many_selective (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER,
                status TEXT DEFAULT 'active',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)

        # Insert initial users
        initial_users = [
            {"email": "alice@example.com", "name": "Alice", "age": 30, "status": "premium"},
            {"email": "bob@example.com", "name": "Bob", "age": 25, "status": "premium"},
        ]

        for user in initial_users:
            await execute(
                "INSERT INTO test_upsert_many_selective (email, name, age, status) VALUES ($1, $2, $3, $4)",
                [user["email"], user["name"], user["age"], user["status"]]
            )

        # Upsert with selective update (only update age, preserve status)
        update_users = [
            {"email": "alice@example.com", "name": "Alice Updated", "age": 31, "status": "inactive"},
            {"email": "bob@example.com", "name": "Bob Updated", "age": 26, "status": "inactive"},
        ]

        results = await upsert_many(
            "test_upsert_many_selective",
            update_users,
            conflict_target="email",
            update_columns=["name", "age"]  # Don't update status
        )

        # Verify selective updates
        expect(len(results)).to_equal(2)
        for result in results:
            expect(result["name"].endswith("Updated")).to_be_true()
            expect(result["status"]).to_equal("premium")

        # Verify in database
        rows = await execute("SELECT * FROM test_upsert_many_selective ORDER BY email")
        expect(len(rows)).to_equal(2)
        expect(all(row["status"] == "premium" for row in rows)).to_be_true()
        expect(all(row["name"].endswith("Updated") for row in rows)).to_be_true()


@pytest.mark.integration
@pytest.mark.asyncio
class TestUpsertErrors:
    """Test error handling for upsert operations."""

    async def test_upsert_invalid_conflict_column(self):
        """
        Test upsert_one with non-existent conflict column.

        Verifies that appropriate error is raised when conflict_target
        references a column that doesn't exist.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Try to upsert with invalid conflict column
        exc_info = expect(lambda: await upsert_one().to_raise(Exception)
                "test_upsert_users",
                {"email": "alice@example.com", "name": "Alice", "age": 30},
                conflict_target="nonexistent_column"  # Invalid column
            )

        # Verify error message indicates the problem
        error_msg = str(exc_info.value).lower()
        expect("column" in error_msg or "constraint" in error_msg or "conflict" in error_msg).to_be_true()

    async def test_upsert_empty_conflict_target(self):
        """
        Test upsert_one with empty conflict_target.

        Verifies that appropriate error is raised when conflict_target
        is an empty list or empty string.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Try to upsert with empty conflict target (empty list)
        exc_info = expect(lambda: await upsert_one().to_raise(Exception)
                "test_upsert_users",
                {"email": "alice@example.com", "name": "Alice", "age": 30},
                conflict_target=[]  # Empty list
            )

        # Error should be raised
        expect(exc_info.value).not_to_be_none()

    async def test_upsert_many_empty_documents(self):
        """
        Test upsert_many with empty document list.

        Verifies that upsert_many handles empty input gracefully.
        """
        # Create test table
        await execute("""
            CREATE TABLE test_upsert_users (
                id SERIAL PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT,
                age INTEGER
            )
        """)

        # Upsert empty list
        results = await upsert_many(
            "test_upsert_users",
            [],  # Empty list
            conflict_target="email"
        )

        # Should return empty list
        expect(results).to_equal([])

        # Verify no rows in database
        rows = await execute("SELECT * FROM test_upsert_users")
        expect(len(rows)).to_equal(0)

    async def test_upsert_table_not_exists(self):
        """
        Test upsert operations on non-existent table.

        Verifies that appropriate error is raised when table doesn't exist.
        """
        # Try to upsert to non-existent table
        exc_info = expect(lambda: await upsert_one().to_raise(Exception)
                "nonexistent_table",
                {"email": "alice@example.com", "name": "Alice"},
                conflict_target="email"
            )

        # Verify error indicates table doesn't exist
        error_msg = str(exc_info.value).lower()
        expect("table" in error_msg or "relation" in error_msg or "not" in error_msg).to_be_true()
