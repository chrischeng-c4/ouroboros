# Architecture Design: API Performance Optimization

## Context
The `data-bridge-api` server currently uses a "Two-Phase GIL" pattern where requests are extracted into a `SerializableRequest` struct before being processed. While this separates I/O from Python logic, the extraction phase currently performs deep copies of body data (`Vec<u8>`) and uses standard `serde_json` for parsing, creating a bottleneck.

## Goals
- **Minimize Allocations**: Reduce memory allocation frequency by using `Bytes` and borrowing where possible.
- **Maximize Serialization Speed**: Use SIMD-accelerated JSON parsing (`sonic-rs`) for both ingress and egress.
- **Optimal Concurrency**: Ensure the Rust runtime drives I/O efficiently without blocking on Python GIL.

## Decisions

### 1. Adopt `bytes::Bytes` for Request Body
- **Current**: `Vec<u8>` is allocated and copied from Hyper's incoming stream.
- **New**: Use `bytes::Bytes`. Hyper produces `Bytes` frames. We will collect them into a single `Bytes` object (cheap ref-counting) for the `SerializableRequest`.
- **Rationale**: Avoids copying large payloads. `sonic-rs` and other parsers can work directly on `&[u8]` derived from `Bytes`.

### 2. Universal `sonic-rs` Usage
- **Current**: `serde_json` for Request parsing, `sonic-rs` for Response serialization.
- **New**: Use `sonic-rs` for **both**.
- **Rationale**: `sonic-rs` offers significant speedups (2-3x) over `serde_json`, especially for large payloads.

### 3. Connection & Buffer Tuning
- **Decision**: Configure Hyper's `http1` builder:
  - `keep_alive(true)`
  - `half_close(true)`
  - `pipeline_flush(true)`
- **Rationale**: Reduce TCP connection overhead for high-throughput scenarios.

## Risks
- **`sonic-rs` Safety**: `sonic-rs` uses `unsafe` for performance. We must ensure it is used safely within the `unsafe` boundaries it provides.
- **Python Compatibility**: Changing `SerializableRequest` internal structure must not break the PyO3 conversion logic (though `SerializableRequest` is primarily a Rust-side intermediate).

## Alternatives
- **Zero-Copy to Python (PyBuffer)**: We considered passing raw pointers/buffers to Python directly.
  - *Rejected*: Too complex for the current stage. The "Two-Phase" extraction to native Rust types (Strings, JSON Value) is safer and still very fast if the Rust side is optimized. We stick to converting to Python objects at the boundary, but optimize the *Rust* side of that boundary.
