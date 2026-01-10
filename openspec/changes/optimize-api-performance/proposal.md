# Change: Optimize API Performance

## Why
The current `data-bridge-api` performance trails behind the target baseline (Uvicorn + FastAPI + ORJSON), measuring approximately 0.39x-0.92x in benchmarks. To achieve the goal of "Zero Python Byte Handling" and justify the architectural complexity, the Rust layer must provide superior throughput and lower latency. The current implementation performs unnecessary data copying and uses slower JSON parsers for incoming requests.

## What Changes
- **Serialization**: Replace `serde_json` with `sonic-rs` for all request body parsing (already used for responses).
- **Zero-Copy**: Refactor `SerializableRequest` and internal buffers to use `Bytes` (reference-counted) instead of `Vec<u8>`, avoiding data duplication between Hyper and the logic layer.
- **Connection Management**: Tune Hyper and Tokio settings for aggressive keep-alive and buffer management.
- **Runtime**: Optimize the async boundary to ensure better cooperation between Tokio (Rust) and asyncio (Python).

## Impact
- **Affected Specs**: `api-server`
- **Affected Code**: `crates/data-bridge-api` (Server, Request, Response modules)
- **Performance**: Targeting >1.5x throughput vs FastAPI baseline.
