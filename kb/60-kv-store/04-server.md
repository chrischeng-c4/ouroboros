# KV Store Server Implementation

**Status**: Completed
**Crate**: `data-bridge-kv-server`
**Type**: Binary (TCP server)
**Binary**: `target/release/kv-server` (1.0MB)

## Overview

The KV server is a high-performance TCP server that exposes the data-bridge KV engine over a binary protocol. It provides a Redis-like interface with support for rich value types including decimals, lists, and maps.

## Architecture

```
┌─────────────────────────────────────┐
│         TCP Listener (main)         │
│     Tokio async, port 6380          │
└────────────┬────────────────────────┘
             │
             ├─► Connection Handler (task 1)
             ├─► Connection Handler (task 2)
             └─► Connection Handler (task N)
                        │
                        ▼
                 ┌─────────────┐
                 │  KvEngine   │
                 │  (sharded)  │
                 └─────────────┘
```

## Components

### 1. Main Entry Point (`src/main.rs`)

- CLI argument parsing with `clap`
- Logging setup with `tracing`
- Server initialization and startup

**CLI Options**:
- `-b, --bind`: Bind address (default: `127.0.0.1:6380`)
- `-s, --shards`: Number of shards (default: `256`)
- `-l, --log-level`: Log level (default: `info`)

### 2. Wire Protocol (`src/protocol.rs`)

Binary protocol for efficient communication:

**Request Format**:
```
[Command (1 byte)][Payload Length (4 bytes)][Payload (variable)]
```

**Response Format**:
```
[Status (1 byte)][Payload Length (4 bytes)][Payload (variable)]
```

**Commands**:
- `0x01` GET - Retrieve value by key
- `0x02` SET - Store value with optional TTL
- `0x03` DEL - Delete key
- `0x04` EXISTS - Check key existence
- `0x05` INCR - Increment integer
- `0x06` DECR - Decrement integer
- `0x07` CAS - Compare-and-swap (Planned)
- `0x08` PING - Health check
- `0x09` INFO - Server stats
- `0x0A` SETNX - Set if not exists
- `0x0B` LOCK - Acquire distributed lock
- `0x0C` UNLOCK - Release distributed lock
- `0x0D` EXTEND - Extend lock TTL

**Status Codes**:
- `0x00` OK - Success
- `0x01` NULL - Key not found
- `0x02` ERROR - Error occurred
- `0x03` INVALID - Invalid command/payload

**Value Types**:
- `0x00` Null
- `0x01` Int (i64)
- `0x02` Float (f64)
- `0x03` Decimal (string representation)
- `0x04` String (UTF-8)
- `0x05` Bytes (raw)
- `0x06` List (recursive)
- `0x07` Map (recursive)

### 3. Server Implementation (`src/server.rs`)

