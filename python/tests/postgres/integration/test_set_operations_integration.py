"""
Integration tests for SQL set operations (UNION/INTERSECT/EXCEPT).

Tests set operation functionality with real PostgreSQL database using raw SQL.
"""
import pytest
from ouroboros.postgres import execute, insert_one


@pytest.fixture
async def customers_2023():
    """Create and populate customers_2023 table."""
    # Create table
    await execute(
        """
        CREATE TABLE IF NOT EXISTS customers_2023 (
            id INTEGER PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            email VARCHAR(100) NOT NULL
        )
        """
    )

    # Insert test data (some will overlap with 2024)
    test_data = [
        {"id": 1, "name": "Alice Johnson", "email": "alice@example.com"},
        {"id": 2, "name": "Bob Smith", "email": "bob@example.com"},
        {"id": 3, "name": "Charlie Brown", "email": "charlie@example.com"},
        {"id": 4, "name": "David Wilson", "email": "david@example.com"},
        {"id": 5, "name": "Eve Martinez", "email": "eve@example.com"},
    ]

    for data in test_data:
        await insert_one("customers_2023", data)

    yield

    # Cleanup
    await execute("DROP TABLE IF EXISTS customers_2023")


@pytest.fixture
async def customers_2024():
    """Create and populate customers_2024 table."""
    # Create table
    await execute(
        """
        CREATE TABLE IF NOT EXISTS customers_2024 (
            id INTEGER PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            email VARCHAR(100) NOT NULL
        )
        """
    )

    # Insert test data (overlap with 2023: Alice, Bob, Charlie)
    test_data = [
        {"id": 1, "name": "Alice Johnson", "email": "alice@example.com"},    # Same as 2023
        {"id": 2, "name": "Bob Smith", "email": "bob@example.com"},          # Same as 2023
        {"id": 3, "name": "Charlie Brown", "email": "charlie@example.com"},  # Same as 2023
        {"id": 6, "name": "Frank Davis", "email": "frank@example.com"},      # New in 2024
        {"id": 7, "name": "Grace Lee", "email": "grace@example.com"},        # New in 2024
    ]

    for data in test_data:
        await insert_one("customers_2024", data)

    yield

    # Cleanup
    await execute("DROP TABLE IF EXISTS customers_2024")


@pytest.mark.asyncio
class TestBasicSetOperations:
    """Test basic set operations."""

    async def test_union_combines_and_removes_duplicates(self, customers_2023, customers_2024):
        """Test UNION - combine two tables with same columns, removing duplicates."""
        # UNION should combine both tables and remove duplicates
        results = await execute("""
            SELECT * FROM customers_2023
            UNION
            SELECT * FROM customers_2024
        """)

        # 2023 has 5 customers, 2024 has 5 customers
        # But 3 are duplicates (Alice, Bob, Charlie)
        # So total unique = 5 + 5 - 3 = 7
        assert len(results) == 7

        # Verify we have all unique emails
        emails = [r["email"] for r in results]
        assert len(set(emails)) == 7

        # Verify both old and new customers are present
        assert "alice@example.com" in emails
        assert "eve@example.com" in emails    # Only in 2023
        assert "frank@example.com" in emails  # Only in 2024

    async def test_union_all_includes_duplicates(self, customers_2023, customers_2024):
        """Test UNION ALL - include duplicates."""
        # UNION ALL should include all rows, even duplicates
        results = await execute("""
            SELECT * FROM customers_2023
            UNION ALL
            SELECT * FROM customers_2024
        """)

        # 2023 has 5 customers, 2024 has 5 customers
        # UNION ALL keeps all 10 rows
        assert len(results) == 10

        # Count occurrences of Alice (should appear twice)
        emails = [r["email"] for r in results]
        alice_count = sum(1 for email in emails if email == "alice@example.com")
        assert alice_count == 2

    async def test_intersect_finds_common_rows(self, customers_2023, customers_2024):
        """Test INTERSECT - find rows present in both tables."""
        # INTERSECT should return only customers in both years
        results = await execute("""
            SELECT * FROM customers_2023
            INTERSECT
            SELECT * FROM customers_2024
        """)

        # Only Alice, Bob, and Charlie are in both years
        assert len(results) == 3

        emails = [r["email"] for r in results]
        assert "alice@example.com" in emails
        assert "bob@example.com" in emails
        assert "charlie@example.com" in emails

        # Eve (only 2023) and Frank (only 2024) should not be in results
        assert "eve@example.com" not in emails
        assert "frank@example.com" not in emails

    async def test_except_finds_rows_in_first_only(self, customers_2023, customers_2024):
        """Test EXCEPT - find rows in first table but not second."""
        # EXCEPT should return customers only in 2023
        results = await execute("""
            SELECT * FROM customers_2023
            EXCEPT
            SELECT * FROM customers_2024
        """)

        # Only David and Eve are exclusive to 2023
        assert len(results) == 2

        emails = [r["email"] for r in results]
        assert "david@example.com" in emails
        assert "eve@example.com" in emails

        # Alice, Bob, Charlie (in both) should not be present
        assert "alice@example.com" not in emails
        assert "bob@example.com" not in emails

        # Frank (only in 2024) should not be present
        assert "frank@example.com" not in emails


