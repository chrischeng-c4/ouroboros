# ouroboros

A unified **high-performance Python platform** built entirely in Rust, combining:

- **API Framework** - FastAPI alternative with Rust-powered routing and validation
- **Validation** - Pydantic-compatible BaseModel with zero dependencies
- **MongoDB ORM** - Zero Python byte handling (1.4-5.4x faster than Beanie)
- **Spreadsheet Engine** - WebAssembly-powered with real-time collaboration
- **HTTP Client**, **Task Queue**, **KV Store** - All powered by Rust

## Why Ouroboros?

**The Problem**: Python's flexibility comes at a cost - runtime overhead, GIL contention, and memory pressure from data processing.

**The Solution**: Ouroboros moves all heavy lifting to Rust while keeping Python's ergonomic API. You write Python, Rust does the work.

```
Traditional Stack          Ouroboros Stack
─────────────────          ───────────────
FastAPI                    ouroboros.api (Rust routing)
Pydantic                   ouroboros.validation (Rust validation)
PyMongo/Beanie            ouroboros.mongodb (Rust BSON)
requests/httpx            ouroboros.http (Rust HTTP)
```

## Overview

`ouroboros` provides a complete platform for building high-performance Python applications:

| Module | Description | Status |
|--------|-------------|--------|
| `ouroboros.api` | FastAPI-compatible web framework | Stable |
| `ouroboros.validation` | Pydantic v2 style BaseModel | Stable |
| `ouroboros.mongodb` | Beanie-compatible MongoDB ORM | Stable |
| `ouroboros.http` | High-performance HTTP client | Stable |
| `ouroboros.test` | Native test framework | Stable |
| `ouroboros.postgres` | PostgreSQL support | Beta |
| `ouroboros.kv` | Key-Value store | Beta |
| `ouroboros.tasks` | Task queue (NATS/Redis) | Planned |

## Performance

### MongoDB ORM Benchmarks (1000 documents, 3 iterations)

| Operation | Library | Avg (ms) | ops/sec | Comparison |
|-----------|---------|----------|---------|------------|
| INSERT_MANY | **ouroboros** | 17.76 | 56,309 | **fastest** |
| INSERT_MANY | PyMongo Async+DC | 49.26 | 20,302 | 2.8x slower |
| INSERT_MANY | Beanie | 57.53 | 17,381 | 3.2x slower |
| FIND | **ouroboros** | 6.32 | 158,247 | **fastest** |
| FIND | PyMongo Async+DC | 6.42 | 155,654 | ~same |
| FIND | Beanie | 8.58 | 116,517 | 1.4x slower |

### Why Rust is Faster

```
MongoDB → BSON bytes → Python bytes → PyMongo → Beanie models  (Traditional)
          ↑ Memory pressure in Python heap ↑

MongoDB → BSON bytes → Rust structs → Python objects  (Ouroboros)
          ↑ Processed in Rust, minimal Python heap ↑
```

Run benchmarks yourself:
```bash
MONGODB_URI="mongodb://localhost:27017/bench" uv run python python/benchmarks/bench_comparison.py
```

## Installation

```bash
# Install with uv (recommended)
uv add ouroboros-kit

# Install from source (requires Rust toolchain)
pip install maturin
cd ouroboros && maturin develop --release
```

## Quick Start

### API Framework (ouroboros.api)

```python
from typing import Annotated
from ouroboros.api import App, Path, Query, Body, JSONResponse
from ouroboros.validation import BaseModel, Field

app = App(title="My API", version="1.0.0")

# Define models with Pydantic v2 style syntax
class UserCreate(BaseModel):
    name: Annotated[str, Field(min_length=1, max_length=100)]
    email: Annotated[str, Field(pattern=r"^[\w\.-]+@[\w\.-]+\.\w+$")]
    age: Annotated[int, Field(ge=0, le=150)] = 0

class UserResponse(BaseModel):
    id: str
    name: str
    email: str

# Route handlers with automatic validation
@app.post("/users", response_model=UserResponse)
async def create_user(user: Annotated[UserCreate, Body()]) -> dict:
    # user is already validated by Rust
    return {"id": "123", "name": user.name, "email": user.email}

@app.get("/users/{user_id}")
async def get_user(
    user_id: Annotated[str, Path()],
    include_details: Annotated[bool, Query()] = False
) -> dict:
    return {"id": user_id, "name": "Alice"}

# Run with: uv run python app.py
if __name__ == "__main__":
    import asyncio
    asyncio.run(app.serve(host="0.0.0.0", port=8000))
```

