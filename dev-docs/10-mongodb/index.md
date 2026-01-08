---
title: MongoDB Solution
status: implemented
component: mongodb
type: index
---

# MongoDB Solution

The MongoDB solution is the primary component of `data-bridge`. It provides a high-performance, Beanie-compatible ORM backed by a custom Rust engine.

## Components

### [1. Core Engine (Rust)](./01-core-engine/index.md)
The internal engine handling BSON serialization, connection pooling, and low-level optimization.
- **Crate**: `data-bridge-mongodb`
- **Documentation**: Architecture, components, and implementation details of the Rust layer.

### [2. Python API](./02-python-api/index.md)
The user-facing Python layer.
- **Package**: `data_bridge`
- **Documentation**: Document models, Query DSL, Field proxies, and usage patterns.

## Architecture

The system uses a "sandwich" architecture:
1.  **Python API**: Thin wrapper for developer experience.
2.  **PyO3 Bridge**: Handles type conversion and GIL release.
3.  **Rust Engine**: Executes heavy logic (BSON, I/O, Validation).
