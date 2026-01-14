"""
Integration tests for Common Table Expressions (CTEs / WITH clauses).

Tests query_with_cte with real PostgreSQL database.
"""
import pytest
from datetime import datetime, timezone
from ouroboros.postgres import execute, insert_one, query_with_cte


@pytest.fixture
async def test_tables():
    """Create and populate test tables for CTE testing."""
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
            amount DECIMAL(10, 2) NOT NULL,
            status VARCHAR(50) NOT NULL,
            created_at TIMESTAMP DEFAULT NOW()
        )
        """
    )

    # Insert test users
    users_data = [
        {"name": "Alice", "email": "alice@example.com", "status": "active"},
        {"name": "Bob", "email": "bob@example.com", "status": "active"},
        {"name": "Charlie", "email": "charlie@example.com", "status": "inactive"},
        {"name": "David", "email": "david@example.com", "status": "active"},
        {"name": "Eve", "email": "eve@example.com", "status": "active"},
    ]
    for data in users_data:
        await insert_one("users", data)

    # Insert test orders
    orders_data = [
        # Alice - 3 orders (2 completed, 1 pending)
        {"user_id": 1, "amount": 100.00, "status": "completed"},
        {"user_id": 1, "amount": 200.00, "status": "completed"},
        {"user_id": 1, "amount": 50.00, "status": "pending"},
        # Bob - 2 orders (1 completed, 1 pending)
        {"user_id": 2, "amount": 150.00, "status": "completed"},
        {"user_id": 2, "amount": 75.00, "status": "pending"},
        # Charlie - 1 order (cancelled)
        {"user_id": 3, "amount": 25.00, "status": "cancelled"},
        # David - 2 orders (both completed)
        {"user_id": 4, "amount": 300.00, "status": "completed"},
        {"user_id": 4, "amount": 250.00, "status": "completed"},
        # Eve - 0 orders
    ]
    for data in orders_data:
        await insert_one("orders", data)

    yield

    # Cleanup
    await execute("DROP TABLE IF EXISTS orders")
    await execute("DROP TABLE IF EXISTS users")


@pytest.mark.asyncio
class TestSimpleCTE:
    """Test simple CTE functionality."""

    async def test_simple_cte_no_params(self, test_tables):
        """Test simple CTE without parameters."""
        # Find all completed orders via CTE
        results = await query_with_cte(
            "completed_orders",
            [("completed_orders", "SELECT * FROM orders WHERE status = 'completed'", [])],
            select_columns=["user_id", "amount"],
            order_by=[("amount", "desc")]
        )

        # Should get 5 completed orders
        assert len(results) == 5
        assert results[0]["amount"] == 300.00  # Highest first

    async def test_simple_cte_with_params(self, test_tables):
        """Test simple CTE with parameters."""
        # Find orders above a threshold using parameter
        results = await query_with_cte(
            "high_value_orders",
            [("high_value_orders", "SELECT * FROM orders WHERE amount > $1", [100.00])],
            select_columns=["user_id", "amount", "status"],
            order_by=[("amount", "asc")]
        )

        # Should get 4 orders > 100: 150, 200, 250, 300
        assert len(results) == 4
        assert results[0]["amount"] == 150.00
        assert results[-1]["amount"] == 300.00

    async def test_cte_with_aggregation(self, test_tables):
        """Test CTE that performs aggregation."""
        # Create CTE that aggregates orders by user
        results = await query_with_cte(
            "user_totals",
            [(
                "user_totals",
                "SELECT user_id, SUM(amount) as total, COUNT(*) as order_count FROM orders GROUP BY user_id",
                []
            )],
            select_columns=["user_id", "total", "order_count"],
            order_by=[("total", "desc")]
        )

        # Should get 4 users with orders
        assert len(results) == 4

        # David has highest total: 300 + 250 = 550
        assert results[0]["user_id"] == 4
        assert float(results[0]["total"]) == 550.00
        assert results[0]["order_count"] == 2


@pytest.mark.asyncio
class TestCTEWithWhereConditions:
    """Test CTE combined with WHERE conditions."""

    async def test_cte_with_simple_where(self, test_tables):
        """Test CTE with WHERE clause on main query."""
        # CTE gets completed orders, then filter by amount
        results = await query_with_cte(
            "completed_orders",
            [("completed_orders", "SELECT * FROM orders WHERE status = $1", ["completed"])],
            select_columns=["user_id", "amount"],
            where_conditions=[("amount", "gt", 150)],
            order_by=[("amount", "asc")]
        )

        # Should get 3 orders: 200, 250, 300
        assert len(results) == 3
        assert results[0]["amount"] == 200.00
        assert results[-1]["amount"] == 300.00

    async def test_cte_with_multiple_where(self, test_tables):
        """Test CTE with multiple WHERE conditions."""
        results = await query_with_cte(
            "all_orders",
            [("all_orders", "SELECT * FROM orders", [])],
            select_columns=["user_id", "amount", "status"],
            where_conditions=[
                ("status", "eq", "completed"),
                ("amount", "gte", 100)
            ],
            order_by=[("user_id", "asc")]
        )

        # Completed AND amount >= 100: Alice(100, 200), Bob(150), David(300, 250)
        assert len(results) == 5


@pytest.mark.asyncio
class TestCTEWithOrderAndLimit:
    """Test CTE with ORDER BY and LIMIT."""

    async def test_cte_with_order_by(self, test_tables):
        """Test CTE with ORDER BY."""
        results = await query_with_cte(
            "all_orders",
            [("all_orders", "SELECT * FROM orders", [])],
            select_columns=["user_id", "amount"],
            order_by=[("amount", "desc"), ("user_id", "asc")]
        )

        # Should be ordered by amount desc, then user_id asc
        assert len(results) == 8
        assert results[0]["amount"] == 300.00
        assert results[-1]["amount"] == 25.00

    async def test_cte_with_limit(self, test_tables):
        """Test CTE with LIMIT."""
        results = await query_with_cte(
            "all_orders",
            [("all_orders", "SELECT * FROM orders WHERE status = $1", ["completed"])],
            select_columns=["amount"],
            order_by=[("amount", "desc")],
            limit=3
        )

        # Top 3 completed orders: 300, 250, 200
        assert len(results) == 3
        assert results[0]["amount"] == 300.00
        assert results[1]["amount"] == 250.00
        assert results[2]["amount"] == 200.00


@pytest.mark.asyncio
class TestMultipleCTEs:
    """Test multiple CTEs in a single query."""

    async def test_two_ctes_independent(self, test_tables):
        """Test two independent CTEs."""
        # Create two CTEs and query from the second one
        results = await query_with_cte(
            "high_value",
            [
                ("completed", "SELECT * FROM orders WHERE status = $1", ["completed"]),
                ("high_value", "SELECT * FROM completed WHERE amount > $1", [150])
            ],
            select_columns=["user_id", "amount"],
            order_by=[("amount", "asc")]
        )

        # Orders that are completed AND > 150: 200, 250, 300
        assert len(results) == 3
        assert results[0]["amount"] == 200.00
        assert results[-1]["amount"] == 300.00

    async def test_cte_referencing_another_cte(self, test_tables):
        """Test CTE that references another CTE."""
        # First CTE: get user order counts
        # Second CTE: filter users with multiple orders
        results = await query_with_cte(
            "active_customers",
            [
                (
                    "user_orders",
                    "SELECT user_id, COUNT(*) as order_count FROM orders GROUP BY user_id",
                    []
                ),
                (
                    "active_customers",
                    "SELECT user_id FROM user_orders WHERE order_count > $1",
                    [1]
                )
            ],
            select_columns=["user_id"],
            order_by=[("user_id", "asc")]
        )

        # Users with > 1 order: Alice (3), Bob (2), David (2)
        assert len(results) == 3
        assert results[0]["user_id"] == 1  # Alice
        assert results[1]["user_id"] == 2  # Bob
        assert results[2]["user_id"] == 4  # David

    async def test_multiple_ctes_with_aggregation(self, test_tables):
        """Test multiple CTEs with different aggregations."""
        results = await query_with_cte(
            "summary",
            [
                (
                    "completed_orders",
                    "SELECT user_id, SUM(amount) as total FROM orders WHERE status = $1 GROUP BY user_id",
                    ["completed"]
                ),
                (
                    "summary",
                    "SELECT user_id, total FROM completed_orders WHERE total > $1",
                    [200]
                )
            ],
            select_columns=["user_id", "total"],
            order_by=[("total", "desc")]
        )

        # Users with completed order total > 200: David (550), Alice (300)
        assert len(results) == 2
        assert results[0]["user_id"] == 4  # David
        assert float(results[0]["total"]) == 550.00
        assert results[1]["user_id"] == 1  # Alice
        assert float(results[1]["total"]) == 300.00


@pytest.mark.asyncio
class TestCTEWithSubquery:
    """Test CTE combined with subqueries."""

    async def test_cte_with_in_subquery(self, test_tables):
        """Test CTE with IN subquery."""
        # CTE for high-value orders, then find users in that CTE
        results = await query_with_cte(
            "users",
            [("high_value_orders", "SELECT user_id FROM orders WHERE amount > $1", [100.00])],
            select_columns=["name", "email"],
            subqueries=[
                ("in", "id", "SELECT user_id FROM high_value_orders", [])
            ],
            order_by=[("name", "asc")]
        )

        # Users with orders > 100: Alice, Bob, David
        assert len(results) == 3
        names = [r["name"] for r in results]
        assert "Alice" in names
        assert "Bob" in names
        assert "David" in names

    async def test_cte_with_exists_subquery(self, test_tables):
        """Test CTE with EXISTS subquery."""
        results = await query_with_cte(
            "users",
            [("completed_orders", "SELECT user_id FROM orders WHERE status = $1", ["completed"])],
            select_columns=["name", "status"],
            subqueries=[
                ("exists", None, "SELECT 1 FROM completed_orders WHERE completed_orders.user_id = users.id", [])
            ],
            where_conditions=[("status", "eq", "active")],
            order_by=[("name", "asc")]
        )

        # Active users with completed orders: Alice, Bob, David
        assert len(results) == 3
        names = [r["name"] for r in results]
        assert "Alice" in names
        assert "Bob" in names
        assert "David" in names

    async def test_cte_with_not_in_subquery(self, test_tables):
        """Test CTE with NOT IN subquery."""
        results = await query_with_cte(
            "users",
            [("order_users", "SELECT DISTINCT user_id FROM orders", [])],
            select_columns=["name", "email"],
            subqueries=[
                ("not_in", "id", "SELECT user_id FROM order_users", [])
            ],
            order_by=[("name", "asc")]
        )

        # Users without orders: Eve
        assert len(results) == 1
        assert results[0]["name"] == "Eve"


@pytest.mark.asyncio
class TestCTEJoiningTables:
    """Test CTE that joins multiple tables."""

    async def test_cte_with_join(self, test_tables):
        """Test CTE that performs a JOIN."""
        # CTE joins users and orders
        results = await query_with_cte(
            "user_orders",
            [(
                "user_orders",
                """
                SELECT u.name, u.email, o.amount, o.status
                FROM users u
                JOIN orders o ON u.id = o.user_id
                WHERE u.status = $1
                """,
                ["active"]
            )],
            select_columns=["name", "amount", "status"],
            order_by=[("name", "asc"), ("amount", "desc")]
        )

        # All orders from active users
        assert len(results) >= 5

        # Check that inactive user (Charlie) is not in results
        names = [r["name"] for r in results]
        assert "Charlie" not in names
        assert "Alice" in names
        assert "Bob" in names

    async def test_cte_with_complex_join(self, test_tables):
        """Test CTE with complex join and aggregation."""
        results = await query_with_cte(
            "user_stats",
            [(
                "user_stats",
                """
                SELECT
                    u.id,
                    u.name,
                    COUNT(o.id) as order_count,
                    COALESCE(SUM(o.amount), 0) as total_spent
                FROM users u
                LEFT JOIN orders o ON u.id = o.user_id AND o.status = $1
                GROUP BY u.id, u.name
                """,
                ["completed"]
            )],
            select_columns=["name", "order_count", "total_spent"],
            where_conditions=[("order_count", "gt", 0)],
            order_by=[("total_spent", "desc")]
        )

        # Users with completed orders
        assert len(results) >= 3

        # David should have highest total (550)
        assert results[0]["name"] == "David"
        assert float(results[0]["total_spent"]) == 550.00


@pytest.mark.asyncio
class TestRecursiveCTE:
    """Test recursive CTEs."""

    async def test_simple_recursive_cte(self, test_tables):
        """Test simple recursive CTE (number sequence)."""
        # Generate numbers 1-10 using recursive CTE
        results = await query_with_cte(
            "numbers",
            [(
                "numbers",
                """
                WITH RECURSIVE numbers AS (
                    SELECT 1 as n
                    UNION ALL
                    SELECT n + 1 FROM numbers WHERE n < 10
                )
                SELECT * FROM numbers
                """,
                []
            )],
            select_columns=["n"],
            order_by=[("n", "asc")]
        )

        assert len(results) == 10
        assert results[0]["n"] == 1
        assert results[-1]["n"] == 10

    async def test_recursive_cte_with_data(self):
        """Test recursive CTE with real data."""
        # Drop and create a categories table with parent-child relationships
        await execute("DROP TABLE IF EXISTS categories")
        await execute(
            """
            CREATE TABLE categories (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                parent_id INTEGER
            )
            """
        )

        try:
            # Insert hierarchical data in a single batch to avoid sequential insert issues
            await execute(
                """
                INSERT INTO categories (name, parent_id) VALUES
                    ('Electronics', NULL),
                    ('Computers', 1),
                    ('Laptops', 2),
                    ('Desktops', 2),
                    ('Phones', 1)
                """
            )

            # Get all descendants of Electronics (id=1)
            results = await query_with_cte(
                "category_tree",
                [(
                    "category_tree",
                    """
                    WITH RECURSIVE category_tree AS (
                        SELECT id, name, parent_id, 1 as level
                        FROM categories
                        WHERE id = $1
                        UNION ALL
                        SELECT c.id, c.name, c.parent_id, ct.level + 1
                        FROM categories c
                        JOIN category_tree ct ON c.parent_id = ct.id
                    )
                    SELECT * FROM category_tree
                    """,
                    [1]  # Start from Electronics
                )],
                select_columns=["id", "name", "level"],
                order_by=[("level", "asc"), ("id", "asc")]
            )

            # Should get all 5 categories
            assert len(results) == 5

            # Check levels
            electronics = next(r for r in results if r["name"] == "Electronics")
            assert electronics["level"] == 1

            computers = next(r for r in results if r["name"] == "Computers")
            assert computers["level"] == 2

            laptops = next(r for r in results if r["name"] == "Laptops")
            assert laptops["level"] == 3
        finally:
            # Cleanup
            await execute("DROP TABLE IF EXISTS categories")


@pytest.mark.asyncio
class TestCTEEdgeCases:
    """Test edge cases and special scenarios."""

    @pytest.fixture
    async def edge_test_tables(self):
        """Create fresh test tables for edge case testing."""
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
                amount DECIMAL(10, 2) NOT NULL,
                status VARCHAR(50) NOT NULL,
                created_at TIMESTAMP DEFAULT NOW()
            )
            """
        )

        # Insert test users
        users_data = [
            {"name": "Alice", "email": "alice@example.com", "status": "active"},
            {"name": "Bob", "email": "bob@example.com", "status": "active"},
            {"name": "David", "email": "david@example.com", "status": "active"},
        ]
        for data in users_data:
            await insert_one("users", data)

        # Insert test orders
        orders_data = [
            {"user_id": 1, "amount": 100.00, "status": "completed"},
            {"user_id": 1, "amount": 200.00, "status": "completed"},
            {"user_id": 2, "amount": 150.00, "status": "completed"},
            {"user_id": 3, "amount": 300.00, "status": "completed"},
            {"user_id": 3, "amount": 250.00, "status": "completed"},
        ]
        for data in orders_data:
            await insert_one("orders", data)

        yield

        # Cleanup happens via cleanup_tables fixture

    async def test_cte_with_no_results(self, edge_test_tables):
        """Test CTE that returns no results."""
        results = await query_with_cte(
            "empty_cte",
            [("empty_cte", "SELECT * FROM orders WHERE amount > $1", [10000])],
            select_columns=["user_id", "amount"]
        )

        assert len(results) == 0

    async def test_cte_with_all_features(self, edge_test_tables):
        """Test CTE with all features combined."""
        # Complex query with CTE, WHERE, subquery, ORDER BY, LIMIT
        results = await query_with_cte(
            "high_value_orders",
            [
                ("completed", "SELECT * FROM orders WHERE status = $1", ["completed"]),
                ("high_value_orders", "SELECT * FROM completed WHERE amount > $1", [100])
            ],
            select_columns=["user_id", "amount"],
            where_conditions=[("amount", "lt", 300)],
            subqueries=[
                ("in", "user_id", "SELECT id FROM users WHERE status = $1", ["active"])
            ],
            order_by=[("amount", "desc")],
            limit=3
        )

        # Completed, > 100, < 300, from active users, top 3
        # Should get: 250 (David), 200 (Alice), 150 (Bob)
        assert len(results) <= 3

        for result in results:
            assert 100 < result["amount"] < 300

    async def test_cte_select_all_columns(self, test_tables):
        """Test CTE with select_columns=None (SELECT *)."""
        results = await query_with_cte(
            "all_orders",
            [("all_orders", "SELECT * FROM orders WHERE status = $1", ["completed"])],
            select_columns=None,  # Should select all columns
            limit=1
        )

        assert len(results) == 1

        # Should have all order columns
        result = results[0]
        assert "id" in result
        assert "user_id" in result
        assert "amount" in result
        assert "status" in result
        assert "created_at" in result

    async def test_cte_with_null_values(self, test_tables):
        """Test CTE handling of NULL values."""
        # Create table with nullable columns
        await execute("DROP TABLE IF EXISTS products")
        await execute(
            """
            CREATE TABLE products (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                description TEXT,
                price DECIMAL(10, 2)
            )
            """
        )

        # Insert data with NULLs
        await insert_one("products", {"name": "Product A", "description": None, "price": None})
        await insert_one("products", {"name": "Product B", "description": "Description", "price": 99.99})

        results = await query_with_cte(
            "all_products",
            [("all_products", "SELECT * FROM products", [])],
            select_columns=["name", "description", "price"],
            order_by=[("name", "asc")]
        )

        assert len(results) == 2

        # Check NULL handling
        assert results[0]["name"] == "Product A"
        assert results[0]["description"] is None
        assert results[0]["price"] is None

        assert results[1]["name"] == "Product B"
        assert results[1]["description"] == "Description"
        assert float(results[1]["price"]) == 99.99

        # Cleanup
        await execute("DROP TABLE IF EXISTS products")


