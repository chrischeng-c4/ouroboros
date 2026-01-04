"""
Integration tests for PostgreSQL window functions.

Tests cover:
- ROW_NUMBER() OVER (ORDER BY ...)
- RANK() and DENSE_RANK() with PARTITION BY
- LAG() and LEAD() window functions
- FIRST_VALUE() and LAST_VALUE()
- Aggregate window functions (SUM, AVG) with PARTITION BY
- Window functions with frame specifications
- Window functions combined with WHERE and GROUP BY
- Multiple window functions in the same query
"""

import pytest
from decimal import Decimal
from datetime import date
from data_bridge.postgres import execute, insert_one, Table, Column
from data_bridge.postgres.query import WindowSpec


@pytest.fixture
async def sales_table():
    """
    Create and populate a sales table for window function testing.

    Table structure:
    - id: SERIAL PRIMARY KEY
    - salesperson: VARCHAR(100)
    - region: VARCHAR(50)
    - amount: DECIMAL(10, 2)
    - sale_date: DATE
    """
    # Create sales table
    await execute(
        """
        CREATE TABLE IF NOT EXISTS sales (
            id SERIAL PRIMARY KEY,
            salesperson VARCHAR(100) NOT NULL,
            region VARCHAR(50) NOT NULL,
            amount DECIMAL(10, 2) NOT NULL,
            sale_date DATE NOT NULL
        )
        """
    )

    # Insert sample data with multiple salespeople and regions
    test_data = [
        # North region
        {"salesperson": "Alice", "region": "North", "amount": 1000.00, "sale_date": "2024-01-15"},
        {"salesperson": "Alice", "region": "North", "amount": 1500.00, "sale_date": "2024-02-10"},
        {"salesperson": "Alice", "region": "North", "amount": 2000.00, "sale_date": "2024-03-05"},
        {"salesperson": "Bob", "region": "North", "amount": 800.00, "sale_date": "2024-01-20"},
        {"salesperson": "Bob", "region": "North", "amount": 1200.00, "sale_date": "2024-02-15"},
        {"salesperson": "Bob", "region": "North", "amount": 900.00, "sale_date": "2024-03-10"},

        # South region
        {"salesperson": "Charlie", "region": "South", "amount": 2200.00, "sale_date": "2024-01-10"},
        {"salesperson": "Charlie", "region": "South", "amount": 1800.00, "sale_date": "2024-02-05"},
        {"salesperson": "Charlie", "region": "South", "amount": 2500.00, "sale_date": "2024-03-01"},
        {"salesperson": "Diana", "region": "South", "amount": 1100.00, "sale_date": "2024-01-25"},
        {"salesperson": "Diana", "region": "South", "amount": 1400.00, "sale_date": "2024-02-20"},
        {"salesperson": "Diana", "region": "South", "amount": 1600.00, "sale_date": "2024-03-15"},

        # East region
        {"salesperson": "Eve", "region": "East", "amount": 1700.00, "sale_date": "2024-01-12"},
        {"salesperson": "Eve", "region": "East", "amount": 1900.00, "sale_date": "2024-02-08"},
        {"salesperson": "Eve", "region": "East", "amount": 2100.00, "sale_date": "2024-03-03"},
        {"salesperson": "Frank", "region": "East", "amount": 1300.00, "sale_date": "2024-01-18"},
        {"salesperson": "Frank", "region": "East", "amount": 1500.00, "sale_date": "2024-02-12"},
        {"salesperson": "Frank", "region": "East", "amount": 1700.00, "sale_date": "2024-03-08"},
    ]

    for data in test_data:
        await insert_one("sales", data)

    yield

    # Cleanup handled by cleanup_tables fixture


