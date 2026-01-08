---
title: Architecture Principles
status: implemented
component: general
type: principles
---

# Architecture Principles

> Part of [Project Overview](./00-index.md)

These core principles guide every architectural decision in `data-bridge`.

## 1. Zero Python Byte Handling
**Rule**: Never process raw BSON bytes in Python.

- **Why**: Decoding BSON in Python creates thousands of temporary dictionaries and strings, putting massive pressure on the Python heap and Garbage Collector.
- **Implementation**:
  - The Rust driver receives raw bytes.
  - Rust deserializes them into Rust structs (`ExtractedValue` or `BsonDocument`).
  - Filtering and transformation happen in Rust.
  - Only the final, required data is converted to Python objects.

## 2. GIL Release Strategy
**Rule**: Release the Global Interpreter Lock (GIL) for any operation > 1ms.

- **Why**: Python's GIL prevents true parallelism. To scale on multi-core CPUs, we must be in "Rust mode" (GIL released) as much as possible.
- **Implementation**:
  - **Released**: BSON conversion (CPU bound), Network I/O (IO bound), Validation.
  - **Held**: Only for extracting data from Python objects and creating final Python results.

## 3. Parallel Processing
**Rule**: Use data parallelism for batch operations.

- **Why**: A single thread cannot saturate modern network links or CPUs during serialization.
- **Implementation**:
  - Uses `rayon` for CPU-bound tasks (serialization/validation).
  - Uses `tokio` for IO-bound tasks.
  - **Threshold**: Operations with ≥50 documents automatically trigger parallel paths.

## 4. Copy-on-Write State Management
**Rule**: Never deepcopy documents for change tracking.

- **Why**: Deepcopy is `O(N)` and slow. Most updates only touch 1-2 fields.
- **Implementation**:
  - We store a reference to the initial state (Arc-like).
  - We only record changes when `__setattr__` is called.
  - `save()` only sends the diff.
  - **Result**: 10x faster than traditional "dirty flag" implementations.

## 5. Lazy Validation
**Rule**: Validate only when necessary.

- **Why**: Validating every field on read (database -> Python) is wasteful if the data is trusted (already in DB).
- **Implementation**:
  - **Reads**: Trust the DB types (mostly).
  - **Writes**: Full validation before sending to DB.
  - **Deferred**: Validation happens at `save()` time, not `__init__` time.

## 6. Security First
**Rule**: Safety at the boundary.

- **Why**: Crossing the FFI boundary is dangerous. Input from Python is "untrusted".
- **Implementation**:
  - **NoSQL Injection**: Validate all field names (no `$` prefixes) and collection names.
  - **Type Safety**: Rust's type system enforces correctness before runtime.
  - **Sanitization**: Error messages are sanitized to prevent credential leaks.

## Performance Targets

| Metric | Target | Current Status |
| :--- | :--- | :--- |
| **Inserts (1k docs)** | <20ms (≥2.8x vs Beanie) | 17.76ms (3.2x) ✅ |
| **Finds (1k docs)** | <7ms (≥1.2x vs Beanie) | 6.32ms (1.4x) ✅ |
| **Memory** | Minimal Python Heap | Verified |
| **Scalability** | Linear with Cores | Verified (Rayon) |
