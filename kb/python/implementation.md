# Python Layer Implementation

> Part of [Python API Layer Documentation](./README.md)

## Metaclass Magic (`document.py`)

The `DocumentMeta` class is the engine behind the declarative API. It runs when a class is *defined* (at import time).

```python
class DocumentMeta(type):
    def __new__(mcs, name, bases, namespace):
        # 1. Extract settings
        settings = namespace.get("Settings")
        
        # 2. Parse annotations
        annotations = namespace.get("__annotations__", {})
        
        # 3. Create FieldProxy descriptors
        for field_name, field_type in annotations.items():
            namespace[field_name] = FieldProxy(field_name)
            
        # 4. Create class
        cls = super().__new__(mcs, name, bases, namespace)
        
        # 5. Extract Schema for Rust
        schema = extract_schema(cls)
        register_model(cls, schema)
        
        return cls
```

## Type Extraction (`type_extraction.py`)

We need to convert complex Python types into something Rust understands.

**Mapping Strategy**:

| Python Type | BSON/Rust Requirement |
| :--- | :--- |
| `str` | String |
| `int` | Int32 or Int64 |
| `float` | Double |
| `bool` | Boolean |
| `datetime` | Date |
| `PydanticObjectId` | ObjectId |
| `List[T]` | Array (validated contents) |
| `Optional[T]` | Nullable |

The `extract_schema` function recursively walks the type tree.

```python
def extract_schema(cls):
    schema = {}
    for name, type_hint in get_type_hints(cls).items():
        schema[name] = python_to_bson_type(type_hint)
    return schema
```

## Helper Functions

### `_engine.py`
This module contains the `find_as_documents` optimization.

```python
# Optimized path: Rust returns instantiated objects directly
def find_as_documents(cls, query):
    # Pass the class constructor to Rust?
    # Currently: Rust returns dicts, we instantiate in Python (fast path)
    # Future: Rust calls constructor via PyO3 (faster path)
    raw_docs = _rust_engine.find(query)
    return [cls._from_db(d) for d in raw_docs]
```

## Link Resolution

Links are stored as `DBRef` or `ObjectId` in MongoDB.

```python
class Link(Generic[T]):
    def __init__(self, ref: Union[DBRef, ObjectId], model_class: Type[T]):
        self.ref = ref
        self.model_class = model_class
        self._cache = None

    async def fetch(self) -> T:
        if self._cache:
            return self._cache
        doc = await self.model_class.get(self.ref.id)
        self._cache = doc
        return doc
```

*Note: This is a simplified view. Actual implementation handles lists of links and back-links.*
