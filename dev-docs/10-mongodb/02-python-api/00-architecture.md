---
title: Python API Architecture
status: implemented
component: data-bridge-python
type: architecture
---

# Python Layer Architecture

> Part of [Python API Layer Documentation](./index.md)

## High-Level Design

The Python layer is designed to be "thin but smart". It handles the syntactic sugar that makes the library pleasant to use (operator overloading, type hints, metaclasses) but strictly avoids heavy data processing.

### 1. The Proxy Pattern (Query DSL)
To enable syntax like `User.age > 18`, we use a proxy pattern.
The `Document` class uses a metaclass to replace type-hinted fields with `FieldProxy` descriptors on the class level.

- **Class Access** (`User.age`): Returns a `FieldProxy` that records operations (e.g., `> 18` becomes `{"age": {"$gt": 18}}`).
- **Instance Access** (`user.age`): Returns the actual value (e.g., `30`).

### 2. Copy-on-Write State Management
Traditional ORMs often use "dirty flags" or deep copies to track changes. Deep copies are slow (`O(N)`).
We use a **Copy-on-Write (COW)** approach in `state.py`.

- **Initial State**: When a document is loaded, we store a reference to the initial values.
- **Modification**: We only record changes when a `__setattr__` occurs.
- **Comparison**: `save()` compares current values against the initial state only for fields that were "touched".
- **Performance**: 10x faster than deepcopy-based tracking for large documents.

### 3. The Bridge Pattern
`_engine.py` serves as the **Bridge** to the Rust backend. It decouples the Python API from the implementation details of the Rust extension.

- **Abstraction**: `find_one`, `insert_one`, etc.
- **Implementation**: `data_bridge.mongodb.RustDocument`.

This allows us to potentially mock the backend for unit tests or swap implementations if needed.

## Schema Extraction Architecture

Unlike Beanie (which uses Pydantic for everything), we perform **Hybrid Validation**.

1.  **Extraction**: `type_extraction.py` analyzes Python type hints at startup.
2.  **Conversion**: It converts them into a Rust-compatible schema definition.
3.  **Enforcement**: Rust enforces these types during BSON serialization.

**Benefit**: We get the developer experience of Pydantic without the runtime overhead of validating every field in Python.

## Link Resolution

Handling relations (`Link`, `BackLink`) is complex. We use a **Lazy Resolution** strategy.

1.  **Definition**: Links are defined as generic types `Link[OtherDoc]`.
2.  **Storage**: In the DB, we store `DBRef` or just `ObjectId`.
3.  **Access**: Accessing the field returns the ID/Ref initially.
4.  **Fetch**: `.fetch_link()` or `fetch_all_links()` triggers a query to load the related document(s).

*Note: This is one area where Python logic is heavier, as it involves coordinating multiple queries.*
