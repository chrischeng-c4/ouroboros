# KV Server Quick Start

## Build

```bash
cargo build --release -p data-bridge-kv-server
```

## Run

```bash
# Default (127.0.0.1:6380, 256 shards)
./target/release/kv-server

# Custom configuration
./target/release/kv-server --bind 0.0.0.0:6380 --shards 512 --log-level debug
```

## Test

```bash
# Unit tests
cargo test -p data-bridge-kv-server

# Integration tests (requires running server)
cargo test -p data-bridge-kv-server -- --ignored
```

## Protocol Cheat Sheet

### Commands
| Code | Name   | Payload |
|------|--------|---------|
| 0x01 | GET    | key (UTF-8) |
| 0x02 | SET    | key_len(2) + key + ttl_ms(8) + value |
| 0x03 | DEL    | key (UTF-8) |
| 0x04 | EXISTS | key (UTF-8) |
| 0x05 | INCR   | key_len(2) + key + delta(8) |
| 0x06 | DECR   | key_len(2) + key + delta(8) |
| 0x08 | PING   | (empty) |
| 0x09 | INFO   | (empty) |

### Response Status
| Code | Meaning |
|------|---------|
| 0x00 | OK |
| 0x01 | NULL (key not found) |
| 0x02 | ERROR |

### Value Types
| Code | Type    | Format |
|------|---------|--------|
| 0x01 | Int     | i64 (big-endian) |
| 0x02 | Float   | f64 (big-endian) |
| 0x03 | Decimal | string |
| 0x04 | String  | UTF-8 |
| 0x05 | Bytes   | raw |
| 0x06 | List    | count(4) + values |
| 0x07 | Map     | count(4) + (key_len(2) + key + value)* |

## Example: PING

**Request**:
```
08 00 00 00 00
```

**Response**:
```
00 00 00 00 04 50 4F 4E 47
(OK, 4 bytes, "PONG")
```

## Example: GET

**Request** (key="test"):
```
01 00 00 00 04 74 65 73 74
(GET, 4 bytes, "test")
```

**Response** (not found):
```
01 00 00 00 00
(NULL, 0 bytes)
```

## Python Client Example

```python
import socket
import struct

def send_ping():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('127.0.0.1', 6380))

    # Send PING
    request = struct.pack('!BI', 0x08, 0)
    sock.send(request)

    # Read response
    header = sock.recv(5)
    status, payload_len = struct.unpack('!BI', header)
    payload = sock.recv(payload_len)

    print(f"Status: {status}, Payload: {payload.decode()}")
    # Output: Status: 0, Payload: PONG

    sock.close()
```

## Next Steps

1. See `/Users/chris.cheng/chris-project/data-bridge/crates/data-bridge-kv-server/README.md` for full documentation
2. See `/Users/chris.cheng/chris-project/data-bridge/kb/60-kv-store/03-kv-server.md` for architecture details
3. Build a Python client library for easy integration