@pytest.mark.asyncio
class TestCTEErrorHandling:
    """Test error handling for CTEs."""

    async def test_invalid_cte_sql(self, test_tables):
        """Test error on invalid CTE SQL."""
        with pytest.raises(Exception):  # Should raise database error
            await query_with_cte(
                "bad_cte",
                [("bad_cte", "SELECT * FROM nonexistent_table", [])],
                select_columns=["id"]
            )

    async def test_invalid_main_table(self, test_tables):
        """Test error when main table doesn't exist and no CTE matches."""
        with pytest.raises(Exception):  # Should raise database error
            await query_with_cte(
                "nonexistent_table",
                [("some_cte", "SELECT * FROM orders", [])],
                select_columns=["id"]
            )

    async def test_cte_with_invalid_column(self, test_tables):
        """Test error when selecting non-existent column from CTE."""
        with pytest.raises(Exception):  # Should raise database error
            await query_with_cte(
                "orders_cte",
                [("orders_cte", "SELECT user_id, amount FROM orders", [])],
                select_columns=["nonexistent_column"]
            )

    async def test_cte_parameter_mismatch(self, test_tables):
        """Test error when CTE parameters don't match placeholders."""
        with pytest.raises(Exception):  # Should raise database error
            await query_with_cte(
                "orders_cte",
                [("orders_cte", "SELECT * FROM orders WHERE status = $1 AND amount > $2", ["completed"])],
                # Missing second parameter
                select_columns=["id"]
            )


