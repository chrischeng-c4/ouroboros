# KV Store Wire Protocol

## Overview
Simple binary protocol for high-performance KV operations, inspired by Redis RESP but optimized for data-bridge's use case.

## Design Goals

1. **Low Parsing Overhead**: Fixed-size headers, minimal branching
2. **Type Safety**: Explicit type tags for all values
3. **Zero-Copy Potential**: Length-prefixed payloads enable direct buffer operations
4. **Pipelined Operations**: Independent request/response pairs
5. **Future-Proof**: Reserved command codes for extensions

## Message Format

### Request Format
```
[1 byte: command] [4 bytes: payload length (big-endian)] [payload]
```

- **Command**: Single byte operation code (0x01-0xFF)
- **Payload Length**: u32 big-endian, excludes header (5 bytes)
- **Payload**: Command-specific binary data

### Response Format
```
[1 byte: status] [4 bytes: payload length (big-endian)] [payload]
```

- **Status**: Single byte result code (0x00-0xFF)
- **Payload Length**: u32 big-endian, excludes header (5 bytes)
- **Payload**: Response-specific binary data

## Command Specification

### Commands

| Code | Name | Description | Payload Format |
|------|------|-------------|----------------|
| 0x01 | GET | Retrieve value by key | `key (UTF-8)` |
| 0x02 | SET | Store key-value pair | `key_len(2) + key + ttl(8) + value` |
| 0x03 | DEL | Delete key | `key (UTF-8)` |
| 0x04 | EXISTS | Check key existence | `key (UTF-8)` |
| 0x05 | INCR | Atomic increment | `key_len(2) + key + delta(8)` |
| 0x06 | DECR | Atomic decrement | `key_len(2) + key + delta(8)` |
| 0x07 | CAS | Compare-and-swap | `key_len(2) + key + expected + new_value + ttl(8)` |
| 0x08 | PING | Health check | `(empty)` |
| 0x09 | INFO | Server statistics | `(empty)` |
| 0x0A | MGET | Multi-get | `count(4) + keys...` |
| 0x0B | MSET | Multi-set | `count(4) + (key_len(2) + key + ttl(8) + value)...` |
| 0x0C | EXPIRE | Set TTL on existing key | `key_len(2) + key + ttl(8)` |
| 0x0D | TTL | Get remaining TTL | `key (UTF-8)` |

**Reserved**: 0x0E-0xFF for future commands

### Status Codes

| Code | Name | Meaning | Payload |
|------|------|---------|---------|
| 0x00 | OK | Success | Command-specific |
| 0x01 | NULL | Key not found | Empty |
| 0x02 | ERROR | Operation failed | Error message (UTF-8) |
| 0x03 | INVALID | Invalid command/payload | Error message (UTF-8) |
| 0x04 | CAS_FAILED | Compare-and-swap mismatch | Empty |

## Value Encoding

All values use tagged encoding:

```
[1 byte: type] [type-specific data]
```

### Type Codes

| Type | Code | Data Format | Description |
|------|------|-------------|-------------|
| Null | 0x00 | `(none)` | Null/None value |
| Int | 0x01 | `8 bytes (i64 big-endian)` | Signed 64-bit integer |
| Float | 0x02 | `8 bytes (f64 big-endian)` | IEEE 754 double |
| Decimal | 0x03 | `len(2) + string` | High-precision decimal (string repr) |
| String | 0x04 | `len(4) + UTF-8 bytes` | UTF-8 encoded string |
| Bytes | 0x05 | `len(4) + raw bytes` | Arbitrary binary data |
| List | 0x06 | `count(4) + values...` | Ordered sequence of values |
| Map | 0x07 | `count(4) + (key_len(2) + key + value)...` | Key-value pairs |
| Bool | 0x08 | `1 byte (0 or 1)` | Boolean value |

**Reserved**: 0x09-0xFF for future types (e.g., timestamps, UUIDs)

## Wire Examples

### Example 1: SET "foo" = 42 (with no TTL)

**Request:**
```
0x02                    # SET command
0x00 0x00 0x00 0x0F     # payload length = 15 bytes
0x00 0x03               # key length = 3
0x66 0x6F 0x6F          # "foo" (UTF-8)
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00  # TTL = 0 (no expiration)
0x01                    # type = Int
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x2A  # value = 42 (i64 big-endian)
```

**Response:**
```
0x00                    # OK
0x00 0x00 0x00 0x00     # payload length = 0
```

### Example 2: GET "foo"

**Request:**
```
0x01                    # GET command
0x00 0x00 0x00 0x03     # payload length = 3
0x66 0x6F 0x6F          # "foo"
```

**Response (key exists):**
```
0x00                    # OK
0x00 0x00 0x00 0x09     # payload length = 9
0x01                    # type = Int
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x2A  # value = 42
```

**Response (key not found):**
```
0x01                    # NULL
0x00 0x00 0x00 0x00     # payload length = 0
```

### Example 3: SET with TTL (60 seconds)

