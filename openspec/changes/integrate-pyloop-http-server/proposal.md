# Change: Integrate PyLoop with HTTP Server

## Why
Currently, the `data-bridge` project uses a standard ASGI stack (uvloop + uvicorn + FastAPI) or a separate Tokio runtime for the Rust API server. This creates unnecessary overhead in context switching and data serialization (BSON -> Python -> JSON).

We have successfully implemented `data-bridge-pyloop` (Phase 3), which provides a Rust-native Python event loop on top of Tokio. By integrating this directly with our Rust-based Hyper/Axum HTTP server, we can achieve:
1.  **Unified Runtime**: A single Tokio runtime handling both HTTP I/O and Python tasks.
2.  **Zero-Copy Routing**: Request parsing and routing happen entirely in Rust (no GIL).
3.  **Hybrid Dispatch**: Pure Rust handlers avoid the GIL entirely; Python handlers are spawned efficiently on the event loop.

## What Changes
- **Architecture**: Replace the Uvicorn/ASGI entry point with a custom Rust entry point (`data_bridge.serve()`) that initializes the shared Tokio runtime and spawns the Hyper server.
- **Dispatch Logic**: Update `data-bridge-api` to support two handler types:
    - **Native Rust**: Executed directly on the I/O thread (async).
    - **Python Bridge**: Wraps Python coroutines in a `PyLoop::spawn` call, awaiting the result as a Rust Future.
- **New Feature**: Add a **Declarative DSL** for routes. Instead of writing boilerplate handlers, users can define a Pydantic model, and Rust will automatically generate high-performance CRUD endpoints.

## Impact
- **Affected Specs**: `api-server`
- **Affected Code**: 
    - `crates/data-bridge-api/` (Server logic, Handler traits)
    - `crates/data-bridge-pyloop/` (Expose `spawn` API for Rust)
    - `python/data_bridge/` (New entry point and decorators)
