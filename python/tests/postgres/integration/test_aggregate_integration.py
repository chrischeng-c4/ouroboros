"""
Integration tests for aggregate query functionality.

Tests query_aggregate with a real PostgreSQL database.
"""
import pytest
from ouroboros.qc import expect
from ouroboros.postgres import execute, insert_one, query_aggregate


@pytest.fixture
async def orders_table():
    """Create and populate an orders table for aggregate testing."""
    # Create table
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

    # Insert test data
    test_data = [
        {"user_id": 1, "amount": 100.50, "status": "completed"},
        {"user_id": 1, "amount": 200.00, "status": "completed"},
        {"user_id": 1, "amount": 50.25, "status": "pending"},
        {"user_id": 2, "amount": 150.00, "status": "completed"},
        {"user_id": 2, "amount": 300.75, "status": "completed"},
        {"user_id": 3, "amount": 75.00, "status": "cancelled"},
    ]

    for data in test_data:
        await insert_one("orders", data)

    yield

    # Cleanup
    await execute("DROP TABLE IF EXISTS orders")


@pytest.mark.asyncio
class TestBasicAggregates:
    """Test basic aggregate functions."""

    async def test_count_all(self, orders_table):
        """Test COUNT(*) - count all rows."""
        results = await query_aggregate(
            "orders",
            [("count", None, "total")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        assert results[0]["total"] == 6

    async def test_count_column(self, orders_table):
        """Test COUNT(column) - count non-null values."""
        results = await query_aggregate(
            "orders",
            [("count_column", "user_id", "user_count")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        assert results[0]["user_count"] == 6

    async def test_count_distinct(self, orders_table):
        """Test COUNT(DISTINCT column) - count unique values."""
        results = await query_aggregate(
            "orders",
            [("count_distinct", "user_id", "unique_users")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        assert results[0]["unique_users"] == 3  # 3 unique users

    async def test_sum(self, orders_table):
        """Test SUM(column) - sum of all values."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total_amount")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        # 100.50 + 200 + 50.25 + 150 + 300.75 + 75 = 876.50
        assert float(results[0]["total_amount"]) == pytest.approx(876.50, rel=0.01)

    async def test_avg(self, orders_table):
        """Test AVG(column) - average of values."""
        results = await query_aggregate(
            "orders",
            [("avg", "amount", "avg_amount")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        # 876.50 / 6 = 146.08
        assert float(results[0]["avg_amount"]) == pytest.approx(146.08, rel=0.01)

    async def test_min(self, orders_table):
        """Test MIN(column) - minimum value."""
        results = await query_aggregate(
            "orders",
            [("min", "amount", "min_amount")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        assert float(results[0]["min_amount"]) == pytest.approx(50.25, rel=0.01)

    async def test_max(self, orders_table):
        """Test MAX(column) - maximum value."""
        results = await query_aggregate(
            "orders",
            [("max", "amount", "max_amount")],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        assert float(results[0]["max_amount"]) == pytest.approx(300.75, rel=0.01)


@pytest.mark.asyncio
class TestGroupBy:
    """Test GROUP BY functionality."""

    async def test_group_by_single_column(self, orders_table):
        """Test GROUP BY with single column."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total"), ("count", None, "count")],
            group_by=["user_id"],
            having=None,
            where_conditions=None,
            order_by=[("user_id", "asc")],
            limit=None
        )

        assert len(results) == 3  # 3 unique users

        # User 1: 100.50 + 200.00 + 50.25 = 350.75
        assert results[0]["user_id"] == 1
        assert float(results[0]["total"]) == pytest.approx(350.75, rel=0.01)
        assert results[0]["count"] == 3

        # User 2: 150.00 + 300.75 = 450.75
        assert results[1]["user_id"] == 2
        assert float(results[1]["total"]) == pytest.approx(450.75, rel=0.01)
        assert results[1]["count"] == 2

        # User 3: 75.00
        assert results[2]["user_id"] == 3
        assert float(results[2]["total"]) == pytest.approx(75.00, rel=0.01)
        assert results[2]["count"] == 1

    async def test_group_by_multiple_columns(self, orders_table):
        """Test GROUP BY with multiple columns."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total")],
            group_by=["user_id", "status"],
            having=None,
            where_conditions=None,
            order_by=[("user_id", "asc"), ("status", "asc")],
            limit=None
        )

        # Should have separate groups for each user_id + status combination
        assert len(results) >= 3  # At least 3 groups


@pytest.mark.asyncio
class TestWhereConditions:
    """Test WHERE clause filtering with aggregates."""

    async def test_where_single_condition(self, orders_table):
        """Test aggregate with single WHERE condition."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total")],
            group_by=None,
            having=None,
            where_conditions=[("status", "eq", "completed")],
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        # 100.50 + 200.00 + 150.00 + 300.75 = 751.25
        assert float(results[0]["total"]) == pytest.approx(751.25, rel=0.01)

    async def test_where_multiple_conditions(self, orders_table):
        """Test aggregate with multiple WHERE conditions (AND)."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total")],
            group_by=None,
            having=None,
            where_conditions=[
                ("status", "eq", "completed"),
                ("amount", "gt", 150)
            ],
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        # Only 200.00 and 300.75 match both conditions = 500.75
        assert float(results[0]["total"]) == pytest.approx(500.75, rel=0.01)

    async def test_where_with_group_by(self, orders_table):
        """Test WHERE clause combined with GROUP BY."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total"), ("count", None, "count")],
            group_by=["user_id"],
            having=None,
            where_conditions=[("status", "eq", "completed")],
            order_by=[("user_id", "asc")],
            limit=None
        )

        # User 1: 100.50 + 200.00 = 300.50 (2 completed orders)
        # User 2: 150.00 + 300.75 = 450.75 (2 completed orders)
        # User 3: 0 (no completed orders)
        assert len(results) == 2  # Only users with completed orders

        assert results[0]["user_id"] == 1
        assert float(results[0]["total"]) == pytest.approx(300.50, rel=0.01)
        assert results[0]["count"] == 2

        assert results[1]["user_id"] == 2
        assert float(results[1]["total"]) == pytest.approx(450.75, rel=0.01)
        assert results[1]["count"] == 2


