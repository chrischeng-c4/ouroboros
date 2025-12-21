# data-bridge

High-performance MongoDB ORM for Python with Rust backend and **zero Python byte handling**.

## Overview

`data-bridge` is a Beanie-compatible MongoDB ORM that handles all BSON serialization/deserialization in Rust, providing maximum performance and memory efficiency for large-scale data operations.

### Key Features

- **Beanie-Compatible API**: Drop-in replacement for most Beanie operations
- **Zero Python Byte Handling**: All BSON serialization/deserialization happens in Rust
- **2.8-3.2x Faster Inserts**: Measured benchmarks show significant insert performance gains
- **Async/Await Support**: Full asyncio integration via PyO3 + Tokio
- **Type Safe**: Query expressions with compile-time (Rust) and runtime (Pydantic) type checking
- **Full CRUD Support**: insert, find, update, delete with bulk operations
- **Query Builder**: Chainable API with sort, skip, limit, projection
- **Security**: RUSTSEC-2025-0020 fixed (PyO3 0.24+)

## Performance

### Benchmarks (1000 documents, 3 iterations)

| Operation | Library | Avg (ms) | ops/sec | Comparison |
|-----------|---------|----------|---------|------------|
| INSERT_MANY | **data-bridge** | 17.76 | 56,309 | **fastest** |
| INSERT_MANY | PyMongo Async+DC | 49.26 | 20,302 | 2.8x slower |
| INSERT_MANY | Beanie | 57.53 | 17,381 | 3.2x slower |
| FIND | **data-bridge** | 6.32 | 158,247 | **fastest** |
| FIND | PyMongo Async+DC | 6.42 | 155,654 | ~same |
| FIND | Beanie | 8.58 | 116,517 | 1.4x slower |

Run benchmarks yourself:
```bash
MONGODB_URI="mongodb://localhost:27017/bench" uv run python benchmarks/bench_comparison.py
```

## Installation

```bash
# Install with uv (recommended)
uv add data-bridge

# Install from source (requires Rust toolchain)
pip install maturin
cd data-bridge && maturin develop --release
```

## Quick Start

```python
from data_bridge import Document, init

# Initialize connection
await init("mongodb://localhost:27017/mydb")

# Define your model (Beanie-compatible)
class User(Document):
    email: str
    name: str
    age: int

    class Settings:
        name = "users"  # Collection name

# Create and save
user = User(email="alice@example.com", name="Alice", age=30)
await user.save()

# Query with type-safe expressions
alice = await User.find_one(User.email == "alice@example.com")

# Complex queries with QueryBuilder
adults = await User.find(User.age >= 18).sort("-age").limit(10).to_list()

# Update
alice.age = 31
await alice.save()

# Delete
await alice.delete()
```

## API Reference

### Document Base Class

```python
from data_bridge import Document

class User(Document):
    email: str
    name: str
    age: int = 0
    tags: list[str] = []

    class Settings:
        name = "users"           # Collection name
        # validate_on_save = True  # Coming soon
```

### Class Methods (CRUD)

```python
# Create
await User.insert_one(user)
await User.insert_many([user1, user2, user3])

# Read
user = await User.find_one(User.email == "alice@example.com")
user = await User.get("507f1f77bcf86cd799439011")  # By ID
users = await User.find(User.age > 25).to_list()
users = await User.find_all().to_list()
count = await User.count()

# Update
await User.update_one(
    User.email == "alice@example.com",
    {"$set": {"age": 31}}
)
await User.update_many(
    User.status == "inactive",
    {"$set": {"archived": True}}
)

# Delete
await User.delete_one(User.email == "test@example.com")
deleted_count = await User.delete_many(User.status == "deleted")
```

### Instance Methods

```python
user = User(email="bob@example.com", name="Bob", age=25)

# Save (insert or update)
await user.save()

# Update fields
await user.set({"age": 26, "verified": True})
await user.inc({"login_count": 1})  # Increment

# Delete
await user.delete()

# Check if persisted
if user.id:
    print("Document saved to MongoDB")
```

