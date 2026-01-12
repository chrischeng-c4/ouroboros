# Raw SQL Execution in data-bridge-postgres

The `execute()` function provides direct SQL execution for power users who need features beyond the ORM capabilities.

## Features

- **Parameterized Queries**: Safe parameter binding using `$1, $2, etc.` placeholders
- **Multiple Query Types**: Automatic handling of SELECT, INSERT/UPDATE/DELETE, and DDL
- **Type Conversion**: Automatic conversion between PostgreSQL and Python types
- **Return Type Detection**: Returns appropriate types based on query

## Installation

```python
from data_bridge.postgres import execute
```

## Basic Usage

### SELECT Query

```python
# Returns: List[Dict[str, Any]]
users = await execute(
    "SELECT * FROM users WHERE age > $1 ORDER BY name LIMIT $2",
    [25, 10]
)

for user in users:
    print(f"{user['name']}: {user['age']} years old")
```

### INSERT Query

```python
# Returns: int (number of rows inserted)
count = await execute(
    "INSERT INTO users (name, email, age) VALUES ($1, $2, $3)",
    ["Alice", "alice@example.com", 30]
)
print(f"Inserted {count} row(s)")
```

### UPDATE Query

```python
# Returns: int (number of rows updated)
count = await execute(
    "UPDATE users SET age = age + 1 WHERE name = $1",
    ["Alice"]
)
print(f"Updated {count} row(s)")
```

### DELETE Query

```python
# Returns: int (number of rows deleted)
count = await execute(
    "DELETE FROM users WHERE age < $1",
    [18]
)
print(f"Deleted {count} row(s)")
```

### DDL Commands

```python
# Returns: None
await execute("CREATE INDEX idx_users_age ON users(age)")
await execute("ALTER TABLE users ADD COLUMN last_login TIMESTAMP")
await execute("DROP INDEX IF EXISTS old_index")
```

## Advanced Usage

### Complex Joins

```python
results = await execute("""
    SELECT u.name, u.age, o.product_name, o.quantity
    FROM users u
    INNER JOIN orders o ON u.id = o.user_id
    WHERE u.age > $1 AND o.quantity > $2
    ORDER BY o.created_at DESC
    LIMIT $3
""", [18, 1, 20])
```

### Window Functions

```python
ranked = await execute("""
    SELECT
        name,
        age,
        RANK() OVER (ORDER BY age DESC) as age_rank,
        ROW_NUMBER() OVER (PARTITION BY city ORDER BY age DESC) as city_rank
    FROM users
    WHERE age > $1
    LIMIT $2
""", [20, 10])
```

### Common Table Expressions (CTEs)

```python
results = await execute("""
    WITH recent_orders AS (
        SELECT user_id, COUNT(*) as order_count
        FROM orders
        WHERE created_at > NOW() - INTERVAL '30 days'
        GROUP BY user_id
    )
    SELECT u.name, u.email, COALESCE(ro.order_count, 0) as recent_orders
    FROM users u
    LEFT JOIN recent_orders ro ON u.id = ro.user_id
    WHERE u.age > $1
    ORDER BY recent_orders DESC
    LIMIT $2
""", [21, 10])
```

### Aggregate Queries

```python
stats = await execute("""
    SELECT
        COUNT(*) as total_users,
        AVG(age) as avg_age,
        MIN(age) as min_age,
        MAX(age) as max_age
    FROM users
    WHERE age > $1
""", [0])

if stats:
    print(f"Total: {stats[0]['total_users']}")
    print(f"Average age: {stats[0]['avg_age']:.1f}")
```

## Parameter Types

The `execute()` function supports all standard Python types:

| Python Type | PostgreSQL Type | Example |
|-------------|-----------------|---------|
| `None` | NULL | `[None]` |
| `bool` | BOOLEAN | `[True, False]` |
| `int` | INTEGER/BIGINT | `[42, 9999999]` |
| `float` | REAL/DOUBLE | `[3.14, 2.718]` |
| `str` | TEXT/VARCHAR | `["hello"]` |
| `bytes` | BYTEA | `[b"data"]` |
| `list` | ARRAY | `[[1, 2, 3]]` |
| `dict` | JSON/JSONB | `[{"key": "value"}]` |

### NULL Handling

```python
# Insert with NULL value
count = await execute(
    "INSERT INTO users (name, email, age) VALUES ($1, $2, $3)",
    ["Bob", None, 28]  # email is NULL
)

# Query for NULL values
users = await execute(
    "SELECT * FROM users WHERE email IS NULL"
)
```