@pytest.mark.asyncio
class TestRowNumberWindow:
    """Test ROW_NUMBER() window function."""

    async def test_row_number_order_by_amount(self, sales_table):
        """Test ROW_NUMBER() with ORDER BY to rank sales by amount."""
        # Execute raw SQL with window function
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                ROW_NUMBER() OVER (ORDER BY amount DESC) as rank
            FROM sales
            ORDER BY amount DESC
            LIMIT 5
            """
        )

        # Verify top 5 sales have sequential row numbers
        assert len(results) == 5
        assert results[0]["rank"] == 1
        assert results[1]["rank"] == 2
        assert results[2]["rank"] == 3
        assert results[3]["rank"] == 4
        assert results[4]["rank"] == 5

        # Verify highest sale
        assert float(results[0]["amount"]) == 2500.00
        assert results[0]["salesperson"] == "Charlie"

    async def test_row_number_partition_by_region(self, sales_table):
        """Test ROW_NUMBER() with PARTITION BY region."""
        # Rank sales within each region
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                ROW_NUMBER() OVER (PARTITION BY region ORDER BY amount DESC) as region_rank
            FROM sales
            ORDER BY region, region_rank
            """
        )

        # Group results by region
        by_region = {}
        for row in results:
            region = row["region"]
            if region not in by_region:
                by_region[region] = []
            by_region[region].append(row)

        # Verify each region has sequential rankings
        for region, rows in by_region.items():
            for i, row in enumerate(rows, start=1):
                assert row["region_rank"] == i

        # Verify East region top sale
        east_sales = by_region["East"]
        assert float(east_sales[0]["amount"]) == 2100.00
        assert east_sales[0]["salesperson"] == "Eve"


@pytest.mark.asyncio
class TestRankDenseRankWindow:
    """Test RANK() and DENSE_RANK() window functions."""

    async def test_rank_with_partition(self, sales_table):
        """Test RANK() with PARTITION BY and ORDER BY."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                RANK() OVER (PARTITION BY region ORDER BY amount DESC) as sales_rank
            FROM sales
            WHERE region = 'North'
            ORDER BY sales_rank
            """
        )

        # Verify North region rankings
        assert len(results) == 6  # 6 sales in North region

        # Alice's top sale should be rank 1
        alice_top = [r for r in results if r["salesperson"] == "Alice" and r["sales_rank"] == 1]
        assert len(alice_top) == 1
        assert float(alice_top[0]["amount"]) == 2000.00

    async def test_dense_rank_window(self, sales_table):
        """Test DENSE_RANK() window function."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                DENSE_RANK() OVER (ORDER BY amount DESC) as dense_rank
            FROM sales
            ORDER BY dense_rank
            LIMIT 10
            """
        )

        # Verify dense rankings are sequential (no gaps)
        assert len(results) == 10
        ranks = [r["dense_rank"] for r in results]

        # Dense rank should not have gaps
        assert ranks[0] == 1
        for i in range(1, len(ranks)):
            assert ranks[i] <= ranks[i-1] + 1


@pytest.mark.asyncio
class TestLagLeadWindow:
    """Test LAG() and LEAD() window functions."""

    async def test_lag_previous_sale_amount(self, sales_table):
        """Test LAG() to get previous sale amount for each salesperson."""
        results = await execute(
            """
            SELECT
                salesperson,
                sale_date,
                amount,
                LAG(amount) OVER (PARTITION BY salesperson ORDER BY sale_date) as prev_amount
            FROM sales
            WHERE salesperson = 'Alice'
            ORDER BY sale_date
            """
        )

        # Alice has 3 sales
        assert len(results) == 3

        # First sale has no previous (NULL)
        assert results[0]["prev_amount"] is None
        assert float(results[0]["amount"]) == 1000.00

        # Second sale's previous is first sale
        assert results[1]["prev_amount"] is not None
        assert float(results[1]["prev_amount"]) == 1000.00
        assert float(results[1]["amount"]) == 1500.00

        # Third sale's previous is second sale
        assert results[2]["prev_amount"] is not None
        assert float(results[2]["prev_amount"]) == 1500.00
        assert float(results[2]["amount"]) == 2000.00

    async def test_lead_next_sale_amount(self, sales_table):
        """Test LEAD() to get next sale amount."""
        results = await execute(
            """
            SELECT
                salesperson,
                sale_date,
                amount,
                LEAD(amount) OVER (PARTITION BY salesperson ORDER BY sale_date) as next_amount
            FROM sales
            WHERE salesperson = 'Bob'
            ORDER BY sale_date
            """
        )

        # Bob has 3 sales
        assert len(results) == 3

        # First sale's next is second sale
        assert results[0]["next_amount"] is not None
        assert float(results[0]["next_amount"]) == 1200.00

        # Second sale's next is third sale
        assert results[1]["next_amount"] is not None
        assert float(results[1]["next_amount"]) == 900.00

        # Last sale has no next (NULL)
        assert results[2]["next_amount"] is None

    async def test_lag_with_offset_and_default(self, sales_table):
        """Test LAG() with custom offset and default value."""
        results = await execute(
            """
            SELECT
                salesperson,
                sale_date,
                amount,
                LAG(amount, 2, 0) OVER (PARTITION BY salesperson ORDER BY sale_date) as prev_prev_amount
            FROM sales
            WHERE salesperson = 'Charlie'
            ORDER BY sale_date
            """
        )

        # Charlie has 3 sales
        assert len(results) == 3

        # First two sales have no prev-prev (default to 0)
        assert float(results[0]["prev_prev_amount"]) == 0.00
        assert float(results[1]["prev_prev_amount"]) == 0.00

        # Third sale's prev-prev is first sale
        assert float(results[2]["prev_prev_amount"]) == 2200.00


