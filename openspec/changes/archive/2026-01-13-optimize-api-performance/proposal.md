# Change: Optimize API Performance

## Why
The current `data-bridge-api` performance trails behind the target baseline (Uvicorn + FastAPI + ORJSON), measuring approximately 0.39x-0.92x in benchmarks. To achieve the goal of "Zero Python Byte Handling" and justify the architectural complexity, the Rust layer must provide superior throughput and lower latency. The current implementation performs unnecessary data copying and uses slower JSON parsers for incoming requests.

## What Changes
- **Serialization**: Replace `serde_json` with `sonic-rs` for all request body parsing (already used for responses).
- **Zero-Copy**: Refactor `SerializableRequest` and internal buffers to use `Bytes` (reference-counted) instead of `Vec<u8>`, avoiding data duplication between Hyper and the logic layer.
- **Connection Management**: Tune Hyper and Tokio settings for aggressive keep-alive and buffer management.
- **Runtime**: Optimize the async boundary to ensure better cooperation between Tokio (Rust) and asyncio (Python).
- **Architecture**: Integrate **PyLoop** (Rust-native Python asyncio event loop backed by Tokio) to replace thread-local `asyncio` loops, ensuring strict adherence to Python's single event loop principle while maximizing performance.

## Architecture
### Current Architecture (PyLoop Phase 2)
```
Request → Axum/Hyper → Router → PythonHandler 
                                      ↓
                              PyLoop (Tokio runtime)
                                      ↓
                              poll_coroutine() loop
                                      ↓
                              Python coroutine (no asyncio)
                                      ↓
                              Response conversion
```

**Key Principles**:
- ✅ Single PyLoop event loop (follows Python best practices)
- ✅ Pure Rust async execution (no Python asyncio dependency)
- ✅ Native coroutine polling (no event loop creation overhead)
- ✅ Proper GIL management (acquire only when needed)
- ✅ Zero Python byte handling (all BSON/JSON in Rust)

## Impact
- **Affected Specs**: `api-server`
- **Affected Code**: `crates/data-bridge-api` (Server, Request, Response modules), `crates/data-bridge-pyloop`
- **Performance**: Targeting ≥1.0x throughput vs FastAPI baseline (Parity or better).
