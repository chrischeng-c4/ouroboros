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
| **Null** | `None` | `None` | Null value. |

## 2. Python API (`data_bridge.kv`)

The Python API is designed to be async-first, matching the high-performance nature of the backend.

### Initialization

```python
from data_bridge.kv import KvClient

# Connect to a store (async context manager)
async with KvClient.connect("127.0.0.1:6380") as client:
    # Operations...
    pass
```

### Basic Operations

```python
# Set a value with optional TTL (seconds)
await client.set("user:101:name", "Alice", ttl=3600)

# Get a value (returns None if missing)
name = await client.get("user:101:name")

# Check existence
exists = await client.exists("user:101:name")

# Delete
await client.delete("user:101:name")
```

### Numeric Operations (Type-Aware)

```python
# Atomic Increment (only works on Int types)
new_count = await client.incr("task_counter", 1)

# Decimal support
from decimal import Decimal
await client.set("wallet:balance", Decimal("100.50"))
```

### Distributed Locking

The store supports distributed locking with auto-expiration to prevent deadlocks.

```python
# Acquire a lock
# Returns True if acquired, False if already held by another
is_locked = await client.lock("resource_lock", owner="worker-1", ttl=30.0)

if is_locked:
    try:
        # Critical section...
        pass
    finally:
        # Release the lock
        await client.unlock("resource_lock", owner="worker-1")

# Or use the context manager helper
from data_bridge.kv import Lock

async with Lock(client, "resource_lock", "worker-1", ttl=30.0) as acquired:
    if acquired:
        # Critical section
        pass
```

### Atomic State Transitions (CAS) - [Planned]

*Note: This feature is specified but currently not implemented in the server.*

Essential for Task Store implementations to prevent race conditions.

```python
# Compare-And-Swap
# Only update state to 'STARTED' if it is currently 'PENDING'
success = await client.cas(
    key="task:uuid:state",
    expected="PENDING",
    new_value="STARTED"
)
```

## 3. Rust Interface (`data-bridge-kv`)

The internal Rust API used by the PyO3 bindings.

```rust
pub struct KvEngine { ... }

impl KvEngine {
    /// Create a new store instance
    pub fn new(num_shards: usize) -> Self;

    /// Get a value
    pub fn get(&self, key: &KvKey) -> Option<KvValue>;

    /// Set a value
    pub fn set(&self, key: &KvKey, value: KvValue, ttl: Option<Duration>);

    /// Atomic Increment
    pub fn incr(&self, key: &KvKey, delta: i64) -> Result<i64, KvError>;
}
```

## 4. Error Handling

Errors are raised as standard Python exceptions where appropriate.

- `ConnectionError`: Unable to connect to server.
- `ValueError`: Invalid arguments (e.g., key too long).
- `TypeError`: Attempting operation on wrong type (e.g. `incr` on string).