### MongoDB ORM (ouroboros.mongodb)

```python
from ouroboros.mongodb import Document, init

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

### Validation (ouroboros.validation)

```python
from typing import Annotated, List, Optional
from ouroboros.validation import BaseModel, Field, ValidationError

# Pydantic v2 style with Annotated syntax
class Address(BaseModel):
    street: Annotated[str, Field(min_length=1)]
    city: Annotated[str, Field(min_length=1)]
    country: str = "USA"

class User(BaseModel):
    name: Annotated[str, Field(min_length=1, max_length=100)]
    email: Annotated[str, Field(pattern=r"^[\w\.-]+@[\w\.-]+\.\w+$")]
    age: Annotated[int, Field(ge=0, le=150)]
    address: Address
    tags: List[str] = []

# Create with validation (happens in Rust!)
user = User(
    name="Alice",
    email="alice@example.com",
    age=30,
    address={"street": "123 Main St", "city": "NYC"}
)

# Nested models work too
print(user.address.city)  # "NYC"

# Convert to dict
data = user.model_dump()

# Get JSON schema (for OpenAPI)
schema = User.model_json_schema()

# Validation errors are structured
try:
    invalid = User(name="", email="invalid", age=-1, address={"street": "", "city": ""})
except ValidationError as e:
    print(e.errors)  # [{"loc": ["name"], "msg": "...", "type": "..."}]
```

**Key Feature**: Zero pydantic dependency! `ouroboros.validation` provides Pydantic-compatible API without requiring pydantic installation.

---

## API Reference

### ouroboros.api - Web Framework

#### App Class

```python
from ouroboros.api import App

app = App(
    title="My API",
    version="1.0.0",
    description="API description",
    docs_url="/docs",       # Swagger UI
    redoc_url="/redoc",     # ReDoc
    openapi_url="/openapi.json"
)
```

#### Route Decorators

```python
@app.get("/path")           # GET request
@app.post("/path")          # POST request
@app.put("/path")           # PUT request
@app.patch("/path")         # PATCH request
@app.delete("/path")        # DELETE request

# With response model (filters output)
@app.get("/users/{id}", response_model=UserPublic)
async def get_user(id: str) -> UserInternal:
    # UserInternal has password_hash, but response only has UserPublic fields
    return await fetch_user(id)
```

#### Parameter Types

```python
from typing import Annotated
from ouroboros.api import Path, Query, Body, Header

@app.get("/users/{user_id}")
async def get_user(
    user_id: Annotated[str, Path(description="User ID")],
    limit: Annotated[int, Query(ge=1, le=100)] = 10,
    x_token: Annotated[str, Header()] = None
) -> dict:
    return {"id": user_id}

@app.post("/users")
async def create_user(
    user: Annotated[UserCreate, Body()]
) -> dict:
    return user.model_dump()
```

#### Dependency Injection

```python
from ouroboros.api import Depends

async def get_db():
    db = await connect_db()
    try:
        yield db
    finally:
        await db.close()

async def get_current_user(db: Annotated[Database, Depends(get_db)]):
    return await db.get_user()

@app.get("/me")
async def get_me(user: Annotated[User, Depends(get_current_user)]):
    return user
```

#### Middleware

```python
from ouroboros.api import CORSMiddleware, CORSConfig, TimingMiddleware

# CORS support
app.add_middleware(CORSMiddleware(CORSConfig(
    allow_origins=["http://localhost:3000"],
    allow_methods=["GET", "POST"],
    allow_headers=["Authorization"],
    allow_credentials=True
)))

# Request timing
app.add_middleware(TimingMiddleware())

# Custom middleware
from ouroboros.api import BaseMiddleware