### Query Expressions

```python
# Comparison operators
User.age == 25
User.age != 25
User.age > 25
User.age >= 25
User.age < 25
User.age <= 25

# Array operators
User.status.in_(["active", "pending"])
User.status.not_in(["deleted"])
User.tags.all(["python", "rust"])
User.tags.elem_match({"name": "admin", "level": {"$gte": 5}})

# String operators
User.email.regex(r"@example\.com$")
User.email.regex(r"admin", options="i")  # Case insensitive

# Existence and null
User.middle_name.exists(True)
User.deleted_at.exists(False)

# Combine with AND/OR
(User.age >= 18) & (User.status == "active")
(User.role == "admin") | (User.role == "superuser")
```

### Text Search

```python
from data_bridge import text_search

# Full-text search (requires text index)
users = await User.find(text_search("python rust")).to_list()

# With language and case sensitivity
users = await User.find(
    text_search("développeur", language="fr", case_sensitive=False)
).to_list()
```

### Geospatial Queries

```python
# Find near a point
User.location.near(
    longitude=-73.97,
    latitude=40.77,
    max_distance=5000  # meters
)

# Geo within polygon
User.location.geo_within_polygon([
    [-73.99, 40.73],
    [-73.98, 40.74],
    [-73.97, 40.73],
    [-73.99, 40.73]
])

# Geo within box
User.location.geo_within_box(
    bottom_left=[-74.0, 40.5],
    top_right=[-73.5, 41.0]
)
```

### QueryBuilder

```python
# Chainable query builder
users = await User.find(User.active == True) \
    .sort("-created_at") \
    .skip(10) \
    .limit(20) \
    .to_list()

# Multi-field sort
users = await User.find() \
    .sort([("status", 1), ("created_at", -1)]) \
    .to_list()

# Projection (select fields)
users = await User.find() \
    .project({"email": 1, "name": 1, "_id": 0}) \
    .to_list()

# Count
count = await User.find(User.status == "active").count()

# Delete matching
deleted = await User.find(User.status == "deleted").delete()

# First match (or None)
user = await User.find(User.email == "test@example.com").first()
```

### Bulk Operations

```python
from data_bridge import (
    BulkOperation, UpdateOne, UpdateMany,
    InsertOne, DeleteOne, DeleteMany, ReplaceOne
)

# Create bulk operation
bulk = BulkOperation(User)

# Add operations
bulk.add(InsertOne({"email": "new@example.com", "name": "New User", "age": 25}))
bulk.add(UpdateOne(
    User.email == "alice@example.com",
    {"$set": {"verified": True}},
    upsert=True
))
bulk.add(UpdateMany(
    User.age < 18,
    {"$set": {"minor": True}}
))
bulk.add(DeleteOne(User.status == "deleted"))
bulk.add(DeleteMany(User.last_login < datetime(2020, 1, 1)))
bulk.add(ReplaceOne(
    User.email == "old@example.com",
    {"email": "old@example.com", "name": "Updated", "age": 30}
))

# Execute
result = await bulk.execute(ordered=True)
print(f"Inserted: {result.inserted_count}")
print(f"Modified: {result.modified_count}")
print(f"Deleted: {result.deleted_count}")
print(f"Upserted IDs: {result.upserted_ids}")
```

### Aggregation

```python
from data_bridge import AggregationBuilder

# Build aggregation pipeline
pipeline = AggregationBuilder(User) \
    .match(User.status == "active") \
    .group({"_id": "$department", "count": {"$sum": 1}}) \
    .sort({"count": -1}) \
    .limit(10)

results = await pipeline.to_list()
```

### Distinct Values

```python
# Get distinct values
emails = await User.distinct("email")
departments = await User.distinct("department", User.status == "active")
```

### Index Management