@pytest.mark.asyncio
class TestFirstLastValueWindow:
    """Test FIRST_VALUE() and LAST_VALUE() window functions."""

    async def test_first_value_by_region(self, sales_table):
        """Test FIRST_VALUE() to get first sale in each region."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                sale_date,
                amount,
                FIRST_VALUE(amount) OVER (
                    PARTITION BY region
                    ORDER BY sale_date
                    ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
                ) as first_sale_amount
            FROM sales
            WHERE region = 'South'
            ORDER BY sale_date
            """
        )

        # All South region sales should have same first_sale_amount
        assert len(results) == 6
        first_amounts = {float(r["first_sale_amount"]) for r in results}
        assert len(first_amounts) == 1  # Should all be the same

        # First sale in South region was Charlie on 2024-01-10
        assert 2200.00 in first_amounts

    async def test_last_value_by_region(self, sales_table):
        """Test LAST_VALUE() to get last sale in each region."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                sale_date,
                amount,
                LAST_VALUE(amount) OVER (
                    PARTITION BY region
                    ORDER BY sale_date
                    ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
                ) as last_sale_amount
            FROM sales
            WHERE region = 'East'
            ORDER BY sale_date
            """
        )

        # All East region sales should have same last_sale_amount
        assert len(results) == 6
        last_amounts = {float(r["last_sale_amount"]) for r in results}
        assert len(last_amounts) == 1  # Should all be the same

        # Last sale in East region was Frank on 2024-03-08
        assert 1700.00 in last_amounts


@pytest.mark.asyncio
class TestAggregateWindowFunctions:
    """Test aggregate functions used as window functions."""

    async def test_sum_over_partition(self, sales_table):
        """Test SUM() OVER (PARTITION BY ...) for running totals."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                SUM(amount) OVER (PARTITION BY region) as region_total
            FROM sales
            WHERE region = 'North'
            ORDER BY salesperson, amount
            """
        )

        # All North region sales should have same total
        assert len(results) == 6
        totals = {float(r["region_total"]) for r in results}
        assert len(totals) == 1  # Should all be the same

        # North region total: Alice (1000+1500+2000) + Bob (800+1200+900) = 7400
        assert 7400.00 in totals

    async def test_avg_over_with_order(self, sales_table):
        """Test AVG() OVER with ORDER BY for moving average."""
        results = await execute(
            """
            SELECT
                salesperson,
                sale_date,
                amount,
                AVG(amount) OVER (
                    PARTITION BY salesperson
                    ORDER BY sale_date
                    ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                ) as cumulative_avg
            FROM sales
            WHERE salesperson = 'Alice'
            ORDER BY sale_date
            """
        )

        # Alice has 3 sales: 1000, 1500, 2000
        assert len(results) == 3

        # First sale: avg of 1000 = 1000
        assert float(results[0]["cumulative_avg"]) == pytest.approx(1000.00, rel=0.01)

        # Second sale: avg of (1000 + 1500) / 2 = 1250
        assert float(results[1]["cumulative_avg"]) == pytest.approx(1250.00, rel=0.01)

        # Third sale: avg of (1000 + 1500 + 2000) / 3 = 1500
        assert float(results[2]["cumulative_avg"]) == pytest.approx(1500.00, rel=0.01)

    async def test_sum_with_frame_specification(self, sales_table):
        """Test SUM() OVER with custom frame specification."""
        results = await execute(
            """
            SELECT
                salesperson,
                sale_date,
                amount,
                SUM(amount) OVER (
                    PARTITION BY salesperson
                    ORDER BY sale_date
                    ROWS BETWEEN 1 PRECEDING AND CURRENT ROW
                ) as two_sale_total
            FROM sales
            WHERE salesperson = 'Bob'
            ORDER BY sale_date
            """
        )

        # Bob has 3 sales: 800, 1200, 900
        assert len(results) == 3

        # First sale: only current row = 800
        assert float(results[0]["two_sale_total"]) == 800.00

        # Second sale: previous + current = 800 + 1200 = 2000
        assert float(results[1]["two_sale_total"]) == 2000.00

        # Third sale: previous + current = 1200 + 900 = 2100
        assert float(results[2]["two_sale_total"]) == 2100.00


