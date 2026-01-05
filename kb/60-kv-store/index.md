---
title: Cloud Native Simple KV Store
status: planning
component: kv-store
type: index
---

# Cloud Native Simple KV Store

The KV Store solution is a planned component of `data-bridge`. It will provide a high-performance, cloud-native key-value store backed by a custom Rust engine, supporting advanced data types beyond standard Redis implementations.

## Overview

A simple yet powerful distributed key-value store designed for cloud-native environments, specifically optimized for **high-performance caching** and **task state management**.

- **Status**: Planning
- **Goal**: High-performance, low-latency storage with strict type safety.
- **Key Feature**: Native support for high-precision numeric types (Decimal) and atomic state transitions.

## Target Use Cases

### 1. High-Performance Cache
Designed to outperform standard Redis setups for Python applications dealing with typed data.

- **Financial & Scientific Caching**: Unlike Redis (which stores numbers as strings), this store natively preserves `Decimal`, `f64`, and `i64` precision. This eliminates parsing overhead and precision loss risks.
- **Zero-Serialization Overhead**: Python objects map directly to Rust types. No intermediate JSON or Pickle step is required for supported types.
- **Smart Eviction**: Standard TTL (Time-To-Live) and LRU (Least Recently Used) policies implemented efficiently in Rust.

### 2. Task Store (Result Backend)
Optimized to serve as a backend for distributed task queues (similar to Celery or Dramatiq result backends), but faster and more type-aware.

- **Atomic State Transitions**: Built-in support for compare-and-swap (CAS) operations to safely transition task states (e.g., `PENDING` → `STARTED` → `SUCCESS`).
- **Structured Results**: Store task results (including complex numerics) directly without serialization penalties.
- **Ephemeral vs. Durable**: Configurable durability allows "fire-and-forget" tasks to run in-memory while critical task histories are persisted via WAL.

## Data Specifications

### Keys
- **Type**: UTF-8 String
- **Constraint**: Maximum 256 characters
- **Validation**: Enforced at the Rust boundary before any storage operation.

### Values
Supports a superset of Redis data types, plus high-precision numerics:

#### Primitive Types
- **Integer**: 64-bit signed integer (`i64`)
- **Float**: 64-bit floating point (`f64`)
- **Decimal**: 128-bit fixed-point decimal (suitable for financial data)

#### Collection Types (Redis-compatible)
- **String**: Binary safe string / bytes
- **List**: Linked list of values
- **Set**: Unordered unique collection
- **Hash**: Field-value map
- **Sorted Set**: Unique collection ordered by score

## Architecture

The system employs a **Hybrid Tiered Storage Architecture** optimized for Kubernetes StatefulSets:

1.  **Multi-Core Engine**: Unlike Redis (single-threaded), this store utilizes a sharded internal architecture (using `DashMap` or partitioned locks) to fully leverage multi-core CPUs for concurrent reads/writes.
2.  **Tiered Storage (RAM + Disk)**:
    - **Hot Data**: Resides in Memory for microsecond latency.
    - **Warm/Cold Data**: When memory limit is reached, data is evicted to Disk (SSD/NVMe) but remains addressable.
    - **Transparent Retrieval**: A request for cold data automatically fetches it from disk back into memory.
3.  **Configurable Durability**:
    - **Task Store Mode**: Uses WAL (Write-Ahead Log) to ensure data survives pod restarts.
    - **Cache Mode**: Can run as purely ephemeral or use disk only for overflow (swap), skipping WAL for maximum write throughput.

## Deployment Model

- **Kubernetes StatefulSet**: Designed to run as a stateful pod with attached Persistent Volume (PVC).
- **Vertical Scaling**: Optimized to scale up with more CPU cores and RAM within a single pod before needing horizontal sharding.

## Key Features

### Performance
- **Multi-Core Utilization**: Parallel processing of requests via Rust's async runtime and partitioned state.
- **Zero Python Byte Handling**: Storage and retrieval logic entirely in Rust.
- **GIL Release**: Released during all I/O and serialization operations.

### Storage & Persistence
- **Hybrid Engine**: Supports datasets larger than available RAM.
- **WAL (Write-Ahead Log)**: Optional durability for Task Store usage.
- **Snapshotting**: Periodic dumps for backup/restore.

### Cache & Task Features

## Technology Stack

- **Core Engine**: Pure Rust
- **Async Runtime**: `tokio` (shared with other components)
- **Serialization**: Custom binary format or BSON
- **Network**: `tonic` (gRPC) or raw TCP for cluster communication
- **Python Binding**: `PyO3`

## Source Code Locations (Planned)

### Rust Crates
- **Core Engine**: `crates/data-bridge-kv/`
- **PyO3 Bindings**: `crates/data-bridge/src/kv.rs`

### Python Package
- **API Layer**: `python/data_bridge/kv/`

## Documentation

- **[Architecture Design](./01-architecture.md)**: Sharding, Tiered Storage, and Persistence details.
- **[Data Types & API](./02-data-types-and-api.md)**: Supported types and Python/Rust API references.

## Implementation Roadmap

### Phase 1: Core Storage Engine (Multi-Core)
- Sharded in-memory data structures for concurrent access.
- Type system implementation (Int, Float, Decimal, Redis types).
- Basic CRUD (Set, Get, Delete, CAS).

### Phase 2: Tiered Storage (Disk Spillover)
- Implementation of the Hot/Cold data management.
- Integration with local file system (RocksDB or custom log-structured merge tree).
- Configurable memory limits.

### Phase 3: Persistence (Task Store Focus)
- WAL (Write-Ahead Log) implementation for durability.
- Recovery mechanism on startup.

### Phase 4: Python Integration
- PyO3 bindings.
- Python client API for Cache and Task usage.

### Future / Under Consideration
- **Clustering & Replication**: Horizontal scaling and high availability (Raft/Paxos).
- **Kubernetes Operator**: For automated management.

