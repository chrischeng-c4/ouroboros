# State Management

Data Bridge PostgreSQL implements the Unit of Work pattern to manage database state changes efficiently and consistently.

## The Session

The `Session` is the central coordination point for database operations. It tracks loaded objects and manages pending changes.

```python
from data_bridge.postgres import Session, User, get_session

async with Session() as session:
    user = User(name="Alice", email="alice@example.com")
    session.add(user)
    
    # Changes are pending, not yet in DB
    await session.commit()
    # Now changes are saved
```

You can also use `get_session()` to retrieve the current active session.

### Identity Map

The session maintains an **Identity Map**, which ensures that for a given session, only one object instance exists for a particular database row.

```python
async with Session() as session:
    # Note: with_for_update=True is accepted but currently NOT implemented
    user1 = await session.get(User, 1, with_for_update=True)
    user2 = await session.get(User, 1)
    
    assert user1 is user2  # They are the exact same instance
```

This prevents conflicting updates and ensures data consistency within a transaction.

## Unit of Work

The Unit of Work pattern tracks object lifecycle state during a session, primarily focusing on insertions and deletions.

1.  **New Objects**: Objects added via `session.add()` or `session.add_all()`.
2.  **Deleted Objects**: Objects marked for deletion via `session.delete()`.

When you call `commit()`, the session executes the pending operations to synchronize the database.

### Modifications

The Session does NOT automatically track attribute changes on loaded objects. While some internal dirty tracking infrastructure exists, it is NOT automatically triggered by attribute modifications.

Therefore, `commit()` will NOT automatically generate UPDATE statements for modified objects. 

You must explicitly call `save()` on the object or use `Table.update_many()` to persist changes.

```python
async with Session() as session:
    user = await session.get(User, 1)
    
    # Modify the object
    user.name = "New Name"
    
    # Explicitly save changes
    await user.save() 
```

## Session Methods

The `Session` provides several key methods for managing state:

-   `add(obj)`: Add an object to the session.
-   `add_all(objects)`: Add a list of objects.
-   `get(model, pk, with_for_update=False)`: Get object by primary key (note: `with_for_update` is NOT implemented).
-   `delete(obj)`: Mark object for deletion.
-   `flush()`: Push pending changes (INSERTs/DELETEs) to the database without committing.
-   `commit()`: Flush changes and commit the transaction.
-   `rollback()`: Rollback the transaction and clear the session.
-   `close()`: Close the session and clear the identity map.
-   `expunge(obj)`: Remove an object from the session.
-   `expunge_all()`: Remove all objects from the session.

## Object States

An object in the session can be in one of several states:

-   **Transient**: Not attached to a session (e.g., newly created `User()`).
-   **Pending**: Added to session, waiting for `INSERT`.
-   **Persistent**: Saved in database and attached to session.
-   **Deleted**: Scheduled for `DELETE`.
-   **Detached**: Was persistent, but session is closed or object was expunged.