@pytest.mark.asyncio
class TestOrderByAndLimit:
    """Test ORDER BY and LIMIT with aggregates."""

    async def test_order_by_aggregate(self, orders_table):
        """Test ordering by aggregate result."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total")],
            group_by=["user_id"],
            having=None,
            where_conditions=None,
            order_by=[("total", "desc")],
            limit=None
        )

        assert len(results) == 3
        # Should be ordered: User 2 (450.75), User 1 (350.75), User 3 (75.00)
        assert results[0]["user_id"] == 2
        assert results[1]["user_id"] == 1
        assert results[2]["user_id"] == 3

    async def test_limit_results(self, orders_table):
        """Test LIMIT on aggregate results."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total")],
            group_by=["user_id"],
            having=None,
            where_conditions=None,
            order_by=[("total", "desc")],
            limit=2
        )

        assert len(results) == 2  # Limited to top 2
        assert results[0]["user_id"] == 2  # Highest total
        assert results[1]["user_id"] == 1  # Second highest


@pytest.mark.asyncio
class TestMultipleAggregates:
    """Test multiple aggregate functions in single query."""

    async def test_multiple_aggregates_no_group(self, orders_table):
        """Test multiple aggregates without GROUP BY."""
        results = await query_aggregate(
            "orders",
            [
                ("count", None, "count"),
                ("sum", "amount", "total"),
                ("avg", "amount", "avg"),
                ("min", "amount", "min"),
                ("max", "amount", "max"),
            ],
            group_by=None,
            having=None,
            where_conditions=None,
            order_by=None,
            limit=None
        )

        assert len(results) == 1
        assert results[0]["count"] == 6
        assert float(results[0]["total"]) == pytest.approx(876.50, rel=0.01)
        assert float(results[0]["avg"]) == pytest.approx(146.08, rel=0.01)
        assert float(results[0]["min"]) == pytest.approx(50.25, rel=0.01)
        assert float(results[0]["max"]) == pytest.approx(300.75, rel=0.01)

    async def test_multiple_aggregates_with_group(self, orders_table):
        """Test multiple aggregates with GROUP BY."""
        results = await query_aggregate(
            "orders",
            [
                ("count", None, "order_count"),
                ("sum", "amount", "total_amount"),
                ("avg", "amount", "avg_amount"),
            ],
            group_by=["user_id"],
            having=None,
            where_conditions=None,
            order_by=[("user_id", "asc")],
            limit=None
        )

        assert len(results) == 3

        # Verify all aggregate columns are present
        for result in results:
            assert "user_id" in result
            assert "order_count" in result
            assert "total_amount" in result
            assert "avg_amount" in result