```python
# Create indexes
await User.create_index("email", unique=True)
await User.create_index([("status", 1), ("created_at", -1)])
await User.create_index("bio", name="text_search_index")

# List indexes
indexes = await User.list_indexes()
for idx in indexes:
    print(f"{idx['name']}: {idx['key']}")

# Drop index
await User.drop_index("email_1")
```

### Types and Helpers

```python
from data_bridge import (
    PydanticObjectId,  # ObjectId type for Pydantic
    Indexed,           # Mark field for indexing
    escape_regex,      # Escape regex special chars
)

class User(Document):
    id: PydanticObjectId = None
    email: Indexed(str, unique=True)  # Creates unique index
    name: Indexed(str)                # Creates regular index

    class Settings:
        name = "users"

# Safe regex queries
pattern = escape_regex("user+test@example.com")  # Escapes the +
users = await User.find(User.email.regex(pattern)).to_list()
```

### Document Relations (Links)

```python
from data_bridge import Link, BackLink

class Author(Document):
    name: str

    class Settings:
        name = "authors"

class Book(Document):
    title: str
    author: Link[Author]  # Reference to Author

    class Settings:
        name = "books"

class Publisher(Document):
    name: str
    books: BackLink[Book]  # Reverse reference

    class Settings:
        name = "publishers"

# Note: Link fetching requires Rust implementation (coming soon)
```

### Lifecycle Hooks (Actions)

```python
from data_bridge import before_event, after_event, Insert, Save, Delete

class User(Document):
    email: str
    created_at: datetime = None

    class Settings:
        name = "users"

    @before_event(Insert)
    async def set_created_at(self):
        self.created_at = datetime.utcnow()

    @after_event(Save)
    async def notify_saved(self):
        print(f"User {self.email} saved!")

    @before_event(Delete)
    async def cleanup(self):
        # Clean up related data
        pass
```

### Connection Management

```python
from data_bridge import init, is_connected, close

# Initialize with URI
await init("mongodb://localhost:27017/mydb")

# Check connection status
if is_connected():
    print("Connected to MongoDB")

# Close connection (optional, for cleanup)
await close()
```

## Migration from Beanie

data-bridge provides a Beanie-compatible API for easy migration:

### Compatible APIs

| Feature | Beanie | data-bridge | Notes |
|---------|--------|-------------|-------|
| Document base class | ✅ | ✅ | Same API |
| Query expressions | ✅ | ✅ | `User.field == value` |
| find/find_one | ✅ | ✅ | Same API |
| save/insert/delete | ✅ | ✅ | Same API |
| QueryBuilder | ✅ | ✅ | sort/skip/limit/to_list |
| Bulk operations | ✅ | ✅ | Same API |
| Aggregation | ✅ | ✅ | Same API |
| Lifecycle hooks | ✅ | ✅ | @before_event/@after_event |
| Index management | ✅ | ✅ | create_index/list_indexes |
| Text search | ✅ | ✅ | text_search() |
| Geo queries | ✅ | ✅ | near/geo_within |

### Not Yet Implemented

| Feature | Status | Notes |
|---------|--------|-------|
| Transactions | Stub | Raises TransactionNotSupportedError |
| Link fetching | Stub | Link/BackLink types defined |
| ValidateOnSave | Planned | Use Pydantic validation for now |
| Revision IDs | Planned | For optimistic concurrency |

### Migration Steps

1. **Change imports**:
   ```python
   # Before (Beanie)
   from beanie import Document, init_beanie

   # After (data-bridge)
   from data_bridge import Document, init
   ```

2. **Update init call**:
   ```python
   # Before (Beanie)
   await init_beanie(database=db, document_models=[User, Post])

   # After (data-bridge) - models auto-registered
   await init("mongodb://localhost:27017/mydb")
   ```

3. **Run tests** - Most code should work without changes!

## Architecture

```
Python Application Layer
        ↓
data-bridge (Beanie-compatible API)
        ↓
PyO3 Rust Extension (PyO3 0.24)
        ↓
Rust MongoDB Driver + BSON
        ↓
MongoDB Server
```