**Features**:
- One async task per connection
- TCP_NODELAY enabled (disable Nagle's algorithm)
- 64KB buffer per connection
- Shared `Arc<KvEngine>` across all connections
- Automatic connection cleanup

**Request Processing**:
1. Read 5-byte header
2. Parse command and payload length
3. Read payload
4. Execute command on KvEngine
5. Encode response
6. Send response

## Performance Optimizations

### 1. Low Latency
- **TCP_NODELAY**: Disabled Nagle's algorithm
- **Zero-copy**: Minimal allocations in hot path
- **Buffer reuse**: 64KB buffer per connection

### 2. Concurrency
- **Async I/O**: Tokio runtime for efficient multiplexing
- **Sharded engine**: Lock-free reads across shards
- **Shared engine**: Single `Arc<KvEngine>` for all connections

### 3. Binary Protocol
- **Compact encoding**: Type-length-value format
- **Big-endian**: Network byte order
- **Max payload**: 64MB limit for safety

## Testing

### Unit Tests (3 tests)
- `test_encode_decode_int`: Int serialization
- `test_encode_decode_string`: String serialization
- `test_encode_decode_list`: List serialization

### Integration Tests
- `test_protocol_encoding`: Protocol correctness
- `test_server_ping`: End-to-end PING (requires running server)

**Run tests**:
```bash
cargo test -p data-bridge-kv-server
```

**Run integration tests** (requires server):
```bash
# Terminal 1
./target/release/kv-server

# Terminal 2
cargo test -p data-bridge-kv-server -- --ignored
```

## Usage Examples

### Start Server

```bash
# Default configuration
./target/release/kv-server

# Bind to all interfaces
./target/release/kv-server --bind 0.0.0.0:6380

# More shards for high concurrency
./target/release/kv-server --shards 512

# Debug logging
./target/release/kv-server --log-level debug
```

### Example Protocol Interaction

**PING Command**:
```
→ [0x08][0x00][0x00][0x00][0x00]
← [0x00][0x00][0x00][0x00][0x04]PONG
```

**GET Command** (key="user:1"):
```
→ [0x01][0x00][0x00][0x00][0x06]user:1
← [0x00][payload_len][encoded_value]  (if found)
← [0x01][0x00][0x00][0x00][0x00]       (if not found)
```

**SET Command** (key="count", value=Int(42), ttl=60s):
```
→ [0x02][payload_len][key_len][key][ttl_ms][value_type][value]
← [0x00][0x00][0x00][0x00][0x00]
```

## Build Information

**Dependencies**:
- `tokio` - Async runtime
- `thiserror` - Error handling
- `clap` - CLI parsing
- `tracing` - Logging
- `rust_decimal` - Decimal type support
- `data-bridge-kv` - KV engine

**Binary Size**: 1.0MB (release, stripped)

**Compilation**:
```bash
# Debug
cargo build -p data-bridge-kv-server

# Release
cargo build -p data-bridge-kv-server --release
```

## Limitations & Future Work

### Current Limitations
1. **No Persistence**: In-memory only
2. **No Clustering**: Single-node
3. **No Authentication**: Open access
4. **CAS Not Implemented**: Planned feature

### Future Enhancements
1. **Persistence Layer**
   - Snapshot to disk
   - WAL (write-ahead log)
   - Background compaction

2. **Clustering**
   - Leader election
   - Replication
   - Sharding across nodes

3. **Security**
   - TLS support
   - Authentication
   - ACLs (access control lists)

4. **Advanced Features**
   - Pub/Sub
   - Transactions
   - Lua scripting
   - Sorted sets

5. **Observability**
   - Prometheus metrics
   - Distributed tracing
   - Health checks

## File Structure

```
crates/data-bridge-kv-server/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs          # CLI entry point
│   ├── protocol.rs      # Wire protocol encoding/decoding
│   └── server.rs        # TCP server implementation
└── tests/
    └── integration_test.rs  # Integration tests
```

## Integration with Python

While this is a standalone binary, it can be used as a backend for:
- Caching layer in data-bridge applications
- Session storage for web applications
- Message queue for task distribution
- Temporary data storage during complex operations

Future work may include a Python client library (`data-bridge-kv-client`) for seamless integration.

## Comparison with Redis

| Feature | data-bridge-kv-server | Redis |
|---------|----------------------|-------|
| Data Types | Int, Float, Decimal, String, Bytes, List, Map | String, List, Set, Sorted Set, Hash |
| Persistence | None (planned) | RDB, AOF |
| Clustering | None (planned) | Cluster mode |
| Protocol | Custom binary | RESP |
| Language | Rust | C |
| Decimal Support | ✅ Native | ❌ String-based |
| Nested Structures | ✅ Yes | ❌ Limited |

## Performance Expectations

Based on the KV engine benchmarks:

- **Single-threaded writes**: ~800K ops/sec
- **Multi-threaded writes (8 threads)**: ~3.7M ops/sec
- **Single-threaded reads**: ~1.2M ops/sec
- **Multi-threaded reads (8 threads)**: ~7.5M ops/sec

Network overhead will reduce these numbers, but the server should still achieve:
- **Reads**: 100K-500K ops/sec (depending on value size)
- **Writes**: 50K-300K ops/sec (depending on value size)

Actual performance will vary based on:
- Network latency
- Value sizes
- Number of concurrent connections
- Number of shards

## Conclusion

The KV server provides a production-ready TCP interface to the data-bridge KV engine with:
- Clean binary protocol
- High performance
- Rich type support
- Extensible architecture

It serves as a foundation for building distributed caching and storage systems within the data-bridge ecosystem.