@pytest.mark.asyncio
class TestOperatorVariations:
    """Test different operator string variations."""

    async def test_eq_operator_variations(self, orders_table):
        """Test 'eq' and '=' both work."""
        results1 = await query_aggregate(
            "orders",
            [("count", None, "count")],
            group_by=None,
            having=None,
            where_conditions=[("status", "eq", "completed")],
            order_by=None,
            limit=None
        )

        results2 = await query_aggregate(
            "orders",
            [("count", None, "count")],
            group_by=None,
            having=None,
            where_conditions=[("status", "=", "completed")],
            order_by=None,
            limit=None
        )

        # Both should give same result
        assert results1[0]["count"] == results2[0]["count"] == 4

    async def test_comparison_operators(self, orders_table):
        """Test gt, gte, lt, lte operators."""
        # Greater than
        results = await query_aggregate(
            "orders",
            [("count", None, "count")],
            group_by=None,
            having=None,
            where_conditions=[("amount", "gt", 100)],
            order_by=None,
            limit=None
        )
        assert results[0]["count"] == 3  # 200, 150, 300.75

        # Greater than or equal
        results = await query_aggregate(
            "orders",
            [("count", None, "count")],
            group_by=None,
            having=None,
            where_conditions=[("amount", "gte", 100)],
            order_by=None,
            limit=None
        )
        assert results[0]["count"] == 4  # Includes 100.50

        # Less than
        results = await query_aggregate(
            "orders",
            [("count", None, "count")],
            group_by=None,
            having=None,
            where_conditions=[("amount", "lt", 100)],
            order_by=None,
            limit=None
        )
        assert results[0]["count"] == 2  # 50.25, 75.00


