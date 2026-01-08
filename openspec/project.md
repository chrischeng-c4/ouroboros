# Project Overview

## 1. Purpose
**data-bridge** is a unified high-performance data platform designed to bridge the gap between Python's ease of use and Rust's performance. It serves as a foundational infrastructure layer combining:
*   **High-Performance MongoDB ORM**: A Beanie-compatible ORM that offloads all BSON serialization, deserialization, and validation to Rust, achieving 1.4-5.4x speed improvements.
*   **Spreadsheet Engine**: A WASM-powered spreadsheet core with real-time collaboration (CRDTs), formula evaluation, and canvas rendering.
*   **Unified Infrastructure**: Integrated HTTP Client, Task Queue, and Key-Value Store, all backed by Rust for optimal concurrency and resource usage.

The primary goal is to solve the "Python serialization bottleneck" in data-intensive applications by enforcing a **Zero Python Byte Handling** policy.

## 2. Tech Stack

### Core
*   **Languages**: Rust (1.70+, Edition 2021), Python (3.12+)
*   **Bridge**: PyO3 (0.24+), Maturin (Build System)
*   **Runtime**: Tokio (Async Rust), uvloop (Async Python)

### Backend (Rust Crates)
*   **Database**: `mongodb` (Rust driver), `bson`, `postgres`, `deadpool`
*   **Web/Net**: `axum`, `reqwest`, `tower`
*   **Concurrency**: `rayon` (Parallelism), `dashmap`, `async-nats`
*   **Utils**: `serde`, `thiserror`, `tracing`, `chrono`, `rust_decimal`

### Frontend (Spreadsheet)
*   **Framework**: React, TypeScript, Vite
*   **WASM**: `wasm-pack`, `wasm-bindgen`
*   **Collaboration**: Yjs (CRDTs), WebSocket
*   **Rendering**: Canvas API

### Python Ecosystem
*   **Manager**: `uv` (Package management)
*   **Testing**: `pytest`, `pytest-asyncio`, `pytest-benchmark`
*   **Linting**: `ruff`, `mypy`

## 3. Project Conventions

### Architecture Patterns
*   **Hybrid Runtime**: Python handles high-level application logic; Rust handles core data processing and I/O.
*   **Zero-Copy**: Data is processed in Rust structs; Python receives only final typed objects. No intermediate BSON/bytes in Python heap.
*   **Lazy Evaluation**: Validation and heavy processing are deferred to the Rust layer upon execution (e.g., `save()`).
*   **Workspace Structure**: Monorepo-style Rust workspace (`crates/*`) with a single Python package entry point.

### Code Style
*   **Rust**: Standard `rustfmt` and strict `clippy` rules (`deny.toml` configured).
*   **Python**: PEP 8 via `ruff`, strict type checking via `mypy`.
*   **Documentation**: Markdown files in `docs/`, inline Rust docs (`cargo doc`), Python docstrings.

## 4. Testing Strategy

### Frameworks
*   **Rust**: `cargo test` for unit tests within crates.
*   **Python**: `pytest` for integration and API-level tests.
*   **Runner**: `dbtest` (custom CLI wrapper via `just test`).

### Test Levels
1.  **Unit (Rust)**: Core logic, parsers, algorithms.
2.  **Unit (Python)**: `SKIP_INTEGRATION=true` mode for logic testing without DB.
3.  **Integration**: Full stack tests against running MongoDB/PostgreSQL containers.
4.  **Benchmarks**: `benchmarks/` directory using `pytest-benchmark` to verify performance gains vs Beanie/PyMongo.
5.  **Frontend**: `pnpm test` (Unit) and E2E tests.

## 5. Git Workflow

*   **Commits**: Conventional Commits (e.g., `feat:`, `fix:`, `perf:`, `docs:`).
*   **Hooks**: Pre-commit hooks via `pre-commit` (configured in `.pre-commit-config.yaml`).
*   **CI**: GitHub Actions for linting, testing, and documentation.
*   **Local**: Use `just dev-quick` for rapid iteration and `just pre-commit` before pushing.

## 6. Domain Context

### MongoDB ORM
*   **Problem**: Python drivers spend significant CPU time converting BSON -> Dict -> Model.
*   **Solution**: `data-bridge` deserializes BSON directly to Rust structs. Python objects are created only when needed, often lazily.
*   **API**: Maintains strict compatibility with **Beanie** to allow drop-in migration.

### Spreadsheet Engine
*   **Problem**: Browser-based spreadsheets lag with large datasets or complex formulas.
*   **Solution**: Formula execution and data storage happen in WASM (Rust). UI updates via Canvas.
*   **Collaboration**: Uses CRDTs (Conflict-free Replicated Data Types) for seamless multi-user editing.

## 7. Important Constraints

*   **Versioning**:
    *   Rust: >= 1.70 (Required for newer language features)
    *   Python: >= 3.12 (Required for latest PyO3/performance features)
    *   PyO3: >= 0.24 (Mandatory for security fix RUSTSEC-2025-0020)
*   **Performance**:
    *   Must maintain >1.4x speedup over Beanie for core operations.
    *   Strict "No GIL holding" during I/O or heavy computation.
*   **Compatibility**: Changes must not break the Beanie-compatible public API unless explicitly documented as a breaking change.

## 8. External Dependencies

*   **Databases**:
    *   MongoDB (3.6+)
    *   PostgreSQL (13+)
*   **Infrastructure**:
    *   Docker (Compose used for local dev stacks: `compose.yaml`)
*   **Frontend**:
    *   Node.js (LTS) & pnpm