---
title: Cloud Native KV Store Core
status: planning
component: kv-store
type: architecture
---

# Architecture Design

The Cloud Native KV Store utilizes a **Hybrid Tiered Storage Architecture** to combine the speed of in-memory caching with the capacity and durability of disk storage. This design is specifically optimized for Kubernetes StatefulSet deployments.

## 1. Sharded Engine (Multi-Core)

To overcome the "Global Lock" limitation common in single-threaded stores (like Redis), the key space is partitioned into independent **Shards** (default: 256 or 1024).

```rust
struct KvStore {
    // Array of shards, each protected by its own lock
    shards: Vec<RwLock<Shard>>,
    // Consistent hashing state
    hasher: RandomState,
}

struct Shard {
    // Hot Data (In-Memory)
    data: HashMap<Key, Entry>,
    // Cold Data (On-Disk Index)
    disk_index: DiskIndex, 
    // Memory Tracking
    current_memory_usage: usize,
    // LRU/Clock list for eviction
    eviction_list: EvictionPolicy,
}
```

### Key Principles
-   **Routing**: Keys are mapped to shards using `hash(key) % num_shards`.
-   **Concurrency**: Operations on different shards run in parallel. A Write on Shard 1 does not block a Read on Shard 2.
-   **Lock Granularity**: `RwLock` ensures multiple readers can access the same shard simultaneously, while writers gain exclusive access only to that specific shard.

## 2. Hybrid Tiered Storage (RAM + Disk)

The engine acts as a virtual memory manager for KV pairs, allowing the dataset to exceed available RAM.

### Eviction Logic (RAM → Disk)
When a shard exceeds its configured memory limit (`Shard.current_memory_usage > Shard.limit`):
1.  **Identify Cold Entries**: The engine uses an LRU (Least Recently Used) or Clock algorithm to find the least accessed entries in the shard.
2.  **Serialize**: The `Entry` is serialized into a compact binary format.
3.  **Write to Disk**: The blob is appended to the current active `DataFile`.
4.  **Update Index**: The `Shard.disk_index` maps the key to the new location `(file_id, offset, length)`.
5.  **Reclaim RAM**: The entry is removed from the in-memory `HashMap`, freeing up space.

### Retrieval Logic (Disk → RAM)
When a `GET(key)` request occurs:
1.  **Check RAM**: If the key exists in `Shard.data`, return it immediately (Latency: <100µs).
2.  **Check Disk Index**: If not in RAM, check `Shard.disk_index`.
3.  **Fetch from Disk**:
    -   Read the specific bytes from the `DataFile` using the stored offset.
    -   Deserialize back into an `Entry`.
    -   **Promote to Hot**: Insert the entry back into `Shard.data`.
    -   **Trigger Eviction**: If this promotion fills memory, evict another cold entry to make room.
4.  **Return**: Serve the value to the caller.

## 3. Persistence Strategy (WAL)

Durability is configurable based on the use case (Cache vs. Task Store).

### Mode A: Cache (Ephemeral)
-   **Write Path**: Writes update Memory. If memory is full, cold data spills to Disk.
-   **Disk Usage**: Acts only as Swap.
-   **Restart**: Data is lost on restart (intended behavior for a pure cache).
-   **Performance**: Maximum throughput, no disk sync latency on writes.

### Mode B: Task Store (Durable)
-   **Write Path**: Every state change (SET, DEL, CAS) is appended to a **Write-Ahead Log (WAL)** *before* being applied to Memory.
-   **WAL Format**: `[CRC][TxID][OpCode][KeyLen][Key][ValLen][Value]`
-   **Sync Policy**: Configurable (`Always`, `EverySec`, `No`).
-   **Restart**: The system replays the WAL to reconstruct the Memory and Disk Index state.

## 4. On-Disk Storage Layout

The disk layer consists of two types of files:

### Data Files (Spillover)
-   **Purpose**: Random access storage for cold data.
-   **Structure**: Append-Only Logs or Page Files (e.g., `data.001.db`).
-   **Compaction**: A background thread monitors "wasted space" (overwritten/deleted data) and rewrites files to reclaim disk space.

### WAL Files (Persistence)
-   **Purpose**: Sequential record of all operations for durability.
-   **Structure**: Strictly append-only.
-   **Rotation**: Rotated based on size (e.g., every 64MB) or time.
