# Events

The event system allows you to hook into the lifecycle of database operations.

## Registering Listeners

Use the `@listens_for` decorator or convenience decorators like `@before_insert`.

```python
from data_bridge.postgres import Table
from data_bridge.postgres.events import before_insert, after_update
from datetime import datetime

class User(Table):
    username: str
    created_at: datetime
    updated_at: datetime

# Listener for a specific class
@before_insert(User)
def set_timestamps(target):
    now = datetime.now()
    target.created_at = now
    target.updated_at = now

@after_update(User)
def log_update(target):
    print(f"User {target.id} was updated")
```

## Available Events

### CRUD Events

These events are triggered during `save()` or `delete()` operations.

- `before_insert(target)` / `after_insert(target)`
- `before_update(target)` / `after_update(target)`
- `before_delete(target)` / `after_delete(target)`

### Session Events

These events are triggered by the `Session` or `UnitOfWork`.

- `before_flush`
- `after_commit`

## Global Listeners

You can register listeners that apply to *all* tables by passing `None` as the target.

```python
from data_bridge.postgres.events import before_insert

@before_insert(None)
def global_timestamp(target):
    if hasattr(target, "updated_at"):
        target.updated_at = datetime.now()
```

## Attribute Events

You can track changes to individual attributes by mixing in `AttributeEvents`.

```python
from data_bridge.postgres import Table
from data_bridge.postgres.events import AttributeEvents, listens_for

class User(Table, AttributeEvents):
    name: str

@listens_for(User, 'attribute_set')
def on_name_change(target, key, old_value, new_value):
    print(f"{key} changed from {old_value} to {new_value}")

# Usage
user = User(name="Old")
user.enable_tracking() # Must explicitly enable
user.name = "New"      # Triggers event
```
