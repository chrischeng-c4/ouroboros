# Querying

The `QueryBuilder` provides a fluent interface for constructing SQL queries safely and efficiently.

## Basic Queries

### Find Many

Use `find()` to start a query. It returns a `QueryBuilder` instance.

```python
# Get all users
users = await User.find().to_list()

# Iterate asynchronously
async for user in User.find():
    print(user.name)
```

### Find One

Use `find_one()` to get a single record.

```python
user = await User.find_one(User.email == "admin@example.com")
```

### Get by ID

Use `get()` for primary key lookups.

```python
user = await User.get(42)
```

## Filtering

The ORM uses operator overloading on column attributes to generate SQL expressions.

### Operators

```python
# Equality
User.name == "Alice"

# Inequality
User.age > 18
User.age <= 65
User.status != "banned"

# String matching
User.email.like("%@gmail.com")
User.name.ilike("alice%")  # Case-insensitive

# IN clause
User.role.in_(["admin", "editor"])

# NULL checks
User.deleted_at.is_(None)
User.deleted_at.is_not(None)
```

### Combining Filters

Multiple arguments to `find()` are treated as `AND`.

```python
# WHERE age > 18 AND active = true
users = await User.find(
    User.age > 18,
    User.active == True
).to_list()
```

For explicit `OR` or complex logic, use `or_`, `and_`, `not_`:

```python
from data_bridge.postgres import or_, and_

# WHERE (role = 'admin') OR (age > 30 AND active = true)
users = await User.find(
    or_(
        User.role == "admin",
        and_(User.age > 30, User.active == True)
    )
).to_list()
```

## Sorting and Pagination

```python
# Sort by name ascending
await User.find().order_by(User.name).to_list()

# Sort by age descending
await User.find().order_by(User.age.desc()).to_list()

# Pagination
page_2 = await User.find()\
    .limit(20)\
    .offset(20)\
    .to_list()
```

## Aggregates

```python
# Count
count = await User.find(User.active == True).count()
```

## Bulk Operations

Perform efficient bulk actions without loading objects into memory.

### Bulk Insert

```python
await User.insert_many([
    {"name": "A", "email": "a@x.com"},
    {"name": "B", "email": "b@x.com"}
])
```

### Bulk Update

```python
# Set status='inactive' for all users where last_login < 2023
await User.update_many(
    {"status": "inactive"},
    User.last_login < datetime(2023, 1, 1)
)
```

### Bulk Delete

```python
await User.delete_many(User.status == "banned")
```

## Relationship Loading

(See dedicated Relationships guide for details)

You can eagerly load related data to avoid N+1 queries using `selectinload` or `joinedload`.

```python
from data_bridge.postgres import selectinload

users = await User.find()
    .options(selectinload(User.posts))
    .to_list()
```
