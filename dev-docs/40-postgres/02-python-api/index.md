---
title: Python API Overview
status: planning
component: data-bridge-python
type: index
---

# Python API Layer Architecture (PostgreSQL)

## Overview

The Python API Layer for PostgreSQL (`python/data_bridge/postgres/`) provides a user-facing ORM for PostgreSQL databases. It follows the Beanie-compatible design patterns established in the MongoDB implementation, adapting them for relational database paradigms while maintaining the same developer experience.

**Key Features**:
- **Beanie-Inspired Design**: Similar API patterns using `Table` instead of `Document`.
- **Type Safety**: Leverages Python type hints and Pydantic for schema definition.
- **Fluent Query API**: Rich DSL for building SQL queries using pythonic expressions.
- **Transaction Support**: First-class support for ACID transactions.
- **Migration System**: Built-in schema migration support.
- **Rust Integration**: All SQL generation and execution happens in Rust for maximum performance.

## Architecture Layers

```mermaid
graph TB
    User[User Application]
    Table[Table Layer<br/>table.py]
    Query[Query Builder<br/>query.py / columns.py]
    Tx[Transaction Context<br/>transaction.py]
    Migrate[Migration API<br/>migration.py]
    Engine[Engine Bridge<br/>_engine.py]
    Rust[Rust Backend<br/>data_bridge_postgres (PyO3)]

    User --> Table
    User --> Query
    User --> Tx
    User --> Migrate
    Table --> Engine
    Query --> Engine
    Tx --> Engine
    Migrate --> Engine
    Engine --> Rust

    style User fill:#e1f5ff
    style Table fill:#fff9c4
    style Query fill:#fff9c4
    style Tx fill:#fff9c4
    style Migrate fill:#fff9c4
    style Engine fill:#ffccbc
    style Rust fill:#ffccbc
```

## Key Classes

### 1. **Table** (Base Model Class)

The `Table` class is the equivalent of MongoDB's `Document`, representing a database table.

```python
from data_bridge.postgres import Table, Column

class User(Table):
    class Settings:
        name = "users"              # Table name
        primary_key = "id"          # Primary key column
        schema = "public"           # PostgreSQL schema
        indexes = [                 # Optional indexes
            {"fields": ["email"], "unique": True}
        ]

    id: int = Column(primary_key=True, auto_increment=True)
    email: str = Column(unique=True, max_length=255)
    name: str = Column(max_length=100)
    age: int = Column(nullable=True)
    created_at: datetime = Column(default="NOW()")
```

**CRUD Methods**:
- `save()` - Insert or update based on primary key presence
- `delete()` - Delete the current instance
- `refresh()` - Reload from database
- `insert()` - Explicit insert (class method)
- `update()` - Update specific fields

**Query Methods** (Class Methods):
- `find(filter)` - Returns QueryBuilder for multiple results
- `find_one(filter)` - Returns single result or None
- `get(pk)` - Get by primary key, raises if not found
- `count(filter)` - Count matching rows

### 2. **ColumnProxy** (Query Expression Builder)

The `ColumnProxy` enables type-safe query construction through operator overloading, similar to `FieldProxy` in MongoDB.

```python
# Comparison operators
User.age == 25                    # WHERE age = 25
User.age != 25                    # WHERE age != 25
User.age > 25                     # WHERE age > 25
User.age >= 25                    # WHERE age >= 25
User.age < 25                     # WHERE age < 25
User.age <= 25                    # WHERE age <= 25

# String operators
User.email.like("%@example.com")       # WHERE email LIKE '%@example.com'
User.email.ilike("%@EXAMPLE.COM")      # WHERE email ILIKE '%@example.com' (case-insensitive)
User.name.startswith("Alice")          # WHERE name LIKE 'Alice%'
User.name.endswith("Smith")            # WHERE name LIKE '%Smith'
User.name.contains("middle")           # WHERE name LIKE '%middle%'

# Collection operators
User.age.in_([25, 30, 35])            # WHERE age IN (25, 30, 35)
User.age.between(25, 35)              # WHERE age BETWEEN 25 AND 35
User.age.is_null()                    # WHERE age IS NULL
User.age.is_not_null()                # WHERE age IS NOT NULL

# Logical operators
(User.age > 25) & (User.name == "Alice")     # WHERE age > 25 AND name = 'Alice'
(User.age < 18) | (User.age > 65)            # WHERE age < 18 OR age > 65
~(User.email.like("%@spam.com"))             # WHERE NOT email LIKE '%@spam.com'
```