@pytest.mark.asyncio
class TestWindowWithWhereAndGroupBy:
    """Test window functions combined with WHERE and GROUP BY."""

    async def test_window_with_where_condition(self, sales_table):
        """Test window function with WHERE clause filtering."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                ROW_NUMBER() OVER (ORDER BY amount DESC) as rank
            FROM sales
            WHERE amount > 1500
            ORDER BY rank
            """
        )

        # Should only include sales > 1500
        assert all(float(r["amount"]) > 1500 for r in results)

        # Rankings should be sequential
        for i, row in enumerate(results, start=1):
            assert row["rank"] == i

    async def test_window_with_having(self, sales_table):
        """Test window function in subquery with GROUP BY and HAVING."""
        results = await execute(
            """
            SELECT
                region,
                total_sales,
                RANK() OVER (ORDER BY total_sales DESC) as region_rank
            FROM (
                SELECT
                    region,
                    SUM(amount) as total_sales
                FROM sales
                GROUP BY region
                HAVING SUM(amount) > 5000
            ) regional_totals
            ORDER BY region_rank
            """
        )

        # Should only include regions with total > 5000
        assert all(float(r["total_sales"]) > 5000 for r in results)

        # Rankings should be sequential
        for i, row in enumerate(results, start=1):
            assert row["region_rank"] == i

    async def test_window_aggregates_by_salesperson(self, sales_table):
        """Test combining GROUP BY with window functions."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                total_amount,
                AVG(total_amount) OVER (PARTITION BY region) as region_avg
            FROM (
                SELECT
                    salesperson,
                    region,
                    SUM(amount) as total_amount
                FROM sales
                GROUP BY salesperson, region
            ) salesperson_totals
            ORDER BY region, salesperson
            """
        )

        # Group by region to verify averages
        by_region = {}
        for row in results:
            region = row["region"]
            if region not in by_region:
                by_region[region] = []
            by_region[region].append(row)

        # Verify each region has consistent average
        for region, rows in by_region.items():
            region_avgs = {float(r["region_avg"]) for r in rows}
            assert len(region_avgs) == 1  # All should have same avg


