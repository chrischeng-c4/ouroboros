---
title: Data Types & API
status: planning
component: kv-store
type: api-spec
---

# Data Types & API Specification

The KV Store provides a strict type system to ensure data integrity across the Python/Rust boundary, along with a rich API for standard and atomic operations.

## 1. Supported Data Types

Unlike Redis which treats everything as a binary string, this store maintains type information to optimize storage and allow native operations (like incrementing an integer or high-precision decimal math).

| Type | Rust Representation | Python Mapping | Description |
| :--- | :--- | :--- | :--- |
| **Key** | `SmallString<256>` | `str` | Max 256 chars. Optimized stack allocation for short keys. |
| **String** | `String` | `str` | UTF-8 encoded text. |
| **Bytes** | `Vec<u8>` | `bytes` | Raw binary data. |
| **Int** | `i64` | `int` | Signed 64-bit integer. |
| **Float** | `f64` | `float` | 64-bit IEEE floating point. |
| **Decimal** | `Decimal` | `decimal.Decimal` | 128-bit fixed-point. Critical for financial accuracy. |
| **List** | `Vec<Value>` | `list` | Ordered collection. Can contain mixed types. |
| **Map** | `HashMap<String, Value>` | `dict` | Key-Value pairs (String keys only). |

## 2. Python API (`data_bridge.kv`)

The Python API is designed to be async-first, matching the high-performance nature of the backend.

### Initialization

```python
from data_bridge.kv import KVStore, Durability

# Connect/Create a store
store = KVStore(
    path="./my_data_store", 
    max_memory="1GB", 
    durability=Durability.WAL_ASYNC
)
```

### Basic Operations

```python
# Set a value with optional TTL (seconds)
await store.set("user:101:name", "Alice", ttl=3600)

# Get a value (returns None if missing)
name = await store.get("user:101:name")

# Check existence
exists = await store.exists("user:101:name")

# Delete
await store.delete("user:101:name")
```

### Numeric Operations (Type-Aware)

```python
# Atomic Increment (only works on Int types)
new_count = await store.incr("task_counter", 1)

# Decimal support
from decimal import Decimal
await store.set("wallet:balance", Decimal("100.50"))
```

### Atomic State Transitions (CAS)

Essential for Task Store implementations to prevent race conditions.

```python
# Compare-And-Swap
# Only update state to 'STARTED' if it is currently 'PENDING'
success = await store.cas(
    key="task:uuid:state",
    expected="PENDING",
    new_value="STARTED"
)

if success:
    print("Task claimed successfully")
else:
    print("Task already started by another worker")
```

## 3. Rust Interface (`data-bridge-kv`)

The internal Rust API used by the PyO3 bindings.

```rust
pub struct KvStore { ... }

impl KvStore {
    /// Create a new store instance
    pub fn new(config: StoreConfig) -> Self;

    /// Get a value (Transparently fetches from disk if cold)
    pub async fn get(&self, key: &str) -> Result<Option<Value>>;

    /// Set a value
    pub async fn set(&self, key: &str, value: Value, options: SetOptions) -> Result<()>;

    /// Atomic Compare-And-Swap
    pub async fn cas(&self, key: &str, old_val: Option<&Value>, new_val: Value) -> Result<bool>;
}
```

## 4. Error Handling

Errors are raised as standard Python exceptions where appropriate.

- `KeyTooLongError`: Key exceeds 256 characters.
- `TypeError`: Attempting `incr` on a non-integer value.
- `StoreError`: IO errors (disk full, permission denied).
