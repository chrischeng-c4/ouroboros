# Aggregation

Data Bridge PostgreSQL supports powerful aggregation capabilities, including standard aggregate functions, GROUP BY clauses, window functions, and Common Table Expressions (CTEs).

## Aggregate Functions

You can perform standard aggregations directly using the `QueryBuilder`.

```python
from data_bridge.postgres import Table

class Order(Table):
    amount: float
    user_id: int
    status: str

# Sum
total_sales = await Order.find(Order.status == "completed").sum(Order.amount, "total").aggregate()

# Average
avg_order = await Order.find().avg(Order.amount, "average_value").aggregate()

# Count
count = await Order.find(Order.user_id == 1).count_agg("order_count").aggregate()
```

The `.aggregate()` method executes the query and returns a list of dictionaries containing the results.

## Group By and Having

You can group results by one or more columns and filter groups using `HAVING`.

```python
# Total sales per user
results = await Order.find() \
    .sum(Order.amount, "total_sales") \
    .group_by("user_id") \
    .aggregate()

# Filter groups: Users with total sales > 1000
results = await Order.find() \
    .sum(Order.amount, "total_sales") \
    .group_by("user_id") \
    .having_sum(Order.amount, ">", 1000) \
    .aggregate()
```

## Window Functions

Window functions allow you to perform calculations across a set of table rows that are somehow related to the current row.

```python
# Rank orders by amount
results = await Order.find() \
    .select(Order.id, Order.amount) \
    .rank("amount_rank") \
    .aggregate()

# Window aggregation
results = await Order.find() \
    .select(Order.id, Order.user_id, Order.amount) \
    .window_sum(Order.amount, "running_total") \
    .aggregate()
```

Supported window functions include:
- `row_number()`
- `rank()`
- `lag()`
- `lead()`
- `window_sum()`
- `window_avg()`

## Common Table Expressions (CTEs)

CTEs allow you to create temporary result sets that can be referenced within the main query. Currently, CTEs are supported only within `aggregate()` queries.

```python
from data_bridge.postgres import QueryBuilder

# Define a CTE for high-value orders
high_value = Order.find(Order.amount > 1000)

# Use the CTE in a main query via QueryBuilder.from_cte
results = await QueryBuilder.from_cte("high_value_orders", high_value) \
    .sum("amount", "total") \
    .aggregate()
```

You can also use raw SQL for CTEs if needed:

```python
results = await Order.find() \
    .with_cte_raw("my_cte", "SELECT * FROM orders WHERE amount > $1", [500]) \
    .aggregate()
```

