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
| 0x07 | CAS | Compare-and-swap (Planned) | `key_len(2) + key + expected + new_value + ttl(8)` |
| 0x08 | PING | Health check | `(empty)` |
| 0x09 | INFO | Server statistics | `(empty)` |
| 0x0A | SETNX | Set if not exists | `key_len(2) + key + ttl(8) + value` |
| 0x0B | LOCK | Acquire lock | `key_len(2) + key + owner_len(2) + owner + ttl(8)` |
| 0x0C | UNLOCK | Release lock | `key_len(2) + key + owner_len(2) + owner` |
| 0x0D | EXTEND | Extend lock TTL | `key_len(2) + key + owner_len(2) + owner + ttl(8)` |
| 0x0E | MGET | Get multiple keys | `count(2) + [key_len(2) + key]...` |
| 0x0F | MSET | Set multiple pairs | `count(2) + ttl(8) + [key_len(2) + key + value]...` |
| 0x10 | MDEL | Delete multiple keys | `count(2) + [key_len(2) + key]...` |

**Reserved**: 0x11-0xFF for future commands

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
| Bool | 0x08 | `1 byte` | Reserved / Not implemented |

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

### Example 3: LOCK "resource" (owner="worker1", ttl=30s)

**Request:**
```
0x0B                    # LOCK command
0x00 0x00 0x00 0x1F     # payload length = 31 bytes
0x00 0x08               # key length = 8
0x72 0x65 0x73 0x6F 0x75 0x72 0x63 0x65  # "resource"
0x00 0x07               # owner length = 7
0x77 0x6F 0x72 0x6B 0x65 0x72 0x31       # "worker1"
0x00 0x00 0x00 0x00 0x00 0x00 0x75 0x30  # TTL = 30000ms (30s)
```

**Response (success):**
```
0x00                    # OK
0x00 0x00 0x00 0x01     # payload length = 1
0x01                    # bool = 1 (true)
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