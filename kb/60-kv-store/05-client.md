# KV Client Connection Pool

## Overview

The `KvPool` provides a thread-safe, async connection pool for `KvClient` with automatic connection management, idle timeout, and RAII-based resource cleanup.

## Features

- **Min/Max Pool Size**: Configure minimum and maximum connections
- **Idle Timeout**: Automatically close connections that are unused for too long
- **Acquire Timeout**: Configurable timeout when waiting for available connections
- **Pre-warming**: Initialize pool with `min_size` connections on startup
- **Automatic Return**: RAII guard (`PooledClient`) returns connections on drop
- **Thread-safe**: Use `Arc<KvPool>` for sharing across tasks/threads
- **Statistics**: Real-time pool metrics (idle/active/max)

## Basic Usage

```rust
use data_bridge_kv_client::{KvPool, PoolConfig, KvValue};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create pool with default config
    let pool = KvPool::connect(
        PoolConfig::new("127.0.0.1:6380")
    ).await?;

    // Acquire a connection (returned automatically on drop)
    {
        let mut conn = pool.acquire().await?;
        conn.client().set("key", KvValue::String("value".to_string()), None).await?;
        let value = conn.client().get("key").await?;
        println!("Value: {:?}", value);
    } // Connection automatically returned to pool here

    Ok(())
}
```

## Configuration

```rust
use std::time::Duration;
use data_bridge_kv_client::{KvPool, PoolConfig};

let pool = KvPool::connect(
    PoolConfig::new("127.0.0.1:6380/cache")  // With namespace
        .min_size(5)                         // Keep 5 connections alive
        .max_size(20)                        // Allow up to 20 connections
        .idle_timeout(Duration::from_secs(300))    // Close after 5 min idle
        .acquire_timeout(Duration::from_secs(10))  // Wait up to 10s for connection
).await?;
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `addr` | `127.0.0.1:6380` | Server address (with optional namespace) |
| `min_size` | `2` | Minimum connections to keep alive |
| `max_size` | `10` | Maximum connections allowed |
| `idle_timeout` | `300s` | Close connections unused for this long |
| `acquire_timeout` | `5s` | Timeout when waiting for a connection |

## Pool Statistics

```rust
let stats = pool.stats().await;
println!("Idle: {}, Active: {}, Max: {}",
    stats.idle, stats.active, stats.max_size);
```

## Namespace Support

The pool extracts namespace from the connection string:

```rust
// Pool with namespace "cache"
let pool = KvPool::connect(
    PoolConfig::new("127.0.0.1:6380/cache")
).await?;

assert_eq!(pool.namespace(), Some("cache"));

// All operations are automatically namespaced
let mut conn = pool.acquire().await?;
conn.client().set("key", value, None).await?;  // Actually sets "cache:key"
```

## Concurrent Usage

The pool is designed for high-concurrency scenarios:

```rust
use std::sync::Arc;

let pool = Arc::new(KvPool::connect(
    PoolConfig::new("127.0.0.1:6380")
        .min_size(10)
        .max_size(50)
).await?);

// Spawn multiple tasks
let mut handles = vec![];
for i in 0..100 {
    let pool = Arc::clone(&pool);
    let handle = tokio::spawn(async move {
        let mut conn = pool.acquire().await?;
        conn.client().set(
            &format!("key_{}", i),
            KvValue::Int(i as i64),
            None
        ).await?;
        Ok::<_, data_bridge_kv_client::ClientError>(())
    });
    handles.push(handle);
}

// Wait for all tasks
for handle in handles {
    handle.await??;
}
```

## Connection Lifecycle

1. **Pre-warming**: On pool creation, `min_size` connections are established
2. **Acquisition**:
   - First tries to reuse idle connection
   - Creates new connection if pool not at `max_size`
   - Waits and retries if pool is full
   - Returns `Err(ClientError::Timeout)` if `acquire_timeout` exceeded
3. **Usage**: Client operations via `conn.client().method()`
4. **Return**: Connection automatically returned on `PooledClient` drop
5. **Expiration**: Idle connections closed after `idle_timeout`

## Error Handling

```rust
use data_bridge_kv_client::{ClientError, KvPool, PoolConfig};

match pool.acquire().await {
    Ok(conn) => { /* Use connection */ },
    Err(ClientError::Timeout) => {
        eprintln!("Pool exhausted - all connections in use");
    },
    Err(ClientError::Connection(e)) => {
        eprintln!("Network error: {}", e);
    },
    Err(e) => {
        eprintln!("Other error: {}", e);
    },
}
```

## Best Practices

### 1. Pre-warm for Latency-Sensitive Apps

```rust
let pool = KvPool::connect(
    PoolConfig::new("127.0.0.1:6380")
        .min_size(10)  // Pre-warm 10 connections
        .max_size(20)
).await?;
```

### 2. Size Pool Based on Load

- **Low traffic**: `min_size=2, max_size=10`
- **Medium traffic**: `min_size=10, max_size=50`
- **High traffic**: `min_size=50, max_size=200`

### 3. Tune Timeouts

```rust
PoolConfig::new("127.0.0.1:6380")
    .idle_timeout(Duration::from_secs(600))   // 10 min for long-lived apps
    .acquire_timeout(Duration::from_secs(1))  // Fail fast for low-latency apps