class AuthMiddleware(BaseMiddleware):
    async def __call__(self, request, call_next):
        token = request.headers.get("Authorization")
        if not token:
            return JSONResponse({"error": "Unauthorized"}, status_code=401)
        return await call_next(request)
```

#### WebSocket Support

```python
from ouroboros.api import WebSocket, WebSocketDisconnect

@app.websocket("/ws")
async def websocket_handler(ws: WebSocket):
    await ws.accept()
    try:
        while True:
            data = await ws.receive_text()
            await ws.send_text(f"Echo: {data}")
    except WebSocketDisconnect:
        print("Client disconnected")
```

#### Server-Sent Events

```python
from ouroboros.api import EventSourceResponse, ServerSentEvent
import asyncio

@app.get("/events")
async def events():
    async def generate():
        for i in range(10):
            yield ServerSentEvent(data=f"Event {i}", event="update")
            await asyncio.sleep(1)
    return EventSourceResponse(generate())
```

#### Background Tasks

```python
from ouroboros.api import BackgroundTasks

@app.post("/send-email")
async def send_email(
    email: str,
    background: BackgroundTasks
):
    background.add_task(send_email_async, email)
    return {"status": "queued"}
```

#### Health Checks

```python
from ouroboros.api import HealthManager, HealthCheck, HealthStatus

health = HealthManager()

@health.check("database")
async def check_db():
    try:
        await db.ping()
        return HealthStatus.HEALTHY
    except:
        return HealthStatus.UNHEALTHY

app.include_health(health, path="/health")
```

---

### ouroboros.validation - Data Validation

#### BaseModel

```python
from ouroboros.validation import BaseModel, Field
from typing import Annotated, Optional, List

class User(BaseModel):
    # Required field with constraints
    name: Annotated[str, Field(min_length=1, max_length=100)]

    # Optional field with default
    bio: Annotated[Optional[str], Field(max_length=500)] = None

    # Numeric constraints
    age: Annotated[int, Field(ge=0, le=150)] = 0

    # List field
    tags: List[str] = []

# Instance methods
user = User(name="Alice")
user.model_dump()           # Convert to dict
User.model_json_schema()    # Get JSON schema
```

#### Field Constraints

```python
from ouroboros.validation import Field

# String constraints
Field(min_length=1)         # Minimum length
Field(max_length=100)       # Maximum length
Field(pattern=r"^\d+$")     # Regex pattern

# Numeric constraints
Field(gt=0)                 # Greater than
Field(ge=0)                 # Greater than or equal
Field(lt=100)               # Less than
Field(le=100)               # Less than or equal
Field(multiple_of=5)        # Must be multiple of

# Collection constraints
Field(min_items=1)          # Minimum items
Field(max_items=10)         # Maximum items

# Metadata
Field(description="User's name")
Field(example="Alice")
Field(title="Full Name")
```

#### Nested Models

```python
class Address(BaseModel):
    street: str
    city: str
    country: str = "USA"

class Company(BaseModel):
    name: str
    address: Address

class User(BaseModel):
    name: str
    company: Company
    addresses: List[Address] = []

# Nested dict is automatically converted
user = User(
    name="Alice",
    company={"name": "Acme", "address": {"street": "123 Main", "city": "NYC"}},
    addresses=[{"street": "456 Oak", "city": "LA"}]
)
```

#### Validation Errors

```python
from ouroboros.validation import ValidationError

try:
    user = User(name="", age=-1)
except ValidationError as e:
    print(e.errors)
    # [
    #   {"loc": ["name"], "msg": "String too short", "type": "string_too_short"},
    #   {"loc": ["age"], "msg": "Value must be >= 0", "type": "value_error"}
    # ]
```

---

### ouroboros.mongodb - MongoDB ORM

#### Document Base Class

```python
from ouroboros.mongodb import Document

class User(Document):
    email: str
    name: str
    age: int = 0
    tags: list[str] = []

    class Settings:
        name = "users"           # Collection name
```

#### Class Methods (CRUD)

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

#### Instance Methods

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

#### Query Expressions

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

#### Text Search

```python
from ouroboros.mongodb import text_search

# Full-text search (requires text index)
users = await User.find(text_search("python rust")).to_list()

