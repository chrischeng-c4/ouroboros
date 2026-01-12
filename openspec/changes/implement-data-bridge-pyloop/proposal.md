# Change: Implement data-bridge-pyloop

## Why
The current architecture uses thread-local `asyncio` event loops (one per Tokio worker thread) to execute Python coroutines. This violates Python best practices (expecting a single loop per process) and creates significant overhead when bridging between Rust and Python, limiting performance to ~1x FastAPI. To achieve the 1.5-2x performance target, we need a unified, zero-copy, pure-Rust execution environment that tightly integrates Tokio with Python's coroutine model.

## What Changes
- **New Crate**: `crates/data-bridge-pyloop` (Pure Rust implementation of Python's async runtime).
- **New Python Module**: `python/data_bridge/pyloop/` (Exposes the runtime to Python).
- **Architecture Shift**: Replaces thread-local event loops with a single, global, thread-safe event loop backed by the main Tokio runtime.
- **Integration**: Updates `data-bridge-api` to use this new runtime for all async execution.

## Impact
- **Affected Specs**:
    - `python-runtime` (New capability)
    - `api-server` (Implementation detail change, performance improvement)
- **Affected Code**:
    - `crates/data-bridge/src/api.rs`: Remove thread-local loop logic.
    - `crates/data-bridge-pyloop/`: New codebase.
    - `python/data_bridge/`: New `pyloop` integration.
