# data-bridge-kv-server

High-performance TCP server for the data-bridge KV store.

## Features

- **Binary Protocol**: Efficient wire protocol for KV operations
- **Multi-threaded**: One task per connection with Tokio async runtime
- **Low Latency**: TCP_NODELAY enabled by default
- **Sharded Engine**: Configurable number of shards for parallelism
- **Rich Value Types**: Int, Float, Decimal, String, Bytes, List, Map

## Installation

Build the server binary:

```bash
cargo build --release -p data-bridge-kv-server
```

The binary will be at `target/release/kv-server`.

## Usage

### Start Server

```bash
# Default: 127.0.0.1:6380, 256 shards
./target/release/kv-server

# Custom bind address
./target/release/kv-server --bind 0.0.0.0:6380

# More shards for better parallelism
./target/release/kv-server --shards 512

# Enable debug logging
./target/release/kv-server --log-level debug
```

### Command Line Options

```
-b, --bind <BIND>            Address to bind to [default: 127.0.0.1:6380]
-s, --shards <SHARDS>        Number of shards for the KV engine [default: 256]
-l, --log-level <LOG_LEVEL>  Log level (trace, debug, info, warn, error) [default: info]
-h, --help                   Print help
```

## Wire Protocol

### Request Format

```
[Command (1 byte)][Payload Length (4 bytes)][Payload (variable)]
```

### Response Format

```
[Status (1 byte)][Payload Length (4 bytes)][Payload (variable)]
```

### Commands

| Code | Command | Description |
|------|---------|-------------|
| 0x01 | GET     | Get value by key |
| 0x02 | SET     | Set key-value with optional TTL |
| 0x03 | DEL     | Delete key |
| 0x04 | EXISTS  | Check if key exists |
| 0x05 | INCR    | Increment integer value |
| 0x06 | DECR    | Decrement integer value |
| 0x07 | CAS     | Compare-and-swap (not yet implemented) |
| 0x08 | PING    | Health check |
| 0x09 | INFO    | Server information |

### Status Codes

| Code | Status | Description |
|------|--------|-------------|
| 0x00 | OK     | Success |
| 0x01 | NULL   | Key not found |
| 0x02 | ERROR  | Error occurred |

### Value Types

| Code | Type    | Encoding |
|------|---------|----------|
| 0x00 | Null    | None |
| 0x01 | Int     | i64 (8 bytes, big-endian) |
| 0x02 | Float   | f64 (8 bytes, big-endian) |
| 0x03 | Decimal | String representation |
| 0x04 | String  | UTF-8 bytes |
| 0x05 | Bytes   | Raw bytes |
| 0x06 | List    | Array of values |
| 0x07 | Map     | Key-value pairs |

## Examples

### PING Command

Request:
```
[0x08][0x00][0x00][0x00][0x00]
```

Response:
```
[0x00][0x00][0x00][0x00][0x04]PONG
```

### GET Command

Request (key="user:1"):
```
[0x01][0x00][0x00][0x00][0x06]user:1
```

Response (value found):
```
[0x00][payload_len][encoded_value]
```

Response (key not found):
```
[0x01][0x00][0x00][0x00][0x00]
```

### SET Command

Payload format: `key_len(2) + key + ttl_ms(8) + value`

Example (key="user:1", ttl=60000ms, value=Int(42)):
```
[0x02][payload_len][0x00][0x06]user:1[ttl_bytes][value_type][value_bytes]
```

## Testing

Run unit tests:
```bash
cargo test -p data-bridge-kv-server
```

Run integration tests (requires server running):
```bash
# Terminal 1: Start server
./target/release/kv-server

# Terminal 2: Run tests
cargo test -p data-bridge-kv-server -- --ignored
```

## Performance

- **Zero-copy**: Minimal allocations in request/response handling
- **Sharded**: Lock-free reads across shards
- **Async I/O**: Tokio runtime for efficient connection handling
- **TCP_NODELAY**: Disabled Nagle's algorithm for low latency

## Architecture

```
┌─────────────────────────────────────┐
│         TCP Listener (main)         │
└────────────┬────────────────────────┘
             │
             ├─► Connection Handler (tokio::spawn)
             ├─► Connection Handler (tokio::spawn)
             └─► Connection Handler (tokio::spawn)
                        │
                        ▼
                 ┌─────────────┐
                 │  KvEngine   │
                 │  (sharded)  │
                 └─────────────┘
```

Each connection runs in its own async task, sharing a single `Arc<KvEngine>` instance.

## Limitations

- **No Persistence**: In-memory only (TODO: add persistence layer)
- **No Clustering**: Single-node only (TODO: add replication)
- **No Authentication**: Open access (TODO: add auth)
- **CAS Not Implemented**: Compare-and-swap planned for future release

## License

MIT