@pytest.mark.asyncio
class TestMultipleWindowFunctions:
    """Test multiple window functions in the same query."""

    async def test_multiple_window_functions_same_query(self, sales_table):
        """Test combining multiple window functions."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                sale_date,
                ROW_NUMBER() OVER (PARTITION BY region ORDER BY amount DESC) as region_rank,
                RANK() OVER (ORDER BY amount DESC) as overall_rank,
                LAG(amount) OVER (PARTITION BY salesperson ORDER BY sale_date) as prev_amount,
                SUM(amount) OVER (PARTITION BY region) as region_total
            FROM sales
            WHERE region IN ('North', 'South')
            ORDER BY region, region_rank
            """
        )

        # Verify all window functions are present
        assert all("region_rank" in r for r in results)
        assert all("overall_rank" in r for r in results)
        assert all("region_total" in r for r in results)

        # North and South regions combined: 12 sales
        assert len(results) == 12

        # Verify region_rank is sequential within each region
        by_region = {}
        for row in results:
            region = row["region"]
            if region not in by_region:
                by_region[region] = []
            by_region[region].append(row)

        for region, rows in by_region.items():
            for i, row in enumerate(rows, start=1):
                assert row["region_rank"] == i

    async def test_multiple_partitions_and_orders(self, sales_table):
        """Test window functions with different PARTITION BY and ORDER BY."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                sale_date,
                ROW_NUMBER() OVER (PARTITION BY salesperson ORDER BY sale_date) as sale_sequence,
                DENSE_RANK() OVER (PARTITION BY region ORDER BY amount DESC) as region_amount_rank,
                FIRST_VALUE(sale_date) OVER (
                    PARTITION BY salesperson
                    ORDER BY sale_date
                    ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
                ) as first_sale_date,
                AVG(amount) OVER (PARTITION BY region) as region_avg_amount
            FROM sales
            WHERE salesperson IN ('Alice', 'Bob', 'Charlie')
            ORDER BY salesperson, sale_date
            """
        )

        # Alice (3) + Bob (3) + Charlie (3) = 9 sales
        assert len(results) == 9

        # Verify sale_sequence is sequential for each salesperson
        by_salesperson = {}
        for row in results:
            sp = row["salesperson"]
            if sp not in by_salesperson:
                by_salesperson[sp] = []
            by_salesperson[sp].append(row)

        for sp, rows in by_salesperson.items():
            for i, row in enumerate(rows, start=1):
                assert row["sale_sequence"] == i
                # Verify all sales by same person have same first_sale_date
                if i > 1:
                    assert row["first_sale_date"] == rows[0]["first_sale_date"]


@pytest.mark.asyncio
class TestWindowEdgeCases:
    """Test edge cases and special scenarios for window functions."""

    async def test_window_with_empty_partition(self, sales_table):
        """Test window function when filtering leaves some partitions empty."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                ROW_NUMBER() OVER (PARTITION BY region ORDER BY amount DESC) as rank
            FROM sales
            WHERE amount > 2000
            ORDER BY region, rank
            """
        )

        # Only sales > 2000: Charlie (2200, 2500), Alice (2000 - excluded), Eve (2100)
        # Actually Alice's 2000 is not > 2000, so excluded
        assert len(results) >= 3

        # Verify all amounts are > 2000
        assert all(float(r["amount"]) > 2000 for r in results)

    async def test_window_with_null_handling(self, sales_table):
        """Test window function with LAG default for NULL values."""
        # Insert a salesperson with only one sale
        await insert_one("sales", {
            "salesperson": "Grace",
            "region": "West",
            "amount": 3000.00,
            "sale_date": "2024-01-05"
        })

        results = await execute(
            """
            SELECT
                salesperson,
                sale_date,
                amount,
                LAG(amount, 1, -1) OVER (PARTITION BY salesperson ORDER BY sale_date) as prev_amount
            FROM sales
            WHERE salesperson = 'Grace'
            ORDER BY sale_date
            """
        )

        # Grace has only one sale, so prev_amount should be default (-1)
        assert len(results) == 1
        assert float(results[0]["prev_amount"]) == -1.00

    async def test_window_with_limit(self, sales_table):
        """Test window function with LIMIT clause."""
        results = await execute(
            """
            SELECT
                salesperson,
                region,
                amount,
                ROW_NUMBER() OVER (ORDER BY amount DESC) as overall_rank
            FROM sales
            ORDER BY overall_rank
            LIMIT 3
            """
        )

        # Should return top 3 sales
        assert len(results) == 3
        assert results[0]["overall_rank"] == 1
        assert results[1]["overall_rank"] == 2
        assert results[2]["overall_rank"] == 3

        # Verify descending order by amount
        assert float(results[0]["amount"]) >= float(results[1]["amount"])
        assert float(results[1]["amount"]) >= float(results[2]["amount"])
