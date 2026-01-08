---
title: Python API Overview
status: implemented
component: data-bridge-python
type: index
---

# Python API Layer Architecture

## Overview

The Python API Layer (`python/data_bridge/`) is the user-facing part of `data-bridge`. It is designed to be a drop-in replacement for **Beanie**, providing a familiar, pythonic interface for MongoDB interactions while delegating all performance-critical operations to the Rust backend.

**Key Features**:
- **Beanie Compatibility**: Mimics the Beanie API (Documents, QueryBuilder, Links).
- **Type Safety**: Leverages Python type hints and Pydantic for schema definition.
- **Efficient State Tracking**: Uses Copy-on-Write (COW) logic to track changes with minimal overhead.
- **Fluent Query API**: A rich DSL for building MongoDB queries using pythonic expressions.
- **Rust Integration**: Acts as a thin wrapper around the high-performance Rust engine.

## Architecture Layers

```mermaid
graph TB
    User[User Application]
    Doc[Document Layer<br/>document.py]
    Query[Query Builder<br/>query.py / fields.py]
    State[State Management<br/>state.py]
    Engine[Engine Bridge<br/>_engine.py]
    Rust[Rust Backend<br/>data_bridge (PyO3)]

    User --> Doc
    User --> Query
    Doc --> State
    Doc --> Engine
    Query --> Engine
    Engine --> Rust

    style User fill:#e1f5ff
    style Doc fill:#fff9c4
    style Query fill:#fff9c4
    style State fill:#fff9c4
    style Engine fill:#ffccbc
    style Rust fill:#ffccbc
```

## Documentation Structure

### 1. [00-architecture.md](./00-architecture.md)
High-level architectural patterns, including:
- The **Proxy Pattern** for field access and query building.
- The **Copy-on-Write** strategy for state tracking.
- The **Bridge Pattern** for Rust integration.

### 2. [10-components.md](./10-components.md)
Detailed breakdown of key components:
- **Document**: The core model class and its metaclass magic.
- **FieldProxy**: How `User.name == "Alice"` works.
- **QueryBuilder**: Implementation of the fluent API (`.find(...).sort(...)`).
- **StateTracker**: Efficient change detection logic.

### 3. [20-data-flows.md](./20-data-flows.md)
Sequence diagrams illustrating:
- **Query Construction**: Python expression → MongoDB Filter.
- **Document Hydration**: Rust BSON → Python Object.
- **Save Lifecycle**: Change detection → Validation → Rust Insert/Update.

### 4. [30-implementation-details.md](./30-implementation-details.md)
Implementation details:
- Metaclass implementation (`DocumentMeta`).
- Type extraction logic (`type_extraction.py`).
- Link resolution strategies.

## Success Criteria

- ✅ **API Compatibility**: Supports 90%+ of Beanie's core API.
- ✅ **Type Hints**: Fully typed for excellent IDE support (VS Code/PyCharm).
- ✅ **Low Overhead**: Minimal Python code execution on hot paths.
- ✅ **Developer Experience**: Clear error messages and intuitive API.

## References

- **Python Source**: `python/data_bridge/`
- **Beanie Docs**: Reference for API compatibility.