### 3. **QueryBuilder** (Fluent Query Interface)

Chainable query construction with automatic SQL generation in Rust.

```python
# Basic queries
users = await User.find(User.age > 25).to_list()
user = await User.find_one(User.email == "alice@example.com")

# Ordering
users = await User.find().order_by(User.created_at).to_list()       # ASC
users = await User.find().order_by(-User.created_at).to_list()      # DESC
users = await User.find().order_by(User.name, -User.age).to_list()  # Multiple

# Pagination
users = await User.find().limit(10).offset(20).to_list()

# Column selection
users = await User.find().select(User.id, User.email).to_list()

# Aggregation
count = await User.find(User.age > 25).count()
exists = await User.find(User.email == "test@example.com").exists()

# Bulk operations
await User.find(User.age < 18).delete()
await User.find(User.status == "inactive").update({"status": "archived"})
```

**Chaining Methods**:
- `order_by(*columns)` - Sort results (use `-column` for DESC)
- `limit(n)` - Limit number of results
- `offset(n)` - Skip n results (for pagination)
- `select(*columns)` - Select specific columns (projection)
- `join(table, on)` - SQL JOIN operations

**Terminal Methods** (Execute Query):
- `to_list()` - Return all results as list
- `first()` - Return first result or None
- `count()` - Return count of matching rows
- `exists()` - Return True if any matching rows exist
- `delete()` - Delete all matching rows
- `update(values)` - Update all matching rows

### 4. **Transaction Context**

First-class transaction support with automatic commit/rollback.

```python
from data_bridge.postgres import pg_transaction

# Automatic transaction management
async with pg_transaction() as tx:
    user = User(email="alice@example.com", name="Alice")
    await user.save()

    # If any exception occurs, transaction is rolled back
    # Otherwise, committed on context exit

# Manual control
tx = await begin_transaction()
try:
    await user.save()
    await tx.commit()
except Exception:
    await tx.rollback()
    raise
```

**Transaction Isolation Levels**:
```python
async with pg_transaction(isolation="SERIALIZABLE") as tx:
    # SERIALIZABLE, REPEATABLE READ, READ COMMITTED, READ UNCOMMITTED
    pass
```

**Savepoints**:
```python
async with pg_transaction() as tx:
    await user1.save()

    savepoint = await tx.savepoint("before_risky_op")
    try:
        await risky_operation()
    except Exception:
        await tx.rollback_to(savepoint)

    await user2.save()
```

### 5. **Migration API**

Built-in schema migration system with version tracking.

```python
from data_bridge.postgres import Migration

class CreateUsersTable(Migration):
    version = "001"

    async def up(self):
        await self.execute("""
            CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                email VARCHAR(255) UNIQUE NOT NULL,
                name VARCHAR(100) NOT NULL,
                created_at TIMESTAMP DEFAULT NOW()
            )
        """)

    async def down(self):
        await self.execute("DROP TABLE users")

# Run migrations
from data_bridge.postgres import run_migrations, get_migration_status

await run_migrations()                    # Run all pending migrations
status = await get_migration_status()     # Get current version and history
```

**Migration Methods**:
- `execute(sql)` - Execute raw SQL
- `create_table(name, columns)` - Helper for CREATE TABLE
- `drop_table(name)` - Helper for DROP TABLE
- `add_column(table, column)` - Add column to existing table
- `drop_column(table, column)` - Remove column
- `create_index(table, columns, unique=False)` - Create index

## Example Usage

### Basic CRUD

```python
from data_bridge.postgres import Table, Column
from datetime import datetime

class User(Table):
    class Settings:
        name = "users"

    id: int = Column(primary_key=True, auto_increment=True)
    email: str = Column(unique=True)
    name: str
    age: int
    created_at: datetime = Column(default="NOW()")

# Create
user = User(email="alice@example.com", name="Alice", age=30)
await user.save()

# Read
user = await User.find_one(User.email == "alice@example.com")
users = await User.find(User.age > 25).order_by(-User.created_at).to_list()

# Update
user.age = 31
await user.save()

# Delete
await user.delete()
```

### Advanced Queries