**Key Principle**: All BSON serialization/deserialization happens in Rust - Python receives only typed, validated objects.

### Why Rust?

**Current Problem (Python-based):**
```
MongoDB → BSON bytes → Python bytes → PyMongo objects → Beanie models
          ↑ Large datasets create memory pressure in Python heap ↑
```

**data-bridge Solution:**
```
MongoDB → BSON bytes → Rust structs → Python objects
          ↑ Processed in Rust, minimal Python heap usage ↑
```

**Benefits:**
- Direct BSON → Rust struct → Python object (no intermediate bytes in Python heap)
- Parallel processing without GIL contention
- Memory-mapped operations where possible
- Zero-copy deserialization for large documents

### Validation Architecture

**data-bridge uses Rust-only validation for performance and security:**

1. **Python layer**: Type hints for IDE support (NOT runtime validation)
2. **Rust layer**: All runtime validation happens here
3. **Zero Python validation overhead**: No Pydantic, no runtime type checking in Python

**Key Innovation:**
- Python does less: Just type hints for model definition
- Rust does more: All runtime validation (type checking, security, BSON conversion)
- Same developer experience as Pydantic, but 10x faster

**Validation Flow:**
```
User.save() → PyO3 Bridge → Rust Validation → MongoDB
                              ↓
                       - Type checking
                       - Security validation
                       - BSON conversion (GIL-free)
```

See [docs/architecture/validation-flow.md](docs/architecture/validation-flow.md) for detailed documentation.

## Development

### Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Python 3.12+
- uv (`pip install uv`)
- Maturin (`pip install maturin`)

### Build from Source

```bash
# Development build (fast iteration)
maturin develop

# Release build (optimized)
maturin develop --release

# Run tests
cargo test                          # Rust tests
uv run pytest tests/ -v             # Python tests (313 passing)
SKIP_INTEGRATION=true uv run pytest # Unit tests only (no MongoDB)

# Coverage
uv run pytest --cov=data_bridge --cov-report=term-missing

# Security audit
cargo audit
```

### Project Structure

```
data-bridge/
├── crates/
│   ├── data-bridge/           # PyO3 Python bindings
│   │   └── src/
│   │       ├── lib.rs         # Module registration
│   │       └── mongodb.rs     # MongoDB operations (1100+ lines)
│   ├── data-bridge-mongodb/   # Pure Rust MongoDB client
│   └── data-bridge-common/    # Shared types and errors
├── python/
│   └── data_bridge/           # Beanie-compatible Python API
│       ├── __init__.py        # Public API exports
│       ├── document.py        # Document base class with metaclass
│       ├── fields.py          # FieldProxy, QueryExpr, operators
│       ├── query.py           # QueryBuilder, AggregationBuilder
│       ├── actions.py         # Lifecycle hooks
│       ├── bulk.py            # Bulk operations
│       ├── links.py           # Document relations
│       ├── types.py           # PydanticObjectId, Indexed
│       ├── transactions.py    # Transaction stub
│       └── connection.py      # Connection management
├── tests/                     # Test suite (313 tests, 80% coverage)
└── benchmarks/                # Performance benchmarks
```

## Testing

```bash
# All tests (requires MongoDB on localhost:27017)
uv run pytest tests/ -v

# Unit tests only (no MongoDB required)
SKIP_INTEGRATION=true uv run pytest tests/ -v

# With coverage report
uv run pytest tests/ --cov=data_bridge --cov-report=term-missing

# Specific test file
uv run pytest tests/test_comprehensive.py -v
```

## License

MIT License - see LICENSE file for details.

## Roadmap

- [x] MongoDB ORM with Beanie compatibility
- [x] Bulk operations
- [x] Aggregation pipeline
- [x] Text search and geo queries
- [x] Lifecycle hooks
- [x] Index management
- [ ] Transaction support (Rust implementation)
- [ ] Link fetching (eager/lazy loading)
- [ ] Redis client
- [ ] PostgreSQL support