**Request:**
```
0x02                    # SET command
0x00 0x00 0x00 0x13     # payload length = 19
0x00 0x03               # key length = 3
0x74 0x6D 0x70          # "tmp"
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x3C  # TTL = 60 seconds
0x04                    # type = String
0x00 0x00 0x00 0x05     # string length = 5
0x68 0x65 0x6C 0x6C 0x6F # "hello"
```

### Example 4: CAS (Compare-And-Swap)

**Request:**
```
0x07                    # CAS command
0x00 0x00 0x00 0x1A     # payload length = 26
0x00 0x03               # key length = 3
0x66 0x6F 0x6F          # "foo"
0x01                    # expected type = Int
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x2A  # expected value = 42
0x01                    # new type = Int
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x64  # new value = 100
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00  # TTL = 0
```

**Response (success):**
```
0x00                    # OK
0x00 0x00 0x00 0x01     # payload length = 1
0x01                    # bool = true
```

**Response (CAS failed):**
```
0x04                    # CAS_FAILED
0x00 0x00 0x00 0x00     # payload length = 0
```

### Example 5: MGET (Multi-Get)

**Request:**
```
0x0A                    # MGET command
0x00 0x00 0x00 0x0E     # payload length = 14
0x00 0x00 0x00 0x02     # count = 2 keys
0x00 0x03               # key1 length = 3
0x66 0x6F 0x6F          # "foo"
0x00 0x03               # key2 length = 3
0x62 0x61 0x72          # "bar"
```

**Response:**
```
0x00                    # OK
0x00 0x00 0x00 0x16     # payload length = 22
0x00 0x00 0x00 0x02     # count = 2 results
0x01                    # result1 type = Int
0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x2A  # value = 42
0x00                    # result2 type = Null (not found)
```

## Connection Management

### Network Configuration
- **Default Port**: 6380 (configurable)
- **Protocol**: TCP with persistent connections
- **Max Payload Size**: 64MB (configurable, prevents DoS)
- **Timeout**: 30s idle connection timeout (configurable)

### Connection Lifecycle
1. **Connect**: TCP handshake
2. **Send Request**: Write command + payload
3. **Receive Response**: Read status + payload
4. **Pipelining**: Multiple requests without waiting for responses
5. **Close**: TCP FIN or timeout

### Error Handling
- **Invalid Command**: Status 0x03 (INVALID) + error message
- **Payload Too Large**: Status 0x02 (ERROR) + "Payload exceeds limit"
- **Connection Closed**: TCP RST or FIN
- **Timeout**: Server closes idle connections after 30s

## Future Extensions

### Authentication (v2)
```
0x10 | AUTH | username_len(2) + username + password_len(2) + password
```

### Transactions (v2)
```
0x20 | MULTI  | Begin transaction
0x21 | EXEC   | Commit transaction
0x22 | DISCARD| Rollback transaction
```

### Pub/Sub (v3)
```
0x30 | SUBSCRIBE   | channel_count(4) + channels...
0x31 | UNSUBSCRIBE | channel_count(4) + channels...
0x32 | PUBLISH     | channel_len(2) + channel + message
```

## Performance Characteristics

### Throughput
- **Fixed Header**: 5 bytes (predictable read)
- **Single Syscall**: Read header, then payload (total: 2 reads)
- **Zero-Copy**: Length-prefixed payloads enable vectored I/O
- **Pipelining**: Multiple requests in flight (Tokio async I/O)

### Latency
- **Parsing**: O(1) command dispatch, O(n) payload decode
- **Encoding**: Pre-allocated buffers, single write
- **Network**: Nagle's algorithm disabled (TCP_NODELAY)

### Security
- **Max Payload**: Prevents memory exhaustion
- **Type Validation**: Server validates all type codes
- **Key Length**: Max 256 bytes (u16 length prefix)
- **No Injection**: Binary protocol, no string parsing

## Implementation Notes

### Rust Server (Tokio)
```rust
// Pseudo-code
async fn handle_request(socket: TcpStream) -> Result<()> {
    let mut buf = [0u8; 5];
    socket.read_exact(&mut buf).await?;  // Read header

    let cmd = buf[0];
    let len = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]);

    let mut payload = vec![0u8; len as usize];
    socket.read_exact(&mut payload).await?;  // Read payload

    let response = match cmd {
        0x01 => handle_get(&payload)?,
        0x02 => handle_set(&payload)?,
        // ...
    };

    socket.write_all(&response).await?;
    Ok(())
}
```

### Python Client (PyO3)
```python
# Pseudo-code
class KVClient:
    def get(self, key: str) -> Optional[Any]:
        # Rust FFI encodes request, sends via socket, decodes response
        return _engine.kv_get(self._handle, key)

    def set(self, key: str, value: Any, ttl: Optional[int] = None):
        return _engine.kv_set(self._handle, key, value, ttl or 0)
```

## Compatibility

- **Endianness**: Big-endian (network byte order)
- **UTF-8**: All string keys and values
- **Versioning**: Future protocol versions use different ports (6381, 6382, etc.)

## References

- [Redis RESP3](https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md)
- [Memcached Binary Protocol](https://github.com/memcached/memcached/wiki/BinaryProtocolRevamped)
- [gRPC Wire Format](https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-HTTP2.md)