# With language and case sensitivity
users = await User.find(
    text_search("developpeur", language="fr", case_sensitive=False)
).to_list()
```

#### Geospatial Queries

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

#### QueryBuilder

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

#### Bulk Operations

```python
from ouroboros.mongodb import (
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

#### Aggregation

```python
from ouroboros.mongodb import AggregationBuilder

# Build aggregation pipeline
pipeline = AggregationBuilder(User) \
    .match(User.status == "active") \
    .group({"_id": "$department", "count": {"$sum": 1}}) \
    .sort({"count": -1}) \
    .limit(10)

results = await pipeline.to_list()
```

#### Index Management

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

#### Lifecycle Hooks

```python
from ouroboros.mongodb import before_event, after_event, Insert, Save, Delete

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

---

### ouroboros.http - HTTP Client

```python
from ouroboros.http import HttpClient

async with HttpClient() as client:
    # GET request
    response = await client.get("https://api.example.com/users")

    # POST with JSON body
    response = await client.post(
        "https://api.example.com/users",
        json={"name": "Alice", "email": "alice@example.com"}
    )

    # With headers
    response = await client.get(
        "https://api.example.com/protected",
        headers={"Authorization": "Bearer token"}
    )
```

---

### ouroboros.test - Test Framework

**ouroboros includes a native test framework** built in Rust for superior performance.

```python
from ouroboros.test import TestSuite, test, expect

class TestUser(TestSuite):
    @test
    def test_create_user(self):
        user = User(name="Alice", age=30)
        expect(user.name).to_equal("Alice")
        expect(user.age).to_be_greater_than(0)

    @test
    async def test_async_operation(self):
        result = await async_function()
        expect(result).to_be_truthy()

if __name__ == "__main__":
    import asyncio
    asyncio.run(TestUser().run())
```

Run tests:
```bash
uv run python python/tests/unit/test_*.py
```

---

## Migration from Beanie

ouroboros provides a Beanie-compatible API for easy migration:

### Compatible APIs

| Feature | Beanie | ouroboros | Notes |
|---------|--------|-----------|-------|
| Document base class | yes | yes | Same API |
| Query expressions | yes | yes | `User.field == value` |
| find/find_one | yes | yes | Same API |
| save/insert/delete | yes | yes | Same API |
| QueryBuilder | yes | yes | sort/skip/limit/to_list |
| Bulk operations | yes | yes | Same API |
| Aggregation | yes | yes | Same API |
| Lifecycle hooks | yes | yes | @before_event/@after_event |
| Index management | yes | yes | create_index/list_indexes |
| Text search | yes | yes | text_search() |
| Geo queries | yes | yes | near/geo_within |

### Migration Steps

1. **Change imports**:
   ```python
   # Before (Beanie)
   from beanie import Document, init_beanie

   # After (ouroboros)
   from ouroboros.mongodb import Document, init
   ```

2. **Update init call**:
   ```python
   # Before (Beanie)
   await init_beanie(database=db, document_models=[User, Post])

   # After (ouroboros) - models auto-registered
   await init("mongodb://localhost:27017/mydb")
   ```

3. **Run tests** - Most code should work without changes!

---

## Migration from Pydantic

ouroboros.validation provides Pydantic-compatible API without the pydantic dependency:

```python
# Before (Pydantic)
from pydantic import BaseModel, Field
from typing import Annotated

class User(BaseModel):
    name: Annotated[str, Field(min_length=1)]
    age: Annotated[int, Field(ge=0)]

# After (ouroboros) - Same syntax!
from ouroboros.validation import BaseModel, Field
from typing import Annotated

class User(BaseModel):
    name: Annotated[str, Field(min_length=1)]
    age: Annotated[int, Field(ge=0)]
```

---

## Spreadsheet Engine

### Installation (Frontend)

```bash
# Install frontend dependencies
cd frontend
pnpm install

# Build WASM module
just build-wasm

# Start development server
just dev-frontend
```

### Basic Usage (TypeScript/React)

```typescript
import { rusheet } from 'rusheet';

// Initialize the engine
await rusheet.init();

// Set cell values
rusheet.setCellValue(0, 0, 'Hello');
rusheet.setCellValue(0, 1, 'World');
rusheet.setCellValue(1, 0, '=A1 & " " & B1');

// Get cell data
const cell = rusheet.getCellData(1, 0);
console.log(cell.displayValue); // "Hello World"

// Subscribe to changes
rusheet.onChange((event) => {
  console.log(`Cell ${event.row},${event.col} changed to ${event.newValue}`);
});
```

### React Component

```tsx
import { RuSheet, useRuSheet } from 'rusheet/react';

function App() {
  const { ref, api } = useRuSheet();

  return (
    <RuSheet
      ref={ref}
      initialData={[
        ['Name', 'Age', 'City'],
        ['Alice', 30, 'NYC'],
        ['Bob', 25, 'LA'],
      ]}
      onChange={(e) => console.log('Changed:', e)}
      width="100%"
      height={500}
    />
  );
}
```

### Features

- **High Performance**: Rust-powered formula engine compiled to WebAssembly
- **Real-time Collaboration**: Multi-user editing with CRDT-based sync (Yjs/yrs)
- **Full Formula Support**: 24+ built-in functions (SUM, IF, VLOOKUP, etc.)
- **Undo/Redo**: Complete history with unlimited undo
- **Event-driven API**: Subscribe to cell changes, selections, and more
- **Zero-copy Rendering**: Direct memory access for optimal performance

See [docs/archive/SHEET_README.md](docs/archive/SHEET_README.md) for complete documentation.

---

## Argus Code Analysis

Argus is a high-performance static analysis engine for Python, built in Rust. It now includes a background daemon and an MCP server for LLM integration.

### Daemon

The Argus Daemon maintains a live, type-aware index of your codebase, providing sub-millisecond response times for queries.

```bash
# Start the daemon
ob argus server
```

### LLM Integration (MCP)

Argus implements the [Model Context Protocol (MCP)](https://modelcontextprotocol.io), allowing tools like Claude Desktop to "see" your code with semantic understanding (types, references, definitions).

**Features:**
- **Deep Type Analysis**: Resolves types across files.
- **Go to Definition**: Finds where symbols are defined.
- **Find References**: Locates usages of symbols.
- **Hover**: Shows documentation and type signatures.

**Setup for Claude Desktop:**

1. Generate the configuration:
   ```bash
   ob argus mcp
   ```

2. Add the output to your Claude Desktop config file.

3. Start the daemon in a separate terminal:
   ```bash
   ob argus server
   ```

To run the MCP server directly (e.g., for direct stdio integration):
```bash
ob argus mcp-server
```

---

## Architecture

### Platform Overview

```
┌─────────────────────────────────────────────────────────────┐
│                   Python Application Layer                   │
│         (ouroboros.api, mongodb, http, validation)           │
└──────────────────────────┬──────────────────────────────────┘
                           │
                   PyO3 Rust Bridge
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                    Rust Core Layer                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  MongoDB ORM │  │  HTTP Client │  │   Validation     │   │
│  │  (BSON/CRUD) │  │  (Reqwest)   │  │   (Type checks)  │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                  Spreadsheet Frontend (Browser)              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Canvas      │  │  Yjs Client │  │   RuSheet API       │  │
│  │ Renderer    │  │  (collab)   │  │   (TypeScript)      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└──────────────────────────┬──────────────────────────────────┘
                           │
                      WASM Bridge
                           │
┌──────────────────────────▼──────────────────────────────────┐
│              Spreadsheet Engine (WASM/Rust)                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ sheet-core   │  │sheet-formula │  │ sheet-history    │   │
│  │ (cells/grid) │  │  (parser)    │  │  (undo/redo)     │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### MongoDB ORM Architecture

```
Python Application Layer
        ↓
ouroboros.mongodb (Beanie-compatible API)
        ↓
PyO3 Rust Extension (PyO3 0.24)
        ↓
Rust MongoDB Driver + BSON
        ↓
MongoDB Server
```

**Key Principle**: All BSON serialization/deserialization happens in Rust - Python receives only typed, validated objects.

### Validation Architecture

**ouroboros uses Rust-only validation for performance and security:**

1. **Python layer**: Type hints for IDE support (NOT runtime validation)
2. **Rust layer**: All runtime validation happens here
3. **Zero Python validation overhead**: No Pydantic, no runtime type checking in Python

**Validation Flow:**
```
User.save() → PyO3 Bridge → Rust Validation → MongoDB
                              ↓
                       - Type checking
                       - Constraint validation
                       - BSON conversion (GIL-free)
```

---

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
cargo test                                           # Rust tests
uv run python python/tests/unit/test_*.py           # Unit tests
uv run python python/tests/integration/test_*.py   # Integration tests

# Security audit
cargo audit
```

### Project Structure

```
ouroboros/
├── crates/                         # Rust workspace
│   ├── data-bridge/                # PyO3 Python bindings (main entry)
│   ├── data-bridge-mongodb/        # Pure Rust MongoDB ORM
│   ├── data-bridge-api/            # API framework core
│   ├── data-bridge-http/           # HTTP client
│   ├── data-bridge-postgres/       # PostgreSQL support
│   ├── data-bridge-test/           # Test framework
│   ├── data-bridge-kv/             # Key-Value store
│   ├── data-bridge-common/         # Shared types and errors
│   │
│   ├── data-bridge-sheet-core/     # Spreadsheet core
│   ├── data-bridge-sheet-db/       # Custom database (Morton encoding)
│   ├── data-bridge-sheet-formula/  # Formula parser & evaluator
│   ├── data-bridge-sheet-history/  # Undo/redo system
│   ├── data-bridge-sheet-server/   # Collaboration server
│   └── data-bridge-sheet-wasm/     # WebAssembly bindings
│
├── python/
│   └── ouroboros/                  # Python API
│       ├── __init__.py             # Public API exports
│       ├── api/                    # Web framework
│       ├── validation/             # BaseModel & Field
│       ├── mongodb/                # MongoDB ORM
│       ├── http/                   # HTTP client
│       └── test/                   # Test framework
│
├── frontend/                       # Spreadsheet frontend
│   ├── src/                        # TypeScript source
│   └── pkg/                        # Built WASM package
│
└── docs/                           # Documentation
```

---

## Testing

```bash
# Run all Python tests
uv run python python/tests/unit/test_*.py
uv run python python/tests/integration/test_*.py

# Run Rust tests
cargo test

# Run benchmarks
uv run python python/benchmarks/bench_comparison.py --rounds 5 --warmup 2

# Run specific test file
uv run python python/tests/unit/test_validation.py
```

---

## License

MIT License - see LICENSE file for details.

---

## Roadmap

### API Framework (ouroboros.api)
- [x] Route decorators (GET, POST, PUT, DELETE, PATCH)
- [x] Path, Query, Body, Header parameters
- [x] Dependency injection
- [x] Middleware support (CORS, timing, logging)
- [x] WebSocket support
- [x] Server-Sent Events
- [x] Background tasks
- [x] OpenAPI documentation (Swagger, ReDoc)
- [x] response_model filtering

### Validation (ouroboros.validation)
- [x] Pydantic v2 Annotated syntax
- [x] Nested model validation
- [x] Rust-powered validation (zero Python overhead)
- [x] JSON schema generation
- [ ] Custom validators

### MongoDB ORM (ouroboros.mongodb)
- [x] Beanie-compatible API
- [x] Bulk operations
- [x] Aggregation pipeline
- [x] Text search and geo queries
- [x] Lifecycle hooks
- [x] Index management
- [ ] Transaction support
- [ ] Link fetching (eager/lazy loading)

### Spreadsheet Engine
- [x] High-performance formula engine (24+ functions)
- [x] Real-time collaboration (CRDT-based)
- [x] Undo/Redo system
- [x] CSV/XLSX import/export
- [x] WebAssembly bindings
- [x] React component wrapper
- [ ] Vue component wrapper
- [ ] Conditional formatting

### Infrastructure
- [x] HTTP client
- [x] Native test framework
- [x] PostgreSQL support (partial)
- [ ] Task queue (NATS/Redis)
- [ ] Key-Value store