@pytest.mark.asyncio
class TestSetOperationsWithFilters:
    """Test set operations combined with WHERE conditions."""

    async def test_union_with_where_conditions(self, customers_2023, customers_2024):
        """Test UNION with WHERE conditions on each part."""
        # Find customers with id < 3 in 2023 OR id > 5 in 2024
        results = await execute("""
            SELECT * FROM customers_2023 WHERE id < 3
            UNION
            SELECT * FROM customers_2024 WHERE id > 5
        """)

        # Should get Alice, Bob, Frank, Grace
        assert len(results) == 4

        emails = [r["email"] for r in results]
        assert "alice@example.com" in emails
        assert "bob@example.com" in emails
        assert "frank@example.com" in emails
        assert "grace@example.com" in emails

    async def test_intersect_with_filters(self, customers_2023, customers_2024):
        """Test INTERSECT with WHERE conditions."""
        # Find customers with id <= 3 in both years
        results = await execute("""
            SELECT * FROM customers_2023 WHERE id <= 3
            INTERSECT
            SELECT * FROM customers_2024 WHERE id <= 3
        """)

        # Alice, Bob, Charlie are in both tables with id <= 3
        assert len(results) == 3

    async def test_except_with_filters(self, customers_2023, customers_2024):
        """Test EXCEPT with WHERE conditions."""
        # Find 2023 customers with id >= 3 but not in 2024 with id >= 3
        results = await execute("""
            SELECT * FROM customers_2023 WHERE id >= 3
            EXCEPT
            SELECT * FROM customers_2024 WHERE id >= 3
        """)

        # David and Eve are in 2023 (id >= 3) but not in 2024 (id >= 3)
        # Charlie is in both so excluded
        assert len(results) == 2

        emails = [r["email"] for r in results]
        assert "david@example.com" in emails
        assert "eve@example.com" in emails


@pytest.mark.asyncio
class TestChainedSetOperations:
    """Test multiple set operations chained together."""

    async def test_multiple_unions_chained(self, customers_2023, customers_2024):
        """Test chaining multiple UNION operations."""
        # Chain multiple UNIONs
        results = await execute("""
            SELECT * FROM customers_2023 WHERE id <= 2
            UNION
            SELECT * FROM customers_2023 WHERE id >= 4
            UNION
            SELECT * FROM customers_2024 WHERE id >= 6
        """)

        # Alice, Bob, David, Eve, Frank, Grace = 6 unique
        assert len(results) == 6

        emails = [r["email"] for r in results]
        assert "alice@example.com" in emails
        assert "bob@example.com" in emails
        assert "david@example.com" in emails
        assert "eve@example.com" in emails
        assert "frank@example.com" in emails
        assert "grace@example.com" in emails

    async def test_union_then_intersect(self, customers_2023, customers_2024):
        """Test combining UNION with INTERSECT using CTE."""
        # Using CTE to combine UNION and INTERSECT
        # This finds customers that are in (2023 OR early_2024) AND all_2024
        results = await execute("""
            WITH combined AS (
                SELECT * FROM customers_2023
                UNION
                SELECT * FROM customers_2024 WHERE id <= 3
            )
            SELECT * FROM combined
            INTERSECT
            SELECT * FROM customers_2024
        """)

        # Customers in both 2023 and 2024_full = Alice, Bob, Charlie
        assert len(results) == 3


@pytest.mark.asyncio
class TestSetOperationsWithOrderBy:
    """Test set operations with ORDER BY clause."""

    async def test_union_with_order_by(self, customers_2023, customers_2024):
        """Test set operations with ORDER BY."""
        # UNION with ORDER BY name descending
        results = await execute("""
            SELECT * FROM customers_2023
            UNION
            SELECT * FROM customers_2024
            ORDER BY name DESC
        """)

        # Verify total count
        assert len(results) == 7

        # Verify ordering (descending by name)
        names = [r["name"] for r in results]
        assert names == sorted(names, reverse=True)

        # First should be the last alphabetically
        assert results[0]["name"] in ["Grace Lee", "Frank Davis"]  # G or F comes last

    async def test_intersect_with_order_by(self, customers_2023, customers_2024):
        """Test INTERSECT with ORDER BY."""
        # INTERSECT with ORDER BY id ascending
        results = await execute("""
            SELECT * FROM customers_2023
            INTERSECT
            SELECT * FROM customers_2024
            ORDER BY id ASC
        """)

        assert len(results) == 3

        # Verify ordering by id
        ids = [r["id"] for r in results]
        assert ids == sorted(ids)
        assert ids == [1, 2, 3]  # Alice, Bob, Charlie


