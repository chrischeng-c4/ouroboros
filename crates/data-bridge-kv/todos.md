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
- [ ] Distributed cache features (MGET/MSET, CAS)

## Pending

### Future Enhancements
- [ ] CAS (Compare-And-Swap) command
- [ ] MGET/MSET batch operations
- [ ] Authentication (AUTH command)
- [ ] TLS support
- [ ] Memory usage profiling
- [ ] Cluster mode (multi-node)

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

### Phase 6: Lock API ✅ (2025-01-05)
- [x] ✅ Implement SETNX command (SET if Not Exists)
- [x] ✅ Add Lock API with auto-release TTL
- [x] ✅ Python context manager: `async with client.lock("key", ttl=30)`
- [x] ✅ Lock renewal (extend TTL while holding)

### Phase 7: Namespace Support ✅ (2025-01-05)
- [x] ✅ Parse namespace from connection string (host:port/namespace)
- [x] ✅ Key prefixing for all operations
- [x] ✅ Namespace isolation (keys, locks)
- [x] ✅ Python namespace property
- [x] ✅ 19 integration tests

### Phase 8: Connection Pooling ✅ (2025-01-05)
- [x] ✅ PoolConfig (min/max size, timeouts)
- [x] ✅ KvPool with RAII PooledClient
- [x] ✅ Idle connection cleanup
- [x] ✅ Pool statistics (idle, active, max)
- [x] ✅ PyO3 bindings (_KvPool, _PoolConfig)
- [x] ✅ Python wrapper (KvPool, PoolConfig)

### Phase 9: Testing Suite ✅ (2025-01-05)
- [x] ✅ Performance benchmarks (13 tests)
- [x] ✅ Security tests (30 tests)
- [x] ✅ Redis comparison (1.4-1.7x faster)
- [x] ✅ Latency metrics (P50/P95/P99)

## Crate Summary

| Crate | Type | Purpose |
|-------|------|---------|
| data-bridge-kv | lib | Core KV engine |
| data-bridge-kv-server | bin | TCP server |
| data-bridge-kv-client | lib | TCP client |
| data-bridge (kv feature) | lib | PyO3 bindings |
| python/data_bridge/kv | pkg | Python wrapper |