```python
# Complex filters
adults = await User.find(
    (User.age >= 18) & (User.email.like("%@example.com"))
).to_list()

# Pagination
page_2_users = await User.find().order_by(User.id).limit(20).offset(20).to_list()

# Aggregation
active_count = await User.find(User.status == "active").count()

# Bulk updates
await User.find(User.last_login < cutoff_date).update({
    "status": "inactive"
})
```

### Transactions

```python
async with pg_transaction() as tx:
    # Create user
    user = User(email="bob@example.com", name="Bob")
    await user.save()

    # Create related profile
    profile = Profile(user_id=user.id, bio="Developer")
    await profile.save()

    # Both saved or both rolled back
```

### Migrations

```python
class Migration_001_CreateUsers(Migration):
    version = "001"

    async def up(self):
        await self.create_table("users", {
            "id": "SERIAL PRIMARY KEY",
            "email": "VARCHAR(255) UNIQUE NOT NULL",
            "name": "VARCHAR(100) NOT NULL",
            "created_at": "TIMESTAMP DEFAULT NOW()"
        })
        await self.create_index("users", ["email"], unique=True)

    async def down(self):
        await self.drop_table("users")

# Apply migrations
await run_migrations()
```

## Documentation Structure

### 1. [00-architecture.md](./00-architecture.md)
High-level architectural patterns, including:
- The **Proxy Pattern** for column access and query building.
- The **Transaction Pattern** for ACID guarantees.
- The **Migration Pattern** for schema evolution.
- The **Bridge Pattern** for Rust integration.

### 2. [10-components.md](./10-components.md)
Detailed breakdown of key components:
- **Table**: The core model class and its metaclass magic.
- **ColumnProxy**: How `User.age > 25` generates SQL.
- **QueryBuilder**: Implementation of the fluent API (`.find(...).order_by(...)`).
- **TransactionContext**: ACID transaction management.
- **MigrationRunner**: Schema version tracking and execution.

### 3. [20-data-flows.md](./20-data-flows.md)
Sequence diagrams illustrating:
- **Query Construction**: Python expression → SQL WHERE clause.
- **Row Hydration**: PostgreSQL result → Python Object.
- **Save Lifecycle**: Change detection → Validation → Rust INSERT/UPDATE.
- **Transaction Lifecycle**: Begin → Operations → Commit/Rollback.

### 4. [30-implementation-details.md](./30-implementation-details.md)
Implementation details:
- Metaclass implementation (`TableMeta`).
- Type extraction logic for PostgreSQL types.
- Foreign key and relationship handling.
- Connection pooling strategies.

## Key Differences from MongoDB API

| Feature | MongoDB (Document) | PostgreSQL (Table) |
|---------|-------------------|-------------------|
| Base Class | `Document` | `Table` |
| Schema Definition | Optional (schemaless) | Required (typed columns) |
| Primary Key | `_id` (ObjectId) | Configurable (usually `id`) |
| Relationships | Manual Links/DBRefs | Foreign Keys (planned) |
| Transactions | Limited (replica sets) | Full ACID support |
| Migrations | Not needed | Built-in migration system |
| Indexes | Define in Settings | Define in Settings + migrations |
| Query Operators | MongoDB operators | SQL operators |

## Success Criteria

- ✅ **API Consistency**: Similar patterns to MongoDB API for easy learning.
- 🔄 **Type Safety**: Fully typed for excellent IDE support (VS Code/PyCharm).
- 🔄 **Transaction Support**: ACID guarantees with context managers.
- 🔄 **Migration System**: Safe schema evolution with version tracking.
- 🔄 **Performance**: All SQL generation in Rust, connection pooling.
- 🔄 **Developer Experience**: Clear error messages and intuitive API.

## Performance Considerations

1. **Connection Pooling**: Managed in Rust layer (deadpool-postgres)
2. **Prepared Statements**: Automatic query parameterization
3. **Batch Operations**: Optimized bulk insert/update
4. **GIL Release**: SQL execution happens outside GIL
5. **Zero-Copy**: Direct PostgreSQL binary protocol handling in Rust

## References

- **Python Source**: `python/data_bridge/postgres/` (planned)
- **Rust Source**: `crates/data-bridge-postgres/` (planned)
- **MongoDB API**: Reference for design patterns
- **SQLAlchemy**: Reference for ORM patterns (inspiration, not compatibility)