@pytest.mark.asyncio
class TestCTEPerformance:
    """Test CTE performance characteristics."""

    async def test_cte_materialization(self, test_tables):
        """Test that CTE is materialized and can be referenced multiple times."""
        # This would fail if CTE wasn't materialized properly
        results = await query_with_cte(
            "user_stats",
            [(
                "user_stats",
                """
                SELECT
                    user_id,
                    COUNT(*) as order_count,
                    SUM(amount) as total
                FROM orders
                GROUP BY user_id
                """,
                []
            )],
            select_columns=["user_id", "order_count", "total"],
            where_conditions=[
                ("order_count", "gte", 2),
                ("total", "gt", 200)
            ],
            order_by=[("total", "desc")]
        )

        # Users with >= 2 orders and total > 200
        assert len(results) >= 1

        for result in results:
            assert result["order_count"] >= 2
            assert float(result["total"]) > 200

    async def test_cte_vs_subquery_equivalence(self, test_tables):
        """Test that CTE produces same results as equivalent subquery."""
        # Query using CTE
        cte_results = await query_with_cte(
            "high_spenders",
            [(
                "high_spenders",
                "SELECT user_id FROM orders GROUP BY user_id HAVING SUM(amount) > $1",
                [200]
            )],
            select_columns=["user_id"],
            order_by=[("user_id", "asc")]
        )

        # Query using subquery (via execute)
        subquery_results = await execute(
            """
            SELECT user_id
            FROM (
                SELECT user_id
                FROM orders
                GROUP BY user_id
                HAVING SUM(amount) > $1
            ) AS high_spenders
            ORDER BY user_id ASC
            """,
            [200]
        )

        # Should produce identical results
        assert len(cte_results) == len(subquery_results)
        for cte_row, sub_row in zip(cte_results, subquery_results):
            assert cte_row["user_id"] == sub_row["user_id"]