## Security

### SQL Injection Prevention

**ALWAYS use parameterized queries** to prevent SQL injection attacks:

```python
# ✅ SAFE: Parameterized query
user_input = "admin'; DROP TABLE users; --"
results = await execute(
    "SELECT * FROM users WHERE name = $1",
    [user_input]
)
# This safely searches for the literal string, doesn't execute the DROP

# ❌ DANGEROUS: String concatenation (NEVER DO THIS!)
results = await execute(f"SELECT * FROM users WHERE name = '{user_input}'")
# This would execute the DROP TABLE command!
```

### Parameter Placeholders

Use PostgreSQL-style numbered placeholders (`$1, $2, $3, ...`):

```python
# Correct
await execute(
    "INSERT INTO users (name, age, city) VALUES ($1, $2, $3)",
    ["Alice", 30, "NYC"]
)

# Incorrect (this won't work)
await execute(
    "INSERT INTO users (name, age, city) VALUES (?, ?, ?)",
    ["Alice", 30, "NYC"]
)
```

## Return Types

The function automatically detects the query type and returns appropriate values:

| Query Type | Return Type | Description |
|------------|-------------|-------------|
| SELECT | `List[Dict[str, Any]]` | List of rows as dictionaries |
| INSERT/UPDATE/DELETE | `int` | Number of affected rows |
| DDL (CREATE/ALTER/DROP) | `None` | No return value |

## Error Handling

```python
try:
    results = await execute("SELECT * FROM non_existent_table")
except RuntimeError as e:
    print(f"Query failed: {e}")
```

## Performance Considerations

### Prepared Statements

The underlying SQLx driver automatically uses prepared statements for parameterized queries, providing:
- **Protection against SQL injection**
- **Improved performance** for repeated queries
- **Automatic type conversion**

### Large Result Sets

For queries returning many rows, results are fetched in batches internally, but the entire result set is returned as a list. For very large datasets, consider:

```python
# Use LIMIT and OFFSET for pagination
page_size = 100
offset = 0

while True:
    batch = await execute(
        "SELECT * FROM large_table ORDER BY id LIMIT $1 OFFSET $2",
        [page_size, offset]
    )
    if not batch:
        break

    # Process batch
    for row in batch:
        process(row)

    offset += page_size
```

## When to Use `execute()`

Use `execute()` when you need:

- ✅ Complex joins across multiple tables
- ✅ Window functions (RANK, ROW_NUMBER, etc.)
- ✅ CTEs (WITH queries)
- ✅ Database-specific functions
- ✅ Custom aggregations
- ✅ DDL operations (CREATE, ALTER, DROP)
- ✅ Raw SQL performance optimization

Use the ORM when you need:

- ✅ Simple CRUD operations
- ✅ Type safety and validation
- ✅ Automatic relationships
- ✅ Schema migrations
- ✅ Model-based queries

## Examples

See `/examples/postgres_raw_sql.py` for comprehensive examples.

## Comparison with ORM

```python
# ORM approach
users = await User.find(User.age > 25).limit(10)

# Raw SQL approach
users = await execute(
    "SELECT * FROM users WHERE age > $1 LIMIT $2",
    [25, 10]
)
```

Both approaches are valid - choose based on your needs:
- **ORM**: Better for type safety, validation, and simple queries
- **Raw SQL**: Better for complex queries, performance-critical code, and database-specific features

## Related Functions

- `init()` - Initialize PostgreSQL connection
- `close()` - Close PostgreSQL connection
- `begin_transaction()` - Start a transaction (can be used with `execute()`)

## Complete Example

```python
import asyncio
from data_bridge.postgres import init, close, execute

async def main():
    # Initialize connection
    await init("postgresql://user:pass@localhost:5432/mydb")

    try:
        # Create table
        await execute("""
            CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                age INTEGER,
                email TEXT
            )
        """)

        # Insert data
        count = await execute(
            "INSERT INTO users (name, age, email) VALUES ($1, $2, $3)",
            ["Alice", 30, "alice@example.com"]
        )
        print(f"Inserted {count} row(s)")

        # Query data
        users = await execute(
            "SELECT * FROM users WHERE age > $1",
            [25]
        )
        for user in users:
            print(f"{user['name']}: {user['age']}")

    finally:
        await close()

if __name__ == "__main__":
    asyncio.run(main())
```
