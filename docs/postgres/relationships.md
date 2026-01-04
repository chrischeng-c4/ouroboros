# PostgreSQL Relationships Guide

**Version**: 1.0
**Date**: 2025-12-30
**Status**: Production Ready

---

## Table of Contents

1. [Overview](#overview)
2. [ForeignKey Setup](#foreignkey-setup)
3. [ForeignKeyProxy - Lazy Loading](#foreignkeyproxy---lazy-loading)
4. [BackReference - Reverse Relationships](#backreference---reverse-relationships)
5. [Eager Loading - Avoiding N+1](#eager-loading---avoiding-n1)
6. [Cascade Delete Operations](#cascade-delete-operations)
7. [Savepoints - Partial Rollback](#savepoints---partial-rollback)
8. [Common Patterns](#common-patterns)
9. [Troubleshooting](#troubleshooting)

---

## Overview

The data-bridge PostgreSQL module provides a comprehensive relationship system inspired by Django and SQLAlchemy, but optimized for Rust performance. This guide covers:

- **Foreign Keys**: One-to-many and many-to-one relationships
- **Lazy Loading**: Fetch related data only when needed
- **Eager Loading**: Avoid N+1 queries with JOIN-based fetching
- **Cascade Operations**: Handle related data deletion
- **Savepoints**: Partial rollback within transactions

All relationship operations are implemented in Rust for maximum performance with zero Python overhead.

---

## ForeignKey Setup

### Basic Configuration

Foreign keys are defined using the `Column` class with the `foreign_key` parameter:

```python
from data_bridge.postgres import Column

class User:
    id: int  # Auto-generated primary key
    name: str
    email: str = Column(unique=True)

class Post:
    id: int
    title: str
    content: str
    # Foreign key to users table
    author_id: int = Column(foreign_key="users")
```

This creates the following SQL:

```sql
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    author_id INTEGER REFERENCES users(id)
);
```

### Advanced Configuration with Cascade Rules

The `Column` class supports full foreign key configuration:

```python
from data_bridge.postgres import Column

class Post:
    title: str
    author_id: int = Column(
        foreign_key="users.id",        # Explicit table.column reference
        on_delete="CASCADE",            # Delete posts when user is deleted
        on_update="CASCADE",            # Update FK when user ID changes
        nullable=False                  # Post must have an author
    )

class Comment:
    post_id: int = Column(
        foreign_key="posts.id",
        on_delete="CASCADE",            # Delete comment when post is deleted
        on_update="CASCADE"
    )

class Profile:
    user_id: int = Column(
        foreign_key="users.id",
        on_delete="SET NULL",           # Keep profile but clear user_id
        on_update="CASCADE",
        nullable=True
    )

class Order:
    product_id: int = Column(
        foreign_key="products.id",
        on_delete="RESTRICT",           # Prevent deletion if orders exist
        on_update="RESTRICT"
    )
```

### Supported Cascade Actions

| Action | Behavior | Use Case |
|--------|----------|----------|
| `CASCADE` | Delete/update child rows when parent changes | Parent-child data (User → Posts) |
| `RESTRICT` | Prevent deletion/update if children exist | Protected data (Product → Orders) |
| `SET NULL` | Set foreign key to NULL | Optional relationships (User → Profile) |
| `SET DEFAULT` | Set foreign key to DEFAULT value | Fallback relationships |
| `NO ACTION` | Same as RESTRICT (PostgreSQL default) | Standard constraint |

### Example Schema

```python
from data_bridge.postgres import execute

# Create tables with foreign key constraints
await execute("""
    CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) NOT NULL,
        email VARCHAR(255) UNIQUE NOT NULL
    )
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        content TEXT NOT NULL,
        author_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )
""")

await execute("""
    CREATE TABLE comments (
        id SERIAL PRIMARY KEY,
        post_id INTEGER REFERENCES posts(id) ON DELETE CASCADE,
        author_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
        content TEXT NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )
""")
```

---

## ForeignKeyProxy - Lazy Loading

The `ForeignKeyProxy` class enables lazy loading of related objects. The related data is fetched only when explicitly requested.

### Creating a ForeignKeyProxy

```python
from data_bridge.postgres import ForeignKeyProxy

# Create proxy for a user reference
author_proxy = ForeignKeyProxy(
    target_table="users",
    foreign_key_column="id",
    foreign_key_value=123
)
```

### Properties

#### `.ref` - Get Foreign Key Value

Access the foreign key value without fetching the related object:

```python
# No database query - just returns the FK value
user_id = author_proxy.ref
print(f"Author ID: {user_id}")  # Output: Author ID: 123
```

#### `.id` - Alias for `.ref`

```python
# Same as .ref
user_id = author_proxy.id
```

#### `.column_value` - Raw Column Value

```python
# Get the raw column value
fk_value = author_proxy.column_value
```

#### `.is_fetched` - Check Fetch Status

```python
# Check if data has been fetched
if not author_proxy.is_fetched:
    print("Data not yet fetched from database")
```

### Methods

#### `.fetch()` - Load Related Object

Fetch the related object from the database:

```python
# First call: queries database
author = await author_proxy.fetch()
print(author["name"])  # Output: Alice

# Subsequent calls: returns cached value (no database query)
author_again = await author_proxy.fetch()
assert author == author_again
```

### Complete Example

```python
from data_bridge.postgres import execute, ForeignKeyProxy

# Create tables
await execute("""
    CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) NOT NULL,
        email VARCHAR(255) UNIQUE NOT NULL
    )
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        author_id INTEGER REFERENCES users(id)
    )
""")

# Insert test data
await execute("INSERT INTO users (id, name, email) VALUES (1, $1, $2)",
              ["Alice", "alice@example.com"])
await execute("INSERT INTO posts (title, author_id) VALUES ($1, $2)",
              ["My First Post", 1])

# Query post
post = await execute("SELECT * FROM posts WHERE id = $1", [1])
post = post[0]

# Create ForeignKeyProxy for lazy loading
author_proxy = ForeignKeyProxy("users", "id", post["author_id"])

# Access FK value without fetching (no database query)
print(f"Author ID: {author_proxy.ref}")
print(f"Fetched: {author_proxy.is_fetched}")  # Output: False

# Fetch related data (database query)
author = await author_proxy.fetch()
print(f"Author: {author['name']}")  # Output: Alice
print(f"Fetched: {author_proxy.is_fetched}")  # Output: True

# Second fetch uses cache (no database query)
author_cached = await author_proxy.fetch()
```

---

## BackReference - Reverse Relationships

`BackReference` enables accessing related rows that have a foreign key pointing to the current table. This is the inverse of `ForeignKeyProxy`.

### Defining BackReferences

```python
from data_bridge.postgres import BackReference

class User:
    id: int
    name: str
    email: str

    # Define reverse relationship - all posts where user_id = this user's id
    posts = BackReference(
        source_table="posts",      # Table that has the FK
        source_column="user_id",   # FK column name
        target_column="id"         # Column in this table (default: "id")
    )
```

### Using BackReferences

BackReferences return a `BackReferenceQuery` object when accessed on an instance:

```python
from data_bridge.postgres import execute, insert_one

# Create tables
await execute("""
    CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) NOT NULL
    )
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        user_id INTEGER REFERENCES users(id)
    )
""")

# Insert data
user = await insert_one("users", {"name": "Alice"})
await insert_one("posts", {"title": "Post 1", "user_id": user["id"]})
await insert_one("posts", {"title": "Post 2", "user_id": user["id"]})

# In a real application with Table classes:
# user.posts would be a BackReferenceQuery
```

### BackReferenceQuery Methods

#### `.fetch_all()` - Fetch All Related Rows

```python
# Fetch all posts for a user
posts = await user.posts.fetch_all()

for post in posts:
    print(f"- {post['title']}")

# Output:
# - Post 1
# - Post 2
```

#### `.fetch_one()` - Fetch First Related Row

```python
# Fetch first post for a user
first_post = await user.posts.fetch_one()

if first_post:
    print(f"First post: {first_post['title']}")
```

#### `.count()` - Count Related Rows

```python
# Count posts for a user
post_count = await user.posts.count()
print(f"User has {post_count} posts")
```

### Complete Example

```python
from data_bridge.postgres import execute, insert_one

# Setup tables
await execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))")
await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255),
        user_id INTEGER REFERENCES users(id)
    )
""")

# Insert data
user = await insert_one("users", {"name": "Bob"})
user_id = user["id"]

for i in range(5):
    await insert_one("posts", {
        "title": f"Post {i+1}",
        "user_id": user_id
    })

# Using BackReferenceQuery (simulated)
# In practice, this would be: user.posts.fetch_all()
from data_bridge.data_bridge import postgres as _engine

where_clause = f"user_id = $1"
posts = await _engine.find_many("posts", where_clause, [user_id])
print(f"Found {len(posts)} posts")  # Output: Found 5 posts

# Count posts
count = await _engine.count("posts", where_clause, [user_id])
print(f"Post count: {count}")  # Output: Post count: 5
```

---

## Eager Loading - Avoiding N+1

The N+1 query problem occurs when you fetch a list of objects, then fetch related data for each one individually:

```python
# BAD: N+1 queries
posts = await fetch_all("posts")  # 1 query
for post in posts:
    author = await fetch_one("users", f"id = {post['author_id']}")  # N queries
    print(f"{post['title']} by {author['name']}")
# Total: 1 + N queries
```

Eager loading solves this by using SQL JOINs to fetch related data in a single query.

### Method 1: `fetch_one_eager()` - Simple Tuple API

For fetching a single row with relations using a simple tuple-based API:

```python
from data_bridge.postgres import fetch_one_eager

# Fetch post with author and category
post = await fetch_one_eager(
    "posts",
    post_id,
    [
        ("author", "author_id", "users"),        # (relation_name, fk_column, target_table)
        ("category", "category_id", "categories")
    ]
)

if post:
    print(f"Title: {post['title']}")
    print(f"Author ID: {post['author_id']}")
    # Related data structure depends on Rust implementation
```

### Method 2: `fetch_one_with_relations()` - Full Configuration

For fetching a single row with detailed JOIN configuration:

```python
from data_bridge.postgres import fetch_one_with_relations

user = await fetch_one_with_relations(
    "users",
    user_id,
    [
        {
            "name": "posts",                    # Relation name in result
            "table": "posts",                   # Related table
            "foreign_key": "user_id",           # FK column in related table
            "reference_column": "id",           # Column in this table
            "join_type": "left",                # JOIN type (left, inner, right, full)
            "select_columns": ["id", "title"]   # Optional: columns to select
        },
        {
            "name": "profile",
            "table": "profiles",
            "foreign_key": "user_id",
            "reference_column": "id",
            "join_type": "left"
        }
    ]
)

if user:
    print(f"User: {user['name']}")
    print(f"Posts: {user['posts']}")
    print(f"Profile: {user['profile']}")
```

### Method 3: `fetch_many_with_relations()` - Batch Eager Loading

Fetch multiple rows with relations efficiently:

```python
from data_bridge.postgres import fetch_many_with_relations

users = await fetch_many_with_relations(
    "users",
    relations=[
        {
            "name": "posts",
            "table": "posts",
            "foreign_key": "user_id",
            "reference_column": "id",
            "join_type": "left"
        }
    ],
    filter={"status": "active"},          # Optional WHERE filter
    order_by=("created_at", "DESC"),      # Optional ORDER BY
    limit=10,                              # Optional LIMIT
    offset=0                               # Optional OFFSET
)

for user in users:
    print(f"{user['name']}: {len(user['posts'])} posts")
```

### JOIN Types

| Join Type | Behavior | Use Case |
|-----------|----------|----------|
| `left` (default) | Include all rows from main table, even without matches | Optional relationships |
| `inner` | Only include rows with matching relations | Required relationships |
| `right` | Include all rows from related table | Rare use case |
| `full` | Include all rows from both tables | Data analysis |

### Performance Comparison

```python
# Lazy Loading (N+1 problem)
# ❌ BAD: 1 + N queries
posts = await execute("SELECT * FROM posts LIMIT 100")  # 1 query
for post in posts:
    author = await execute("SELECT * FROM users WHERE id = $1", [post["author_id"]])  # 100 queries
    print(f"{post['title']} by {author[0]['name']}")
# Total: 101 queries

# Eager Loading with JOIN
# ✅ GOOD: 1 query
posts = await fetch_many_with_relations(
    "posts",
    relations=[
        {
            "name": "author",
            "table": "users",
            "foreign_key": "author_id",
            "reference_column": "id"
        }
    ],
    limit=100
)
for post in posts:
    print(f"{post['title']} by {post['author']['name']}")
# Total: 1 query (100x faster!)
```

### Complete Example

```python
from data_bridge.postgres import (
    execute,
    fetch_one_with_relations,
    fetch_many_with_relations
)

# Setup schema
await execute("""
    CREATE TABLE authors (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) NOT NULL
    )
""")

await execute("""
    CREATE TABLE categories (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) NOT NULL
    )
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        author_id INTEGER REFERENCES authors(id),
        category_id INTEGER REFERENCES categories(id)
    )
""")

# Insert data
await execute("INSERT INTO authors (name) VALUES ($1)", ["Alice"])
await execute("INSERT INTO categories (name) VALUES ($1)", ["Tech"])
await execute("""
    INSERT INTO posts (title, author_id, category_id)
    VALUES ($1, $2, $3)
""", ["My Post", 1, 1])

# Eager load single post with author and category
post = await fetch_one_with_relations(
    "posts",
    1,
    [
        {
            "name": "author",
            "table": "authors",
            "foreign_key": "author_id",
            "reference_column": "id",
            "join_type": "left"
        },
        {
            "name": "category",
            "table": "categories",
            "foreign_key": "category_id",
            "reference_column": "id",
            "join_type": "left"
        }
    ]
)

print(f"Post: {post['title']}")
print(f"Author: {post['author']}")
print(f"Category: {post['category']}")
```

---

## Cascade Delete Operations

When deleting rows with foreign key relationships, you need to handle related data. data-bridge provides functions to handle cascade deletes based on foreign key constraints.

### Method 1: `delete_with_cascade()` - Full Cascade Handling

Manually handles all ON DELETE rules:

```python
from data_bridge.postgres import delete_with_cascade

# Delete user and handle all related data based on FK rules
# - CASCADE: Deletes child rows
# - RESTRICT: Raises error if children exist
# - SET NULL: Sets FK to NULL before delete
# - SET DEFAULT: Sets FK to DEFAULT before delete
deleted_count = await delete_with_cascade("users", user_id)
print(f"Deleted {deleted_count} rows total (including cascaded)")
```

### Method 2: `delete_checked()` - Constraint Validation

Checks RESTRICT constraints before deletion, relies on database CASCADE:

```python
from data_bridge.postgres import delete_checked

try:
    deleted = await delete_checked("users", user_id)
    print(f"User deleted successfully")
except RuntimeError as e:
    print(f"Cannot delete: {e}")
    # Error: "Cannot delete - RESTRICT constraint: posts table has 5 related rows"
```

### Schema Introspection: `get_backreferences()`

Find all tables that reference a given table:

```python
from data_bridge.postgres import get_backreferences

# Find all tables that reference users
backrefs = await get_backreferences("users")

for ref in backrefs:
    print(f"{ref['source_table']}.{ref['source_column']} -> {ref['target_table']}.{ref['target_column']}")
    print(f"  ON DELETE {ref['on_delete']}")
    print(f"  ON UPDATE {ref['on_update']}")

# Output:
# posts.author_id -> users.id
#   ON DELETE CASCADE
#   ON UPDATE CASCADE
# comments.author_id -> users.id
#   ON DELETE SET NULL
#   ON UPDATE CASCADE
```

### Complete Example

```python
from data_bridge.postgres import (
    execute,
    insert_one,
    delete_with_cascade,
    delete_checked,
    get_backreferences
)

# Setup schema
await execute("""
    CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255)
    )
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255),
        user_id INTEGER REFERENCES users(id) ON DELETE CASCADE
    )
""")

await execute("""
    CREATE TABLE comments (
        id SERIAL PRIMARY KEY,
        content TEXT,
        user_id INTEGER REFERENCES users(id) ON DELETE RESTRICT
    )
""")

# Insert data
user = await insert_one("users", {"name": "Alice"})
user_id = user["id"]

await insert_one("posts", {"title": "Post 1", "user_id": user_id})
await insert_one("posts", {"title": "Post 2", "user_id": user_id})
await insert_one("comments", {"content": "Comment 1", "user_id": user_id})

# Check backreferences before deletion
backrefs = await get_backreferences("users")
print(f"Found {len(backrefs)} backreferences:")
for ref in backrefs:
    print(f"  - {ref['source_table']}.{ref['source_column']} ({ref['on_delete']})")

# Attempt delete_checked (fails due to RESTRICT on comments)
try:
    await delete_checked("users", user_id)
except RuntimeError as e:
    print(f"Delete failed: {e}")

# Use delete_with_cascade (handles CASCADE and RESTRICT)
try:
    total_deleted = await delete_with_cascade("users", user_id)
    print(f"Successfully deleted {total_deleted} rows")
except RuntimeError as e:
    print(f"Cannot delete due to RESTRICT constraint: {e}")
```

---

## Savepoints - Partial Rollback

Savepoints allow partial rollback within a transaction, enabling you to undo specific operations without rolling back the entire transaction.

### Basic Savepoint Usage

```python
from data_bridge.postgres.transactions import pg_transaction

async with pg_transaction() as tx:
    # Insert initial data
    await tx._tx.insert_one("users", {"name": "Alice", "email": "alice@example.com"})

    # Create savepoint
    sp = await tx.savepoint("before_posts")

    # Insert posts
    await tx._tx.insert_one("posts", {"title": "Post 1", "user_id": 1})
    await tx._tx.insert_one("posts", {"title": "Post 2", "user_id": 1})

    # Rollback to savepoint (undoes post inserts)
    await sp.rollback()

    # Transaction commits (only user is saved)
    await tx.commit()
```

### Savepoint as Context Manager

Savepoints can be used as async context managers for automatic cleanup:

```python
from data_bridge.postgres.transactions import pg_transaction

async with pg_transaction() as tx:
    # Insert user
    await tx._tx.insert_one("users", {"name": "Bob"})

    # Savepoint auto-releases on success
    async with await tx.savepoint("risky_operation"):
        await tx._tx.insert_one("posts", {"title": "Safe Post", "user_id": 1})
        # If no exception, savepoint is released (changes kept)

    # Savepoint auto-rolls back on exception
    try:
        async with await tx.savepoint("failing_operation"):
            await tx._tx.insert_one("posts", {"title": "Bad Post", "user_id": 999})
            raise ValueError("Simulated error")
    except ValueError:
        pass  # Changes rolled back automatically

    await tx.commit()  # User and "Safe Post" are saved
```

### Savepoint Methods

#### `await tx.savepoint(name)` - Create Savepoint

```python
sp = await tx.savepoint("checkpoint")
```

#### `await sp.rollback()` - Rollback to Savepoint

Undo all changes made after the savepoint was created:

```python
await sp.rollback()
```

#### `await sp.release()` - Release Savepoint

Destroy the savepoint but keep the changes:

```python
await sp.release()
```

### Nested Savepoints

You can create multiple savepoints and roll back to any of them:

```python
from data_bridge.postgres.transactions import pg_transaction
from data_bridge.postgres import connection

async with pg_transaction() as tx:
    # Phase 1: Initial data
    await connection.insert_one("users", {"name": "Alice"})

    # Savepoint 1
    await tx._tx.savepoint("sp1")
    await connection.insert_one("posts", {"title": "Post 1", "user_id": 1})

    # Savepoint 2 (nested)
    await tx._tx.savepoint("sp2")
    await connection.insert_one("posts", {"title": "Post 2", "user_id": 1})

    # Rollback to sp1 (removes both Post 1 and Post 2)
    await tx._tx.rollback_to_savepoint("sp1")

    # Add new data after rollback
    await connection.insert_one("posts", {"title": "Post 3", "user_id": 1})

    await tx.commit()  # User and Post 3 are saved
```

### Complete Example: Complex Workflow

```python
from data_bridge.postgres.transactions import pg_transaction
from data_bridge.postgres import connection, execute

async with pg_transaction() as tx:
    # Create user
    user = await connection.insert_one("users", {
        "name": "Charlie",
        "email": "charlie@example.com"
    })
    user_id = user["id"]

    # Try to create posts with error handling
    sp_posts = await tx.savepoint("posts_operation")
    try:
        await connection.insert_one("posts", {"title": "Post 1", "user_id": user_id})
        await connection.insert_one("posts", {"title": "Post 2", "user_id": user_id})

        # Simulate validation error
        if True:  # Replace with actual validation
            raise ValueError("Posts validation failed")

        await sp_posts.release()
    except ValueError:
        # Rollback posts but keep user
        await sp_posts.rollback()
        print("Posts rolled back due to validation error")

    # Create profile (independent operation)
    sp_profile = await tx.savepoint("profile_operation")
    try:
        await connection.insert_one("profiles", {
            "user_id": user_id,
            "bio": "Developer"
        })
        await sp_profile.release()
    except Exception as e:
        await sp_profile.rollback()
        print(f"Profile creation failed: {e}")

    # Transaction commits with user and profile (posts were rolled back)
    await tx.commit()

# Verify results
users = await execute("SELECT * FROM users WHERE name = $1", ["Charlie"])
print(f"Users: {len(users)}")  # Output: 1

posts = await execute("SELECT * FROM posts WHERE user_id = $1", [users[0]["id"]])
print(f"Posts: {len(posts)}")  # Output: 0 (rolled back)

profiles = await execute("SELECT * FROM profiles WHERE user_id = $1", [users[0]["id"]])
print(f"Profiles: {len(profiles)}")  # Output: 1
```

### Sequential Savepoints

You can create multiple savepoints in sequence:

```python
async with pg_transaction() as tx:
    await connection.insert_one("users", {"name": "Dave"})

    # Savepoint 1
    await tx._tx.savepoint("sp1")
    await connection.insert_one("posts", {"title": "Post 1", "user_id": 1})
    await tx._tx.release_savepoint("sp1")  # Keep changes

    # Savepoint 2 (after sp1 is released)
    await tx._tx.savepoint("sp2")
    await connection.insert_one("posts", {"title": "Post 2", "user_id": 1})
    await tx._tx.rollback_to_savepoint("sp2")  # Discard Post 2

    # Savepoint 3
    await tx._tx.savepoint("sp3")
    await connection.insert_one("posts", {"title": "Post 3", "user_id": 1})
    await tx._tx.release_savepoint("sp3")  # Keep changes

    await tx.commit()  # User, Post 1, and Post 3 are saved
```

---

## Common Patterns

### Pattern 1: User with Posts (One-to-Many)

```python
from data_bridge.postgres import execute, insert_one, BackReference, ForeignKeyProxy

# Schema
await execute("""
    CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) NOT NULL,
        email VARCHAR(255) UNIQUE NOT NULL
    )
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        content TEXT,
        user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )
""")

# Insert data
user = await insert_one("users", {"name": "Alice", "email": "alice@example.com"})
user_id = user["id"]

post1 = await insert_one("posts", {"title": "First Post", "user_id": user_id})
post2 = await insert_one("posts", {"title": "Second Post", "user_id": user_id})

# Access via ForeignKeyProxy (lazy loading)
author_proxy = ForeignKeyProxy("users", "id", post1["user_id"])
author = await author_proxy.fetch()
print(f"Author: {author['name']}")

# Access via BackReference (reverse query)
# In practice, this would be: user.posts.fetch_all()
from data_bridge.data_bridge import postgres as _engine
posts = await _engine.find_many("posts", "user_id = $1", [user_id])
print(f"User has {len(posts)} posts")
```

### Pattern 2: Blog with Categories (Many-to-One)

```python
# Schema
await execute("""
    CREATE TABLE categories (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) UNIQUE NOT NULL,
        slug VARCHAR(255) UNIQUE NOT NULL
    )
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL,
        published BOOLEAN DEFAULT FALSE
    )
""")

# Insert data
tech = await insert_one("categories", {"name": "Technology", "slug": "tech"})
post = await insert_one("posts", {"title": "AI News", "category_id": tech["id"]})

# Lazy load category
category_proxy = ForeignKeyProxy("categories", "id", post["category_id"])
category = await category_proxy.fetch()
print(f"Category: {category['name']}")

# Eager load with JOIN
from data_bridge.postgres import fetch_one_with_relations

post_with_category = await fetch_one_with_relations(
    "posts",
    post["id"],
    [
        {
            "name": "category",
            "table": "categories",
            "foreign_key": "category_id",
            "reference_column": "id",
            "join_type": "left"
        }
    ]
)
```

### Pattern 3: Multi-level Relationships

```python
# Schema: Users → Posts → Comments
await execute("""
    CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255))
""")

await execute("""
    CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        title VARCHAR(255),
        user_id INTEGER REFERENCES users(id) ON DELETE CASCADE
    )
""")

await execute("""
    CREATE TABLE comments (
        id SERIAL PRIMARY KEY,
        content TEXT,
        post_id INTEGER REFERENCES posts(id) ON DELETE CASCADE,
        user_id INTEGER REFERENCES users(id) ON DELETE SET NULL
    )
""")

# Eager load entire chain
from data_bridge.postgres import fetch_many_with_relations

comments = await fetch_many_with_relations(
    "comments",
    relations=[
        {
            "name": "post",
            "table": "posts",
            "foreign_key": "post_id",
            "reference_column": "id"
        },
        {
            "name": "user",
            "table": "users",
            "foreign_key": "user_id",
            "reference_column": "id"
        }
    ],
    limit=100
)

for comment in comments:
    print(f"{comment['user']['name']} commented on {comment['post']['title']}")
```

### Pattern 4: Safe Deletion with Transaction and Savepoint

```python
from data_bridge.postgres.transactions import pg_transaction
from data_bridge.postgres import connection, delete_with_cascade

async with pg_transaction() as tx:
    # Savepoint before deletion
    sp = await tx.savepoint("before_delete")

    try:
        # Attempt to delete user and related data
        deleted = await delete_with_cascade("users", user_id)
        print(f"Deleted {deleted} rows")

        # Verify deletion
        remaining_posts = await tx.execute(
            "SELECT COUNT(*) FROM posts WHERE user_id = $1",
            [user_id]
        )

        if remaining_posts[0]["count"] > 0:
            # Unexpected state - rollback
            raise ValueError("Posts still exist after cascade delete")

        # Commit if everything is OK
        await tx.commit()

    except Exception as e:
        # Rollback to savepoint
        await sp.rollback()
        print(f"Deletion failed, rolled back: {e}")
        raise
```

### Pattern 5: Batch Operations with Relationships

```python
from data_bridge.postgres import insert_many, fetch_many_with_relations

# Batch insert users
users = await insert_many("users", [
    {"name": "Alice", "email": "alice@example.com"},
    {"name": "Bob", "email": "bob@example.com"},
    {"name": "Charlie", "email": "charlie@example.com"}
])

# Batch insert posts
posts_data = []
for user in users:
    for i in range(3):
        posts_data.append({
            "title": f"{user['name']}'s Post {i+1}",
            "user_id": user["id"]
        })

posts = await insert_many("posts", posts_data)

# Eager load all users with their posts
users_with_posts = await fetch_many_with_relations(
    "users",
    relations=[
        {
            "name": "posts",
            "table": "posts",
            "foreign_key": "user_id",
            "reference_column": "id"
        }
    ]
)

for user in users_with_posts:
    print(f"{user['name']}: {len(user['posts'])} posts")
```

---

## Troubleshooting

### Issue 1: Foreign Key Constraint Violation

**Error**: `foreign key constraint "fk_name" violated`

**Cause**: Attempting to insert a row with a foreign key value that doesn't exist in the referenced table.

**Solution**:
```python
# ❌ BAD: Insert post with non-existent user_id
await insert_one("posts", {"title": "My Post", "user_id": 999})
# Error: foreign key constraint violated

# ✅ GOOD: Insert user first, then post
user = await insert_one("users", {"name": "Alice", "email": "alice@example.com"})
await insert_one("posts", {"title": "My Post", "user_id": user["id"]})
```

### Issue 2: Cannot Delete Due to RESTRICT Constraint

**Error**: `Cannot delete - RESTRICT constraint: posts table has 5 related rows`

**Cause**: Attempting to delete a row that has related rows with ON DELETE RESTRICT.

**Solution**:
```python
# Option 1: Delete related rows first
posts = await execute("SELECT id FROM posts WHERE user_id = $1", [user_id])
for post in posts:
    await execute("DELETE FROM posts WHERE id = $1", [post["id"]])
await execute("DELETE FROM users WHERE id = $1", [user_id])

# Option 2: Change constraint to CASCADE
await execute("""
    ALTER TABLE posts
    DROP CONSTRAINT posts_user_id_fkey,
    ADD CONSTRAINT posts_user_id_fkey
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
""")

# Option 3: Use delete_with_cascade (handles RESTRICT with error)
from data_bridge.postgres import delete_with_cascade
try:
    await delete_with_cascade("users", user_id)
except RuntimeError as e:
    print(f"Cannot delete: {e}")
```

### Issue 3: N+1 Query Performance Problem

**Symptom**: Slow performance when fetching related data in a loop.

**Cause**: Making separate database queries for each related object.

**Solution**:
```python
# ❌ BAD: N+1 queries (1 + 100 queries)
posts = await execute("SELECT * FROM posts LIMIT 100")
for post in posts:
    author = await execute("SELECT * FROM users WHERE id = $1", [post["author_id"]])
    print(f"{post['title']} by {author[0]['name']}")

# ✅ GOOD: Eager loading with JOIN (1 query)
from data_bridge.postgres import fetch_many_with_relations

posts = await fetch_many_with_relations(
    "posts",
    relations=[
        {
            "name": "author",
            "table": "users",
            "foreign_key": "author_id",
            "reference_column": "id"
        }
    ],
    limit=100
)

for post in posts:
    print(f"{post['title']} by {post['author']['name']}")
```

### Issue 4: Savepoint Already Released

**Error**: `RuntimeError: Savepoint has been released`

**Cause**: Attempting to use a savepoint after it has been released.

**Solution**:
```python
# ❌ BAD: Using savepoint after release
sp = await tx.savepoint("sp1")
await sp.release()
await sp.rollback()  # Error: already released

# ✅ GOOD: Check before using or use context manager
sp = await tx.savepoint("sp1")
if not sp._released:
    await sp.rollback()

# ✅ BETTER: Use context manager (auto-release/rollback)
async with await tx.savepoint("sp1"):
    await some_operation()
    # Auto-releases on success, auto-rolls back on exception
```

### Issue 5: ForeignKeyProxy Returns None

**Symptom**: `await proxy.fetch()` returns `None` even though foreign key value exists.

**Cause**: The referenced row has been deleted or the foreign key value is invalid.

**Solution**:
```python
# Check if foreign key value is valid
author_proxy = ForeignKeyProxy("users", "id", post["author_id"])

if post["author_id"] is None:
    print("Post has no author (NULL foreign key)")
else:
    author = await author_proxy.fetch()
    if author is None:
        print(f"Author with id {post['author_id']} not found")
    else:
        print(f"Author: {author['name']}")

# Alternative: Use LEFT JOIN to handle NULL FKs
from data_bridge.postgres import fetch_one_with_relations

post = await fetch_one_with_relations(
    "posts",
    post_id,
    [
        {
            "name": "author",
            "table": "users",
            "foreign_key": "author_id",
            "reference_column": "id",
            "join_type": "left"  # Includes posts without authors
        }
    ]
)
```

### Issue 6: Circular Foreign Key Dependencies

**Error**: Unable to create tables due to circular foreign key references.

**Cause**: Two tables reference each other, creating a dependency cycle.

**Solution**:
```python
# ❌ BAD: Circular dependency
# CREATE TABLE users (
#     profile_id INTEGER REFERENCES profiles(id)  -- References profiles
# );
# CREATE TABLE profiles (
#     user_id INTEGER REFERENCES users(id)  -- References users
# );

# ✅ GOOD: Create tables first, add FKs later
await execute("""
    CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255)
    )
""")

await execute("""
    CREATE TABLE profiles (
        id SERIAL PRIMARY KEY,
        user_id INTEGER,
        bio TEXT
    )
""")

# Add foreign keys after tables exist
await execute("""
    ALTER TABLE users
    ADD COLUMN profile_id INTEGER REFERENCES profiles(id)
""")

await execute("""
    ALTER TABLE profiles
    ADD CONSTRAINT fk_profiles_users
    FOREIGN KEY (user_id) REFERENCES users(id)
""")
```

### Issue 7: Memory Usage with Large Eager Loads

**Symptom**: High memory usage when loading many rows with relations.

**Cause**: Loading too much data into memory at once.

**Solution**:
```python
# ❌ BAD: Loading 100,000 rows with relations
users = await fetch_many_with_relations(
    "users",
    relations=[...],
    limit=100000  # Too much data
)

# ✅ GOOD: Use pagination
batch_size = 1000
offset = 0

while True:
    users = await fetch_many_with_relations(
        "users",
        relations=[...],
        limit=batch_size,
        offset=offset
    )

    if not users:
        break

    # Process batch
    for user in users:
        process_user(user)

    offset += batch_size
```

### Issue 8: Transaction Deadlock

**Error**: `deadlock detected`

**Cause**: Two transactions waiting for each other to release locks.

**Solution**:
```python
# ❌ BAD: Acquiring locks in different orders
# Transaction 1:
# UPDATE users WHERE id = 1
# UPDATE posts WHERE id = 2

# Transaction 2:
# UPDATE posts WHERE id = 2  (waits for Transaction 1)
# UPDATE users WHERE id = 1  (deadlock!)

# ✅ GOOD: Acquire locks in consistent order
from data_bridge.postgres.transactions import pg_transaction

async with pg_transaction() as tx:
    # Always lock in the same order (users before posts)
    await tx.execute("SELECT * FROM users WHERE id = $1 FOR UPDATE", [1])
    await tx.execute("SELECT * FROM posts WHERE id = $1 FOR UPDATE", [2])

    # Perform updates
    await tx.execute("UPDATE users SET name = $1 WHERE id = $2", ["Alice", 1])
    await tx.execute("UPDATE posts SET title = $1 WHERE id = $2", ["New Title", 2])

    await tx.commit()
```

---

## Best Practices

1. **Use CASCADE for parent-child relationships**: When child data has no meaning without the parent (e.g., User → Posts).

2. **Use RESTRICT for protected data**: When deletion should be prevented if dependencies exist (e.g., Product → Orders).

3. **Prefer eager loading for known relationships**: Avoid N+1 queries by using `fetch_many_with_relations()`.

4. **Use lazy loading for optional relationships**: Only fetch related data when needed with `ForeignKeyProxy`.

5. **Use savepoints for complex workflows**: Break down long transactions into smaller steps with rollback points.

6. **Check backreferences before deletion**: Use `get_backreferences()` to understand dependencies.

7. **Use transactions for multi-step operations**: Ensure atomicity with `pg_transaction()`.

8. **Paginate large result sets**: Don't load thousands of rows at once.

---

## References

- [PostgreSQL Transaction Support](../postgres_transactions.md)
- [PostgreSQL ORM Design](../postgres_orm_design.md)
- [Raw SQL Execution](../postgres_raw_sql.md)
- [Migration System](../postgres_migrations.md)
