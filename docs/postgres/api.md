# API Reference

## Models

### `Table`

Base class for all PostgreSQL models.

**Methods:**
-   `save()`: Save the current instance (insert or update).
-   `delete()`: Delete the current instance.
-   `refresh()`: Reload data from the database.
-   `to_dict()`: Convert instance to dictionary.

**Class Methods:**
-   `find(filter)`: Start a query builder.
-   `find_one(filter)`: Find a single record.
-   `get(id)`: Get record by primary key.

### `Column`

Descriptor for table columns.

**Parameters:**
-   `default`: Default value.
-   `default_factory`: Callable to generate default value.
-   `unique`: Boolean, shorthand for unique index.

### `Settings`

Inner class for model configuration.

**Attributes:**
-   `table_name`: Custom table name.
-   `schema`: Database schema (default "public").
-   `indexes`: List of index definitions.

## Querying

### `QueryBuilder`

Fluent interface for building queries.

**Methods:**
-   `order_by(column)`: Sort results.
-   `limit(n)`: Limit number of results.
-   `offset(n)`: Skip results.
-   `select(*columns)`: Select specific columns.
-   `group_by(*columns)`: Group results.
-   `having(aggregate, column, operator, value)`: Filter groups.
-   `to_list()`: Execute and return list of objects.
-   `first()`: Execute and return first object or None.
-   `count()`: Execute and return count.
-   `exists()`: Check if any rows match.
-   `aggregate()`: Execute aggregation query.
-   `with_cte(name, query)`: Add a Common Table Expression.
-   `from_cte(name, query)`: Create a query from a CTE (classmethod).

**Aggregations:**
-   `sum(column, alias)`
-   `avg(column, alias)`
-   `min(column, alias)`
-   `max(column, alias)`
-   `count_agg(alias)`

## Connection & Session

### `init()`
Initialize the connection pool.
```python
await init(connection_string, min_connections=1, max_connections=10)
```

### `close()`
Close the connection pool.

### `is_connected()`
Check if the connection pool is initialized.

### `Session`
Manages a database transaction/unit of work.

**Methods:**
-   `add(obj)`: Add object to session.
-   `add_all(objects)`: Add multiple objects.
-   `delete(obj)`: Mark object for deletion.
-   `get(model, id, with_for_update=False)`: Get object by ID.
-   `commit()`: Save all changes.
-   `rollback()`: Discard all changes.
-   `flush()`: Push changes to DB without committing transaction.
-   `close()`: Close the session.
-   `expunge(obj)`: Remove object from session.
-   `expunge_all()`: Remove all objects.

### `execute()`
Run raw SQL.
```python
await execute("SELECT * FROM table WHERE id = $1", [1])
```

### `query_aggregate()`
Execute a raw aggregation query.

### `query_with_cte()`
Execute a raw query with CTEs.

## CRUD Operations

-   `insert_one(table, data)`: Insert a single row.
-   `insert_many(table, rows)`: Insert multiple rows.
-   `upsert_one(table, keys, data)`: Insert or update a single row.
-   `upsert_many(table, keys, rows)`: Insert or update multiple rows.

## Introspection

-   `inspect_table(table_name)`: Get full table schema.
-   `get_indexes(table_name)`: Get list of indexes.
-   `get_columns(table_name)`: Get list of columns.
-   `list_tables(schema)`: List all tables.

## Optimization

-   `fetch_many_with_relations()`: Eager load relations in a single query.
-   `fetch_one_with_relations()`: Fetch single object with relations.

## Other Modules

-   `migrations`: Database migration tools.
-   `relationships`: Relationship definitions.
-   `transactions`: Transaction management (`pg_transaction`).
-   `events`: Event listeners (`before_insert`, etc.).
-   `validation`: Data validation tools.
