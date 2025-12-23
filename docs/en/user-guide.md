# User Guide

`data-bridge` is a high-performance MongoDB ORM for Python, powered by a Rust backend. It provides a Beanie-compatible API while handling all BSON serialization and CPU-intensive tasks in Rust, offering significant performance improvements.

## Getting Started

First, initialize the connection to your MongoDB instance. This is typically done at application startup.

```python
import asyncio
from data_bridge import init

async def main():
    # Initialize with connection string and database name
    await init("mongodb://localhost:27017/my_database")

if __name__ == "__main__":
    asyncio.run(main())
```

## Defining Models

Models are defined by inheriting from `Document`. You can use standard Python type hints.

```python
from typing import Optional
from data_bridge import Document, Indexed

class User(Document):
    name: str
    email: Indexed(str, unique=True)  # Create a unique index
    age: int = 0
    is_active: bool = True
    
    class Settings:
        name = "users"  # Collection name
```

### Settings Configuration
The `Settings` inner class configures the model:

*   `name`: Collection name (defaults to class name in lowercase)
*   `indexes`: List of index definitions
*   `use_revision`: Enable optimistic locking via `_revision_id`
*   `is_root`: Mark as root for document inheritance

## CRUD Operations

### Create

Create a new document instance and save it.

```python
user = User(name="Alice", email="alice@example.com", age=30)
await user.save()
```

### Read

Find documents by ID or other criteria.

```python
# Find by ID
user = await User.get("507f1f77bcf86cd799439011")

# Find one by field
user = await User.find_one(User.email == "alice@example.com")
```

### Update

Modify fields and save changes.

```python
user.age = 31
await user.save()

# Update using query (without fetching first)
await User.find(User.name == "Alice").update({"$set": {"age": 32}})
```

### Delete

Delete a document instance or match documents.

```python
# Delete instance
await user.delete()

# Delete by query
await User.find(User.is_active == False).delete()
```

## Querying

`data-bridge` supports a fluent, chainable query API with type-safe expressions.

### Basic Filtering

```python
# Exact match
users = await User.find(User.age == 30).to_list()

# Comparison operators
users = await User.find(User.age > 25).to_list()
users = await User.find(User.age <= 50).to_list()

# Multiple conditions (AND)
users = await User.find(
    User.age > 25,
    User.is_active == True
).to_list()
```

### Sorting, Skipping, and Limiting

```python
users = await User.find(User.is_active == True) \
    .sort(-User.age) \
    .skip(10) \
    .limit(20) \
    .to_list()
```

*   `.sort(+User.field)`: Ascending
*   `.sort(-User.field)`: Descending

### Projections

Fetch only specific fields to save bandwidth.

```python
# Include only name and email
users = await User.find().project(name=1, email=1).to_list()
```

## Bulk Operations

Perform multiple write operations efficiently using the fluent bulk API. All operations are processed in Rust.

```python
from data_bridge import UpdateOne, InsertOne, DeleteOne

await User.bulk_write([
    # Insert a new user
    InsertOne(User(name="Bob", email="bob@example.com")),
    
    # Update existing user
    UpdateOne(User.email == "alice@example.com")
        .set(User.status, "vip")
        .inc(User.login_count, 1),
        
    # Delete inactive users
    DeleteOne(User.last_login < "2023-01-01")
])
```

## Advanced Models

### Embedded Documents

You can nest documents within other documents using `EmbeddedDocument`. Unlike `Document`, these don't have their own collection.

```python
from data_bridge import Document, EmbeddedDocument

class Address(EmbeddedDocument):
    city: str
    zip_code: str
    street: str | None = None

class User(Document):
    name: str
    address: Address

    class Settings:
        name = "users"

# Usage
user = User(
    name="Alice",
    address=Address(city="NYC", zip_code="10001")
)
await user.save()
```

### Constraints and Validation

`data-bridge` supports field-level validation using `typing.Annotated`. Validation is performed in the Rust backend for high performance.

```python
from typing import Annotated
from data_bridge import Document, MinLen, MaxLen, Min, Max, Email, Url

class Product(Document):
    name: Annotated[str, MinLen(3), MaxLen(100)]
    price: Annotated[float, Min(0.0)]
    contact_email: Annotated[str, Email()]
    website: Annotated[Optional[str], Url()] = None

    class Settings:
        use_validation = True # Enable validation on save
```

---

## Relations (Links)

`data-bridge` provides Beanie-compatible document linking.

### One-to-One / Many-to-One
Use `Link[T]` to reference another document.

```python
from data_bridge import Document, Link

class User(Document):
    name: str

class Post(Document):
    title: str
    author: Link[User]

# Linking
user = await User.find_one(User.name == "Alice")
post = Post(title="Hello World", author=user)
await post.save()

# Fetching with links resolved
post = await Post.find_one(Post.title == "Hello World", fetch_links=True)
print(post.author.name) # "Alice"
```

### One-to-Many
Use `BackLink[T]` to define the reverse relationship.

```python
from data_bridge import Document, BackLink

class User(Document):
    name: str
    # References to Posts that point to this user
    posts: BackLink["Post"] = BackLink(document_class="Post", link_field="author")

# Accessing
user = await User.find_one(User.name == "Alice", fetch_links=True)
for post in user.posts:
    print(post.title)
```

---

## Programmatic Migrations

`data-bridge` supports programmatic migrations to evolve your schema.

```python
from data_bridge.migrations import Migration, iterative_migration, run_migrations

@iterative_migration(User, batch_size=50)
class NormalizeEmails(Migration):
    version = "001"
    description = "Lowercase all email addresses"

    async def transform(self, user: User) -> User:
        user.email = user.email.lower()
        return user

# Run all pending migrations
await run_migrations([NormalizeEmails])
```

---

## Time-Series Collections

For high-frequency data, use MongoDB's native time-series collections.

```python
from datetime import datetime
from data_bridge import Document
from data_bridge.timeseries import TimeSeriesConfig, Granularity

class Measurement(Document):
    timestamp: datetime
    sensor_id: str
    value: float

    class Settings:
        name = "measurements"
        timeseries = TimeSeriesConfig(
            time_field="timestamp",
            meta_field="sensor_id",
            granularity=Granularity.seconds,
            expire_after_seconds=86400 * 7 # 7 days TTL
        )
```

---

## HTTP Client

The library includes a high-performance async HTTP client backed by Rust (`reqwest`), which bypasses the GIL for maximum throughput.

```python
from data_bridge.http import HttpClient

client = HttpClient(
    base_url="https://api.example.com",
    timeout=30.0
)

# Async GET request
response = await client.get("/users/123")

if response.is_success():
    data = response.json()
    print(f"User: {data['name']}")
    print(f"Latency: {response.latency_ms}ms")
```