@pytest.mark.asyncio
class TestSetOperationsWithLimit:
    """Test set operations with LIMIT clause."""

    async def test_union_with_limit(self, customers_2023, customers_2024):
        """Test UNION with LIMIT."""
        # UNION with LIMIT 3
        results = await execute("""
            SELECT * FROM customers_2023
            UNION
            SELECT * FROM customers_2024
            LIMIT 3
        """)

        # Should return only first 3 results
        assert len(results) == 3

    async def test_union_with_offset_and_limit(self, customers_2023, customers_2024):
        """Test UNION with OFFSET and LIMIT."""
        # UNION with ORDER BY, OFFSET, and LIMIT for pagination
        results = await execute("""
            SELECT * FROM customers_2023
            UNION
            SELECT * FROM customers_2024
            ORDER BY id
            OFFSET 2
            LIMIT 3
        """)

        # Skip first 2, return next 3
        assert len(results) == 3

        # Should be ids 3, 4, 5
        ids = [r["id"] for r in results]
        assert min(ids) >= 3  # At least id 3


@pytest.mark.asyncio
class TestSetOperationsWithColumns:
    """Test set operations must have matching column structure."""

    async def test_union_compatible_columns(self, customers_2023, customers_2024):
        """Test UNION works when selecting same columns."""
        # Select specific columns from both tables
        results = await execute("""
            SELECT id, name FROM customers_2023
            UNION
            SELECT id, name FROM customers_2024
        """)

        # Should work and return all unique customers
        assert len(results) == 7

        # Verify each result has the selected columns
        for result in results:
            assert "id" in result
            assert "name" in result


@pytest.mark.asyncio
class TestIntersectAll:
    """Test INTERSECT ALL (keeps duplicates)."""

    async def test_intersect_all(self, customers_2023, customers_2024):
        """Test INTERSECT ALL keeps all matching rows."""
        # INTERSECT ALL should return common rows, keeping duplicates if any
        results = await execute("""
            SELECT * FROM customers_2023
            INTERSECT ALL
            SELECT * FROM customers_2024
        """)

        # Alice, Bob, Charlie are in both (no actual duplicates in our data)
        assert len(results) == 3


@pytest.mark.asyncio
class TestExceptAll:
    """Test EXCEPT ALL (keeps duplicates)."""

    async def test_except_all(self, customers_2023, customers_2024):
        """Test EXCEPT ALL keeps all non-matching rows."""
        # EXCEPT ALL should return rows only in 2023
        results = await execute("""
            SELECT * FROM customers_2023
            EXCEPT ALL
            SELECT * FROM customers_2024
        """)

        # David and Eve are only in 2023
        assert len(results) == 2

        emails = [r["email"] for r in results]
        assert "david@example.com" in emails
        assert "eve@example.com" in emails


@pytest.mark.asyncio
class TestSetOperationsEdgeCases:
    """Test edge cases and special scenarios."""

    async def test_union_empty_result(self, customers_2023, customers_2024):
        """Test UNION when one query returns no results."""
        # Query that returns no results
        results = await execute("""
            SELECT * FROM customers_2023 WHERE id > 100
            UNION
            SELECT * FROM customers_2024
        """)

        # Should return only results from 2024
        assert len(results) == 5

    async def test_intersect_no_overlap(self, customers_2023, customers_2024):
        """Test INTERSECT when there's no overlap."""
        # Find customers only in 2023 vs only in 2024
        results = await execute("""
            SELECT * FROM customers_2023 WHERE id >= 4
            INTERSECT
            SELECT * FROM customers_2024 WHERE id >= 6
        """)

        # No overlap, should return empty
        assert len(results) == 0

    async def test_except_all_removed(self, customers_2023, customers_2024):
        """Test EXCEPT when all rows would be removed."""
        # Find only common customers
        results = await execute("""
            SELECT * FROM customers_2023 WHERE id <= 3
            EXCEPT
            SELECT * FROM customers_2024
        """)

        # All customers in query_2023_common are also in 2024
        assert len(results) == 0
