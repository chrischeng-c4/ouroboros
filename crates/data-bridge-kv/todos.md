# data-bridge-kv - Implementation Todos

## Architecture
Client-Server model with TCP protocol:
```
kv-server (Rust binary)  ←→  kv-client (Rust crate)  ←→  PyO3  ←→  Python
```

## MVP Scope
Redis-comparable in-memory KV store with:
- Basic types: String, Int, Float, Decimal, Bytes, List, Map
- Core ops: SET, GET, DELETE, EXISTS
- Atomic: INCR/DECR
- TTL support
- Sharded multi-core engine (256 shards)
- TCP server/client architecture

---

## In Progress
- [ ] Integration testing (server + client + Python)

## Pending

### Testing & Validation
- [ ] End-to-end integration tests
- [ ] Benchmark vs Redis
- [ ] Memory usage profiling
- [ ] Connection pooling (client)

### Future Enhancements
- [ ] CAS (Compare-And-Swap) command
- [ ] MGET/MSET batch operations
- [ ] Connection pooling
- [ ] Authentication (AUTH command)
- [ ] TLS support

## Completed

### Phase 1: Core Engine ✅ (2025-01-05)
- [x] ✅ Create crate structure (data-bridge-kv)
- [x] ✅ Define KvKey, KvValue types
- [x] ✅ Define KvError types
- [x] ✅ Implement `Shard` struct with RwLock<HashMap>
- [x] ✅ Implement `Entry` struct with TTL and version
- [x] ✅ Implement `KvEngine` with shard routing (256 shards)

### Phase 2: Operations ✅ (2025-01-05)
- [x] ✅ SET operation
- [x] ✅ GET operation
- [x] ✅ DELETE operation
- [x] ✅ EXISTS operation
- [x] ✅ INCR/DECR (atomic, type-safe)
- [x] ✅ TTL expiration support
- [x] ✅ cleanup_expired() for manual TTL cleanup
- [x] ✅ 16 unit tests (all passing)

### Phase 3: Server ✅ (2025-01-05)
- [x] ✅ Design binary wire protocol (protocol.md)
- [x] ✅ Create data-bridge-kv-server crate
- [x] ✅ Implement TCP server with Tokio
- [x] ✅ Command processing (GET, SET, DEL, EXISTS, INCR, DECR, PING, INFO)
- [x] ✅ CLI with clap (--bind, --shards, --log-level)
- [x] ✅ Protocol unit tests

### Phase 4: Client ✅ (2025-01-05)
- [x] ✅ Create data-bridge-kv-client crate
- [x] ✅ Implement async TCP client
- [x] ✅ Full API support (connect, ping, get, set, delete, exists, incr, decr, info)
- [x] ✅ Protocol encoding/decoding
- [x] ✅ Integration tests (ignored, require server)

### Phase 5: Python Integration ✅ (2025-01-05)
- [x] ✅ Update PyO3 bindings for KvClient (not KvEngine)
- [x] ✅ Async methods with future_into_py
- [x] ✅ Value conversion (Python ↔ KvValue)
- [x] ✅ Error mapping (ClientError → PyErr)
- [x] ✅ Create python/data_bridge/kv/ wrapper
- [x] ✅ Type annotations and docstrings
- [x] ✅ Feature-gated with graceful degradation

## Crate Summary

| Crate | Type | Purpose |
|-------|------|---------|
| data-bridge-kv | lib | Core KV engine |
| data-bridge-kv-server | bin | TCP server |
| data-bridge-kv-client | lib | TCP client |
| data-bridge (kv feature) | lib | PyO3 bindings |
| python/data_bridge/kv | pkg | Python wrapper |