@pytest.mark.asyncio
class TestHavingClause:
    """Test HAVING clause functionality."""

    async def test_having_simple_condition(self, orders_table):
        """Test HAVING with simple aggregate condition."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total"), ("count", None, "count")],
            group_by=["user_id"],
            having=[("sum", "amount", "gt", 400)],
            where_conditions=None,
            order_by=[("user_id", "asc")],
            limit=None
        )

        # Only user 2 has total > 400 (450.75)
        assert len(results) == 1
        assert results[0]["user_id"] == 2
        assert float(results[0]["total"]) == pytest.approx(450.75, rel=0.01)

    async def test_having_count_condition(self, orders_table):
        """Test HAVING with COUNT condition."""
        results = await query_aggregate(
            "orders",
            [("count", None, "order_count"), ("sum", "amount", "total")],
            group_by=["user_id"],
            having=[("count", None, "gte", 2)],
            where_conditions=None,
            order_by=[("user_id", "asc")],
            limit=None
        )

        # User 1 has 3 orders, User 2 has 2 orders, User 3 has 1 order
        assert len(results) == 2  # Users 1 and 2
        assert results[0]["user_id"] == 1
        assert results[0]["order_count"] == 3
        assert results[1]["user_id"] == 2
        assert results[1]["order_count"] == 2

    async def test_having_avg_condition(self, orders_table):
        """Test HAVING with AVG condition."""
        results = await query_aggregate(
            "orders",
            [("avg", "amount", "avg_amount"), ("count", None, "count")],
            group_by=["user_id"],
            having=[("avg", "amount", "gt", 150)],
            where_conditions=None,
            order_by=[("user_id", "asc")],
            limit=None
        )

        # User 2 avg: (150 + 300.75) / 2 = 225.375 > 150
        assert len(results) >= 1
        assert results[0]["user_id"] == 2

    async def test_having_with_where(self, orders_table):
        """Test HAVING combined with WHERE clause."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total"), ("count", None, "count")],
            group_by=["user_id"],
            having=[("sum", "amount", "gt", 200)],
            where_conditions=[("status", "eq", "completed")],
            order_by=[("user_id", "asc")],
            limit=None
        )

        # WHERE filters first: only completed orders
        # User 1 completed: 100.50 + 200.00 = 300.50 > 200 ✓
        # User 2 completed: 150.00 + 300.75 = 450.75 > 200 ✓
        assert len(results) == 2
        assert results[0]["user_id"] == 1
        assert results[1]["user_id"] == 2

    async def test_having_multiple_conditions(self, orders_table):
        """Test HAVING with multiple conditions."""
        results = await query_aggregate(
            "orders",
            [("sum", "amount", "total"), ("count", None, "count")],
            group_by=["user_id"],
            having=[
                ("sum", "amount", "gt", 300),
                ("count", None, "gte", 2)
            ],
            where_conditions=None,
            order_by=[("user_id", "asc")],
            limit=None
        )

        # User 1: total=350.75 > 300, count=3 >= 2 ✓
        # User 2: total=450.75 > 300, count=2 >= 2 ✓
        # User 3: total=75.00 < 300 ✗
        assert len(results) == 2
        assert results[0]["user_id"] == 1
        assert results[1]["user_id"] == 2

    async def test_having_min_max(self, orders_table):
        """Test HAVING with MIN/MAX aggregate functions."""
        results = await query_aggregate(
            "orders",
            [("max", "amount", "max_amount"), ("count", None, "count")],
            group_by=["user_id"],
            having=[("max", "amount", "gt", 250)],
            where_conditions=None,
            order_by=[("user_id", "asc")],
            limit=None
        )

        # Only User 2 has max amount > 250 (300.75)
        assert len(results) == 1
        assert results[0]["user_id"] == 2
        assert float(results[0]["max_amount"]) == pytest.approx(300.75, rel=0.01)


@pytest.mark.asyncio
class TestErrorHandling:
    """Test error handling and validation."""

    async def test_invalid_table_name(self):
        """Test error on invalid table name."""
        with pytest.raises(Exception):  # Should raise error
            await query_aggregate(
                "nonexistent_table",
                [("count", None, "count")],
                group_by=None,
                having=None,
                where_conditions=None,
                order_by=None,
                limit=None
            )

    async def test_invalid_column_name(self, orders_table):
        """Test error on invalid column name."""
        with pytest.raises(Exception):  # Should raise error
            await query_aggregate(
                "orders",
                [("sum", "nonexistent_column", "total")],
                group_by=None,
                having=None,
                where_conditions=None,
                order_by=None,
                limit=None
            )

    async def test_invalid_aggregate_function(self):
        """Test error on unknown aggregate function."""
        expect(lambda: await query_aggregate().to_raise(ValueError)
                "orders",
                [("invalid_func", "amount", "result")],
                group_by=None,
                having=None,
                where_conditions=None,
                order_by=None,
                limit=None
            )

    async def test_invalid_operator(self, orders_table):
        """Test error on unknown operator."""
        expect(lambda: await query_aggregate().to_raise(ValueError)
                "orders",
                [("count", None, "count")],
                group_by=None,
                having=None,
                where_conditions=[("status", "invalid_op", "completed")],
                order_by=None,
                limit=None
            )
