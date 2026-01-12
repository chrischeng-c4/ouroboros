# Indexes

Data Bridge PostgreSQL allows you to inspect existing indexes on your tables.

*(Note: Index creation is currently handled via external migrations or SQL scripts, not directly through the Python model definitions.)*

## Introspection

You can inspect existing indexes on a table using the `inspect_table` or `get_indexes` functions. This is useful for verifying your schema against the database.

```python
from data_bridge.postgres import get_indexes, inspect_table

# Get simple list of indexes
indexes = await get_indexes("users")
for idx in indexes:
    print(f"Name: {idx['name']}, Columns: {idx['columns']}, Unique: {idx['is_unique']}")

# Get full table inspection
schema_info = await inspect_table("users")
print(f"Table {schema_info['name']} has {len(schema_info['indexes'])} indexes.")
```
