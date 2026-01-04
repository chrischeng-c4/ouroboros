"""
Integration tests for subquery support.

Tests subquery functionality (IN, NOT IN, EXISTS, NOT EXISTS) with real PostgreSQL database.
"""
import pytest
from data_bridge.postgres import execute, insert_one, query_aggregate, query_with_cte


@pytest.fixture
async def test_tables():
    """Create and populate test tables for subquery testing."""
    # Create users table
    await execute(
        """
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            email VARCHAR(100) NOT NULL,
            status VARCHAR(50) NOT NULL
        )
        """
    )

    # Create orders table
    await execute(
        """
        CREATE TABLE IF NOT EXISTS orders (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL,
            total DECIMAL(10, 2) NOT NULL,
            status VARCHAR(50) NOT NULL
        )
        """
    )

    # Insert test users
    users_data = [
        {"name": "Alice", "email": "alice@example.com", "status": "active"},
        {"name": "Bob", "email": "bob@example.com", "status": "active"},
        {"name": "Charlie", "email": "charlie@example.com", "status": "inactive"},
        {"name": "David", "email": "david@example.com", "status": "active"},
    ]
    for data in users_data:
        await insert_one("users", data)

    # Insert test orders (Alice and Bob have orders, Charlie and David don't)
    orders_data = [
        {"user_id": 1, "total": 100.00, "status": "completed"},  # Alice
        {"user_id": 1, "total": 200.00, "status": "pending"},    # Alice
        {"user_id": 2, "total": 150.00, "status": "completed"},  # Bob
    ]
    for data in orders_data:
        await insert_one("orders", data)

    yield

    # Cleanup
    await execute("DROP TABLE IF EXISTS orders")
    await execute("DROP TABLE IF EXISTS users")


@pytest.mark.asyncio
class TestSubqueryIn:
    """Test IN subquery functionality."""

    async def test_where_in_subquery(self, test_tables):
        """Test WHERE column IN (subquery)."""
        # Find users who have orders
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("in", "id", "SELECT user_id FROM orders", [])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 2  # Alice and Bob have orders

    async def test_where_in_subquery_with_params(self, test_tables):
        """Test WHERE column IN (subquery with parameters)."""
        # Find users who have completed orders
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("in", "id", "SELECT user_id FROM orders WHERE status = $1", ["completed"])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 2  # Alice and Bob have completed orders

    async def test_where_not_in_subquery(self, test_tables):
        """Test WHERE column NOT IN (subquery)."""
        # Find users who DON'T have orders
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("not_in", "id", "SELECT user_id FROM orders", [])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 2  # Charlie and David don't have orders


@pytest.mark.asyncio
class TestSubqueryExists:
    """Test EXISTS subquery functionality."""

    async def test_where_exists(self, test_tables):
        """Test WHERE EXISTS (subquery)."""
        # Find users who have orders (using correlated subquery)
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("exists", None, "SELECT 1 FROM orders WHERE orders.user_id = users.id", [])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 2  # Alice and Bob

    async def test_where_not_exists(self, test_tables):
        """Test WHERE NOT EXISTS (subquery)."""
        # Find users who DON'T have orders
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("not_exists", None, "SELECT 1 FROM orders WHERE orders.user_id = users.id", [])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 2  # Charlie and David

    async def test_where_exists_with_condition(self, test_tables):
        """Test WHERE EXISTS with additional conditions."""
        # Find users who have high-value orders (> $150)
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("exists", None, "SELECT 1 FROM orders WHERE orders.user_id = users.id AND orders.total > $1", [150.00])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 1  # Only Alice has order > 150


@pytest.mark.asyncio
class TestSubqueryCombinations:
    """Test combining subqueries with other WHERE conditions."""

    async def test_subquery_with_where_conditions(self, test_tables):
        """Test subquery combined with regular WHERE conditions."""
        # Find active users who have orders
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=[("status", "eq", "active")],
            order_by=None,
            limit=None,
            subqueries=[
                ("in", "id", "SELECT user_id FROM orders", [])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 2  # Alice and Bob are active and have orders

    async def test_multiple_subqueries(self, test_tables):
        """Test multiple subquery conditions."""
        # Find users who have orders AND have completed orders
        results = await query_aggregate(
            "users",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("in", "id", "SELECT user_id FROM orders", []),
                ("exists", None, "SELECT 1 FROM orders WHERE orders.user_id = users.id AND orders.status = $1", ["completed"])
            ]
        )

        assert len(results) == 1
        assert results[0]["total"] == 2  # Alice and Bob


@pytest.mark.asyncio
class TestSubqueryWithCTE:
    """Test subqueries with CTEs."""

    async def test_cte_with_subquery(self, test_tables):
        """Test query_with_cte with subquery support."""
        # Create a CTE for high-value orders, then find users in that CTE
        results = await query_with_cte(
            "users",
            [("high_value_orders", "SELECT user_id FROM orders WHERE total > $1", [100.00])],
            select_columns=["name", "email"],
            where_conditions=None,
            order_by=None,
            limit=None,
            subqueries=[
                ("in", "id", "SELECT user_id FROM high_value_orders", [])
            ]
        )

        # Should return Alice and Bob (both have orders > 100)
        assert len(results) == 2
        names = [r["name"] for r in results]
        assert "Alice" in names
        assert "Bob" in names


@pytest.mark.asyncio
class TestSubqueryErrors:
    """Test error handling for subqueries."""

    async def test_invalid_subquery_type(self, test_tables):
        """Test that invalid subquery type raises error."""
        with pytest.raises(Exception) as exc_info:
            await query_aggregate(
                "users",
                [("count", None, "total")],
                group_by=None,
                having=None,
                where_conditions=None,
                order_by=None,
                limit=None,
                subqueries=[
                    ("invalid_type", "id", "SELECT user_id FROM orders", [])
                ]
            )

        assert "Unknown subquery type" in str(exc_info.value)

    async def test_in_subquery_without_field(self, test_tables):
        """Test that IN subquery without field raises error."""
        with pytest.raises(Exception) as exc_info:
            await query_aggregate(
                "users",
                [("count", None, "total")],
                group_by=None,
                having=None,
                where_conditions=None,
                order_by=None,
                limit=None,
                subqueries=[
                    ("in", None, "SELECT user_id FROM orders", [])
                ]
            )

        assert "IN subquery requires a field name" in str(exc_info.value)
