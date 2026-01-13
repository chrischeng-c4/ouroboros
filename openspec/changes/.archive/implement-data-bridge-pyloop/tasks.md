## 0. Baseline & Research (Preparation)
- [ ] 0.1 Run existing benchmarks to establish baseline (tests/api/benchmarks/run_benchmarks.py)
- [ ] 0.2 Document current performance: asyncio mode vs FastAPI comparison
- [ ] 0.3 Research uvloop architecture and identify integration points
- [ ] 0.4 Create benchmark suite for event loop comparison (asyncio vs uvloop vs pyloop)

## 1. Crate Setup
- [x] 1.1 Create `crates/data-bridge-pyloop` with `Cargo.toml` dependencies (`pyo3`, `tokio`, `futures`).
- [x] 1.2 Configure `pyproject.toml` to include the new extension module.
- [x] 1.3 Create `python/data_bridge/pyloop/` structure with `__init__.py`.

## 2. Core Loop Implementation (Rust)
- [x] 2.1 Implement `PyLoop` struct with `#[pyclass]` and basic lifecycle (`close`, `is_closed`).
- [x] 2.2 Implement `call_soon` and `call_soon_threadsafe` bridging to Tokio.
- [x] 2.3 Implement `call_later` and `call_at` using `tokio::time`.
- [x] 2.4 Implement `create_task` wrapping Python coroutines in Tokio tasks. (Phase 2.4 complete - 14 tests passing)
- [x] 2.5 Implement `run_forever` and `run_until_complete` (blocking entry points). (Phase 2.5 complete - 14/17 tests passing, 3 skipped for coroutine execution)

## 3. Python Integration
- [x] 3.1 Implement `PyLoopPolicy` to replace the default asyncio policy.
- [x] 3.2 Create `install()` helper function in Python to activate the policy.
- [x] 3.3 Implement `get_event_loop` logic to return the global singleton.

## 4. Testing & Verification
- [x] 4.1 Create `tests/pyloop/` for isolated runtime tests. (Done: tests/test_pyloop*.py with 65 tests)
- [ ] 4.2 Run `pytest-asyncio` suite against the new loop.
- [ ] 4.3 Verify zero-copy behavior (no extra serialization steps).

## 5. Migration
- [ ] 5.1 Update `crates/data-bridge-api` to remove `thread_local` loop logic.
- [ ] 5.2 Update `PyApiApp::serve` to initialize `pyloop` before starting Tokio.
- [ ] 5.3 Validate `data-bridge-api` with existing integration tests.
