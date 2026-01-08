---
title: Cloud Native Simple KV Store
status: active
component: kv-store
type: index
---

# Cloud Native Simple KV Store

The KV Store solution is a component of `data-bridge` providing a high-performance, cloud-native key-value store backed by a custom Rust engine. It supports advanced data types beyond standard Redis implementations and is optimized for both caching and task state management.

## Overview

A simple yet powerful distributed key-value store designed for cloud-native environments.

-   **Status**: Active / Alpha
-   **Goal**: High-performance, low-latency storage with strict type safety.
-   **Key Feature**: Native support for high-precision numeric types (Decimal) and atomic state transitions.

## Target Use Cases

### 1. High-Performance Cache
Designed to outperform standard Redis setups for Python applications dealing with typed data.

-   **Financial & Scientific Caching**: Unlike Redis (which stores numbers as strings), this store natively preserves `Decimal`, `f64`, and `i64` precision. This eliminates parsing overhead and precision loss risks.
-   **Zero-Serialization Overhead**: Python objects map directly to Rust types. No intermediate JSON or Pickle step is required for supported types.
-   **Smart Eviction**: Standard TTL (Time-To-Live) and LRU (Least Recently Used) policies implemented efficiently in Rust.

### 2. Task Store (Result Backend)
Optimized to serve as a backend for distributed task queues (similar to Celery or Dramatiq result backends), but faster and more type-aware.

-   **Distributed Locking**: Built-in support for `lock` / `unlock` with auto-expiration to manage task exclusivity.
-   **Structured Results**: Store task results (including complex numerics) directly without serialization penalties.
-   **Crash-Safe Durability**: ✅ WAL (Write-Ahead Log) persistence ensures task results survive pod restarts with ~100ms data loss window.

## Data Specifications

### Keys
-   **Type**: UTF-8 String
-   **Constraint**: Maximum 256 characters
-   **Validation**: Enforced at the Rust boundary before any storage operation.

### Values
Supports a superset of Redis data types, plus high-precision numerics:

#### Primitive Types
-   **Integer**: 64-bit signed integer (`i64`)
-   **Float**: 64-bit floating point (`f64`)
-   **Decimal**: 128-bit fixed-point decimal (suitable for financial data)

#### Collection Types (Redis-compatible)
-   **String**: Binary safe string / bytes
-   **List**: Linked list of values
-   **Map**: Key-Value map

## Architecture

The system employs a **Sharded In-Memory Architecture** with optional **WAL Persistence** for crash-safe durability. A **Hybrid Tiered Storage Architecture** (RAM + Disk) is planned for future enhancements.

1.  **Multi-Core Engine**: Unlike Redis (single-threaded), this store utilizes a sharded internal architecture (256 shards with `RwLock<HashMap>`) to fully leverage multi-core CPUs for concurrent reads/writes.
2.  **WAL Persistence**: ✅ **Implemented (2026-01-06)**
    -   **Write-Ahead Log**: All write operations logged before acknowledgment
    -   **Batched Fsync**: 100ms batched fsync for optimal throughput (industry best practice)
    -   **Periodic Snapshots**: Full state snapshots every 5 minutes or 100K operations
    -   **Fast Recovery**: Load snapshot + replay WAL delta on startup
    -   **Non-Blocking Writes**: Background persistence thread with 10K operation buffer
    -   **Data Loss Window**: ~100ms (acceptable for task backends)
3.  **Tiered Storage (RAM + Disk)**: [Planned]
    -   **Hot Data**: Resides in Memory for microsecond latency.
    -   **Warm/Cold Data**: When memory limit is reached, data is evicted to Disk (SSD/NVMe) but remains addressable.
    -   **Transparent Retrieval**: A request for cold data automatically fetches it from disk back into memory.

## Deployment Model

-   **Standalone**: Runs as a single TCP server binary (`kv-server`).
-   **Kubernetes StatefulSet**: Designed to run as a stateful pod (Planned).

## Key Features

### Performance
-   **Multi-Core Utilization**: Parallel processing of requests via Rust's async runtime (`tokio`) and partitioned state.
-   **Zero Python Byte Handling**: Storage and retrieval logic entirely in Rust.
-   **GIL Release**: Released during all I/O and serialization operations.

### API Features
-   **Connection Pooling**: Built-in thread-safe pooling with auto-reconnection.
-   **Namespaces**: Isolation of keys via connection strings (e.g., `localhost:6380/my_app`).
-   **Distributed Locking**: `setnx`, `lock`, `unlock`, `extend` commands for coordination.

## Technology Stack

-   **Core Engine**: Pure Rust
-   **Async Runtime**: `tokio` (shared with other components)
-   **Serialization**: Custom binary format or BSON
-   **Network**: Raw TCP with custom binary protocol
-   **Python Binding**: `PyO3`

## Source Code Locations

### Rust Crates
-   **Core Engine**: `crates/data-bridge-kv/`
-   **Server**: `crates/data-bridge-kv-server/`
-   **Client**: `crates/data-bridge-kv-client/`
-   **PyO3 Bindings**: `crates/data-bridge/src/kv.rs`

### Python Package
-   **API Layer**: `python/data_bridge/kv/`

## Documentation

-   **[Architecture Design](./01-architecture.md)**: Sharding, Tiered Storage (Planned), and Persistence details.
-   **[Wire Protocol](./02-protocol.md)**: Binary wire protocol specification.
-   **[Data Types & API](./03-api.md)**: Supported types and Python/Rust API references.
-   **[KV Server](./04-server.md)**: TCP server implementation with async request handling.
-   **[Client & Pooling](./05-client.md)**: Thread-safe connection pooling for high-concurrency scenarios.
-   **[User Guide](./06-guide.md)**: Guide to building and running the KV server.
-   **[Roadmap & Todos](./09-roadmap.md)**: Planned tasks and improvements.
-   **[Batch Operations](./10-batch-operations-implementation.md)**: ✅ MGET/MSET/MDEL implementation (2026-01-06).
-   **[WAL Persistence](./11-wal-persistence-implementation.md)**: ✅ Write-Ahead Log with crash-safe durability (2026-01-06).
-   **[Benchmarks](./benchmarks/01-rust-bench-suite.md)**: Performance benchmarks and reports.
