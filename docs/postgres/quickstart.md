# PostgreSQL Quickstart

This guide gets you started with the `data-bridge` PostgreSQL ORM. It's designed for high-performance async applications, offering a familiar API for developers coming from SQLAlchemy or Beanie.

## Installation

Install `data-bridge` with PostgreSQL support:

```bash
pip install "data-bridge[postgres]"
```

## Basic Setup

### 1. Define a Model

Models are defined by inheriting from `Table` and using type hints.

```python
from data_bridge.postgres import Table, Column
from datetime import datetime

class User(Table):
    # Columns are defined with type hints
    email: str = Column(unique=True)
    name: str
    is_active: bool = True
    created_at: datetime = Column(default_factory=datetime.now)

    class Settings:
        # Optional configuration
        table_name = "users"
        schema = "public"
```

### 2. Connect to Database

Initialize the connection pool at your application startup.

```python
import asyncio
from data_bridge.postgres import init

async def main():
    # Connect to PostgreSQL
    await init(
        database_url="postgresql://user:pass@localhost/dbname",
        min_size=2,
        max_size=10
    )

    # ... application code ...

if __name__ == "__main__":
    asyncio.run(main())
```

## CRUD Operations

### Create

Create and save a new record:

```python
user = User(email="alice@example.com", name="Alice")
await user.save()

print(user.id)  # Access the auto-generated ID
```

### Read

Find records using the fluent query API:

```python
# Find by primary key
user = await User.get(1)

# Find one by criteria
user = await User.find_one(User.email == "alice@example.com")

# Find many with filtering
active_users = await User.find(
    User.is_active == True
).to_list()
```

### Update

Modify attributes and save:

```python
user = await User.find_one(User.email == "alice@example.com")
if user:
    user.name = "Alice Cooper"
    await user.save()
```

### Delete

Delete a record:

```python
user = await User.get(1)
if user:
    await user.delete()
```

## Next Steps

- **[Tables & Columns](guides/tables_and_columns.md)**: Learn about types, constraints, and computed columns.
- **[Querying](guides/querying.md)**: Master the QueryBuilder, filtering, and joins.
- **[Validation](guides/validation.md)**: Add data integrity checks to your models.
- **[Events](guides/events.md)**: Hook into lifecycle events.
