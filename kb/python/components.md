# Python Layer Components

> Part of [Python API Layer Documentation](./README.md)

## 1. Document (`document.py`)

**Role**: The base class for all user models.

**Key Components**:
- **`DocumentMeta`**: A metaclass that:
    - Parses type hints (`__annotations__`).
    - Creates `FieldProxy` descriptors for each field.
    - Registers the model with the global registry.
    - Extracts collection settings (nested `Settings` class).
- **`id` field**: Automatically manages the `_id` field alias.
- **Lifecycle Methods**: `before_event`, `after_event` hooks.

**Usage**:
```python
class User(Document):
    name: str
    age: int
    
    class Settings:
        name = "users"
```

## 2. Query DSL (`fields.py`)

**Role**: Enables the magic syntax for queries.

**Key Components**:
- **`FieldProxy`**: Represents a field on the class. Overloads operators:
    - `__eq__`, `__ne__`, `__gt__`, `__lt__`, `__ge__`, `__le__`
    - `__add__`, `__sub__` (for updates like `$inc`)
- **`QueryExpr`**: Represents a query expression (e.g., `{"age": {"$gt": 18}}`). Supports logical composition:
    - `&` (AND)
    - `|` (OR)
    - `~` (NOT)

**Example**:
```python
# (User.age > 18) returns a QueryExpr
# (User.name == "Alice") returns a QueryExpr
# The '&' combines them into {"$and": [...]}
```

## 3. Query Builder (`query.py`)

**Role**: Provides the fluent interface for database operations.

**Key Components**:
- **`QueryBuilder`**: The main object returned by `User.find()`.
    - Stores state: `filter`, `sort`, `limit`, `skip`, `projection`.
    - Methods: `.sort()`, `.limit()`, `.project()`, `.to_list()`, `.first()`.
- **`AggregationBuilder`**: For building aggregation pipelines.

**Optimization**:
The builder constructs the query parameters in Python, then passes them efficiently to Rust's `QueryBuilder` for execution.

## 4. State Tracker (`state.py`)

**Role**: Tracks modifications to document instances.

**Key Components**:
- **`StateTracker`**: Attached to every `Document` instance.
- **`__setattr__` intercept**: The `Document` class hooks `__setattr__` to notify the tracker.
- **`get_changes()`**: Returns a dict of `{field: new_value}` for fields that changed.

## 5. Engine Bridge (`_engine.py`)

**Role**: The low-level interface to Rust.

**Key Responsibilities**:
- **`RustDocument` wrapper**: Wraps the PyO3 class.
- **Error Handling**: Translates Rust errors into Python exceptions.
- **Batch Operations**: Handles looping or delegating batch ops to Rust.

## 6. Type Extractor (`type_extraction.py`)

**Role**: Bridges Python types to Rust validation.

**Function**: `extract_schema(cls)`
- Inspects `cls.__annotations__`.
- Handles `Optional`, `List`, `Dict`, `Union`, `Annotated`.
- Returns a schema definition dict passed to Rust at initialization.
