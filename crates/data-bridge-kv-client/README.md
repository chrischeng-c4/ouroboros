# data-bridge-kv-client

High-performance async TCP client for the data-bridge KV store server.

## Features

- Async TCP communication with the KV server
- Support for all KV operations (GET, SET, DELETE, INCR, DECR, etc.)
- Type-safe protocol handling
- Optional TTL support for keys
- Connection pooling ready

## Usage

```rust
use data_bridge_kv_client::{KvClient, KvValue};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to KV server
    let mut client = KvClient::connect("127.0.0.1:6380").await?;

    // Ping server
    let pong = client.ping().await?;
    println!("Server response: {}", pong);

    // Set a value
    client.set("user:1:name", KvValue::String("Alice".to_string()), None).await?;

    // Set with TTL (expires in 60 seconds)
    client.set(
        "session:abc123",
        KvValue::String("active".to_string()),
        Some(Duration::from_secs(60))
    ).await?;

    // Get a value
    if let Some(value) = client.get("user:1:name").await? {
        println!("Name: {:?}", value);
    }

    // Increment a counter
    client.set("counter", KvValue::Int(0), None).await?;
    let new_value = client.incr("counter", 5).await?;
    println!("Counter: {}", new_value);

    // Check if key exists
    let exists = client.exists("user:1:name").await?;
    println!("Key exists: {}", exists);

    // Delete a key
    let deleted = client.delete("user:1:name").await?;
    println!("Deleted: {}", deleted);

    Ok(())
}
```

## Running Integration Tests

Integration tests require a running KV server:

```bash
# Terminal 1: Start the server
cargo run -p data-bridge-kv-server

# Terminal 2: Run integration tests
cargo test -p data-bridge-kv-client -- --ignored
```

## API

### Connection

- `KvClient::connect(addr: &str)` - Connect to server

### Basic Operations

- `ping()` - Ping server
- `get(key)` - Get value by key
- `set(key, value, ttl)` - Set value with optional TTL
- `delete(key)` - Delete key
- `exists(key)` - Check if key exists

### Numeric Operations

- `incr(key, delta)` - Increment integer value
- `decr(key, delta)` - Decrement integer value

### Info

- `info()` - Get server information

## Protocol

The client uses a binary protocol over TCP:

**Request Format:**
```
[Command:1][PayloadLen:4][Payload:N]
```

**Response Format:**
```
[Status:1][PayloadLen:4][Payload:N]
```

See `protocol.rs` for complete protocol specification.
