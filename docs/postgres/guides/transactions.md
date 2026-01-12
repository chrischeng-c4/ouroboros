# PostgreSQL Transaction Support

## Overview

The data-bridge PostgreSQL module provides full transaction support with ACID guarantees. Transactions ensure that database operations are executed atomically, consistently, isolated, and durably.

## Basic Usage

### Simple Transaction

```python
from data_bridge.postgres import connection

# Initialize connection
await connection.init("postgresql://user:pass@localhost:5432/mydb")

# Use transaction context manager
async with connection.begin_transaction() as tx:
    # All operations within this block are part of the transaction
    await insert_one("users", {"name": "Alice", "email": "alice@example.com"})
    await update_one("accounts", {"user": "Alice"}, {"balance": 1000})
    # Transaction auto-commits on successful exit
```

### Explicit Commit

```python
async with connection.begin_transaction() as tx:
    await insert_one("users", {"name": "Bob"})
    await tx.commit()  # Explicit commit
```

### Explicit Rollback

```python
async with connection.begin_transaction() as tx:
    await insert_one("users", {"name": "Charlie"})
    # Decide to rollback based on some condition
    if some_error_condition:
        await tx.rollback()
        return
    await tx.commit()
```

### Automatic Rollback on Exception

```python
try:
    async with connection.begin_transaction() as tx:
        await insert_one("users", {"name": "Dave"})
        raise ValueError("Something went wrong")
        # Transaction automatically rolls back on exception
except ValueError:
    print("Transaction rolled back due to error")
```

## Isolation Levels

PostgreSQL supports four transaction isolation levels. You can specify the isolation level when beginning a transaction:

### Read Committed (Default)

```python
async with connection.begin_transaction("read_committed") as tx:
    # Prevents dirty reads
    # Other transactions' uncommitted changes are not visible
    await fetch_one("users", "id = $1", [1])
```

### Repeatable Read

```python
async with connection.begin_transaction("repeatable_read") as tx:
    # Prevents non-repeatable reads
    # Reading the same row multiple times returns the same data
    user1 = await fetch_one("users", "id = $1", [1])
    # Even if another transaction modifies this row, we see the same data
    user2 = await fetch_one("users", "id = $1", [1])
    assert user1 == user2
```

### Serializable

```python
async with connection.begin_transaction("serializable") as tx:
    # Highest isolation level
    # Prevents phantom reads
    # Transactions appear to execute serially
    users = await fetch_all("users", "age > $1", [18])
    # No new rows matching this condition can appear
    # even if another transaction inserts them
```

### Read Uncommitted

```python
async with connection.begin_transaction("read_uncommitted") as tx:
    # Note: PostgreSQL treats this the same as READ COMMITTED
    # True dirty reads are not possible in PostgreSQL
    await fetch_all("users")
```

## ACID Properties

### Atomicity
All operations in a transaction succeed or fail together:

```python
async with connection.begin_transaction() as tx:
    await insert_one("orders", {"product": "Widget", "quantity": 10})
    await update_one("inventory", {"product": "Widget"}, {"stock": {"$dec": 10}})
    # Both operations commit together, or neither commits
```

### Consistency
Database constraints are enforced:

```python
async with connection.begin_transaction() as tx:
    # If this violates a unique constraint, the entire transaction rolls back
    await insert_one("users", {"email": "duplicate@example.com"})
```

### Isolation
Concurrent transactions don't interfere:

```python
# Transaction 1
async with connection.begin_transaction("serializable") as tx1:
    balance = await fetch_one("accounts", "id = $1", [1])
    await update_one("accounts", {"id": 1}, {"balance": balance["balance"] + 100})

# Transaction 2 (concurrent)
async with connection.begin_transaction("serializable") as tx2:
    balance = await fetch_one("accounts", "id = $1", [1])
    await update_one("accounts", {"id": 1}, {"balance": balance["balance"] - 50})

# Isolation ensures correct final balance
```

### Durability
Committed changes persist even after system failure:

```python
async with connection.begin_transaction() as tx:
    await insert_one("critical_data", {"value": "important"})
    await tx.commit()
    # Data is guaranteed to persist even if system crashes after this point
```

## Error Handling

### Handling Transaction Errors

```python
try:
    async with connection.begin_transaction() as tx:
        await insert_one("users", {"name": "Eve"})
        # Database error (e.g., constraint violation)
except RuntimeError as e:
    print(f"Transaction failed: {e}")
    # Transaction automatically rolled back
```

### Invalid Isolation Level

```python
try:
    async with connection.begin_transaction("invalid_level") as tx:
        pass
except ValueError as e:
    print(f"Invalid isolation level: {e}")
```

## Best Practices

### 1. Keep Transactions Short
```python
# ✅ Good: Short transaction
async with connection.begin_transaction() as tx:
    await insert_one("users", user_data)

# ❌ Bad: Long-running transaction
async with connection.begin_transaction() as tx:
    await insert_one("users", user_data)
    await asyncio.sleep(60)  # Don't do this!
```

### 2. Choose Appropriate Isolation Level
```python
# Use READ COMMITTED for most cases (default)
async with connection.begin_transaction() as tx:
    await insert_one("logs", {"message": "Event occurred"})

# Use SERIALIZABLE only when necessary
async with connection.begin_transaction("serializable") as tx:
    # Critical financial transaction
    await transfer_funds(from_account, to_account, amount)
```

### 3. Handle Errors Appropriately
```python
max_retries = 3
for attempt in range(max_retries):
    try:
        async with connection.begin_transaction("serializable") as tx:
            await complex_operation()
        break  # Success
    except RuntimeError as e:
        if "serialization failure" in str(e) and attempt < max_retries - 1:
            continue  # Retry on serialization failure
        raise  # Re-raise other errors
```

## Future Work

The following features are planned for future releases:

### Savepoints
```python
# Not yet implemented - will be available in future version
async with connection.begin_transaction() as tx:
    await insert_one("users", {"name": "Frank"})
    await tx.savepoint("sp1")
    await insert_one("users", {"name": "Grace"})
    await tx.rollback_to("sp1")  # Rollback to savepoint
    await tx.commit()  # Only Frank is inserted
```

### Nested Transactions
```python
# Not yet implemented - will be available in future version
async with connection.begin_transaction() as outer_tx:
    await insert_one("users", {"name": "Henry"})
    async with connection.begin_transaction() as inner_tx:
        await insert_one("users", {"name": "Iris"})
        # Inner transaction
```

## Implementation Details

### Rust Backend
- All transaction logic is implemented in Rust for performance
- Uses SQLx's native transaction support
- Automatic rollback on drop if not committed
- Zero Python overhead

### Performance
- Minimal overhead compared to raw SQL
- GIL released during transaction operations
- No Python byte handling for BSON/SQL data

### Safety
- Transactions are RAII-safe (automatically rollback on drop)
- Type-safe isolation level handling
- Comprehensive error messages