```

### 4. Share Pool Across App

```rust
// In main.rs or app state
pub struct AppState {
    kv_pool: Arc<KvPool>,
}

// In handlers
async fn handler(state: &AppState) -> Result<()> {
    let mut conn = state.kv_pool.acquire().await?;
    conn.client().get("key").await?;
    Ok(())
}
```

### 5. Monitor Pool Health

```rust
// Periodic health check
tokio::spawn({
    let pool = Arc::clone(&pool);
    async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let stats = pool.stats().await;
            if stats.active == stats.max_size {
                eprintln!("WARNING: Pool exhausted!");
            }
        }
    }
});
```

## Architecture

```
┌─────────────────────────────────────┐
│          Application Code           │
└────────────┬────────────────────────┘
             │ acquire()
             ▼
┌─────────────────────────────────────┐
│            KvPool                   │
│  ┌─────────────────────────────┐   │
│  │  Idle Queue (VecDeque)      │   │
│  │  [conn1, conn2, conn3, ...] │   │
│  └─────────────────────────────┘   │
│  Active Count: 5                    │
└────────────┬────────────────────────┘
             │ PooledClient (RAII guard)
             ▼
┌─────────────────────────────────────┐
│          KvClient                   │
│       (TCP connection)              │
└────────────┬────────────────────────┘
             │ Protocol commands
             ▼
┌─────────────────────────────────────┐
│        KV Server (TCP)              │
└─────────────────────────────────────┘
```

## Performance Characteristics

- **Acquire latency**: O(1) for idle connections, O(n) for busy pool (n = wait iterations)
- **Memory overhead**: ~1KB per idle connection
- **Lock contention**: Minimal (short-lived locks on acquire/release)
- **Async drop**: Connection return happens in spawned task (non-blocking)

## Testing

Unit tests (no server required):
```bash
cargo test -p data-bridge-kv-client --lib
```

Integration tests (requires running server):
```bash
# Terminal 1: Start server
cargo run -p data-bridge-kv-server

# Terminal 2: Run tests
cargo test -p data-bridge-kv-client -- --ignored
```

## Comparison with Direct Client

| Scenario | Direct Client | Pool |
|----------|---------------|------|
| **Single request** | Faster (no pool overhead) | Slightly slower |
| **Many sequential requests** | Same connection reused | Same connection reused |
| **Concurrent requests** | Must create multiple clients | Reuses existing connections |
| **Connection setup cost** | Per-request overhead | Amortized via pre-warming |
| **Resource cleanup** | Manual | Automatic |

## Python API

The connection pool is also available from Python via PyO3 bindings:

### Basic Usage

```python
from data_bridge.kv import KvPool, PoolConfig

# Create pool configuration
config = PoolConfig(
    "127.0.0.1:6380",
    min_size=2,
    max_size=10,
    idle_timeout=300.0,
    acquire_timeout=5.0,
)

# Connect with pool
pool = await KvPool.connect(config)

# Use pool for operations
await pool.set("key", "value")
value = await pool.get("key")

# Check pool stats
stats = await pool.stats()
print(f"Idle: {stats.idle}, Active: {stats.active}")
```

### Configuration Options

```python
config = PoolConfig(
    addr="127.0.0.1:6380/cache",  # With namespace
    min_size=5,                    # Minimum connections
    max_size=20,                   # Maximum connections
    idle_timeout=600.0,            # Idle timeout in seconds
    acquire_timeout=10.0,          # Acquire timeout in seconds
)
```

### Concurrent Operations

The pool automatically handles concurrent requests:

```python
import asyncio
from data_bridge.kv import KvPool, PoolConfig

async def worker(pool: KvPool, worker_id: int):
    for i in range(100):
        key = f"worker_{worker_id}_key_{i}"
        await pool.set(key, i)
        value = await pool.get(key)
        await pool.delete(key)

async def main():
    config = PoolConfig(
        "127.0.0.1:6380",
        min_size=5,
        max_size=20,
    )
    pool = await KvPool.connect(config)

    # Run 20 concurrent workers
    tasks = [worker(pool, i) for i in range(20)]
    await asyncio.gather(*tasks)

    # Check stats
    stats = await pool.stats()
    print(f"Pool stats: {stats}")

asyncio.run(main())
```

### API Methods

All `KvClient` methods are available on `KvPool`:

- `await pool.get(key) -> Optional[KvValue]`
- `await pool.set(key, value, ttl=None) -> None`
- `await pool.delete(key) -> bool`
- `await pool.exists(key) -> bool`
- `await pool.incr(key, delta=1) -> int`
- `await pool.decr(key, delta=1) -> int`
- `await pool.setnx(key, value, ttl=None) -> bool`
- `await pool.lock(key, owner, ttl=30.0) -> bool`
- `await pool.unlock(key, owner) -> bool`
- `await pool.extend_lock(key, owner, ttl=30.0) -> bool`
- `await pool.ping() -> str`
- `await pool.info() -> str`
- `await pool.stats() -> PoolStats`

### Properties

- `pool.namespace -> Optional[str]` - Get the namespace if configured

## See Also

- [KV Client API](/kb/60-kv-store/02-data-types-and-api.md) - Core client methods
- [KV Server](/kb/60-kv-store/03-kv-server.md) - Server implementation
- [Protocol](/kb/60-kv-store/protocol.md) - Wire protocol details
