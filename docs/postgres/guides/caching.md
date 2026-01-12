# Caching & Performance

Data Bridge PostgreSQL includes several features to optimize database performance, including connection pooling, prepared statements, and efficient query patterns.

## Connection Pooling

The library uses a high-performance connection pool managed by the underlying Rust engine (SQLx (PgPool/PgPoolOptions)). This eliminates the overhead of establishing a new connection for every query.

### Configuration

You configure the pool when initializing the library:

```python
from data_bridge.postgres import init

await init(
    "postgres://user:pass@localhost/db",
    min_connections=5,   # Keep at least 5 connections open
    max_connections=20   # Allow up to 20 concurrent connections
)
```

- **min_connections**: The pool will maintain this many idle connections, ready for immediate use.
- **max_connections**: The hard limit on open connections. Requests exceeding this will wait for a connection to become available.

## Eager Loading (Reducing N+1 Queries)

To avoid the N+1 query problem (fetching a list of items and then executing a separate query for each item's related data), use eager loading.

### `fetch_many_with_relations`

This function performs a single query with JOINs to fetch main records and their related data.

```python
from data_bridge.postgres import fetch_many_with_relations

users = await fetch_many_with_relations(
    "users",
    relations=[
        {
            "name": "posts",
            "table": "posts",
            "foreign_key": "user_id",
            "join_type": "left"
        }
    ],
    filter={"active": True}
)

# Access data without extra queries
for user in users:
    print(f"User {user['name']} has {len(user['posts'])} posts")
```

## Result Optimization

For read-heavy operations where you don't need full model instances, you can use aggregation queries or raw selects to fetch only the data you need.

```python
# Fetch specific columns (returns Table instances)
users = await User.find().select("id", "email").to_list()
for user in users:
    print(user.email)
```
