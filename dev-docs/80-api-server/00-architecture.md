# API Server Architecture

## Overview

The `data-bridge-api` is a hybrid Rust/Python web framework designed to provide the developer experience of FastAPI with the performance of a native Rust server.

### Core Philosophy

1.  **Rust for Heavy Lifting**: Routing, Validation, Serialization, and HTTP handling happen in Rust.
2.  **Python for Business Logic**: Route handlers and Dependency definitions are written in Python.
3.  **Zero-Copy (Where Possible)**: Minimizing overhead at the FFI boundary.

## Comparison with FastAPI

| Feature | FastAPI / Uvicorn | Data Bridge API |
| :--- | :--- | :--- |
| **Language** | Python (Starlette) | Rust (Hyper/Tokio) |
| **JSON** | Standard `json` or `orjson` | `sonic-rs` (Rust) |
| **Validation** | Pydantic (Python) | Rust-native validation |
| **Routing** | Python Regex/Match | Rust Match (High Perf) |
| **Concurrency** | Python `asyncio` loop | Tokio Runtime + Python `asyncio` |

## Key Components

### 1. The Router
The router is implemented in Rust and handles URL matching before any Python code executes. This prevents GIL contention for 404s or invalid methods.

### 2. Dependency Injection
We use **Kahn's Algorithm** to resolve the dependency graph.
- Dependencies are defined in Python.
- The graph is resolved at startup.
- Execution plan is cached.
- Scoped dependencies (Singleton, Request) are supported.

### 3. Middleware Chain
Middleware is handled in a unified chain that can interleave Rust and Python middleware, though the core execution pipeline is optimized in Rust.
