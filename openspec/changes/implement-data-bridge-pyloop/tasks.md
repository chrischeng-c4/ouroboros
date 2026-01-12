## 1. Crate Setup
- [ ] 1.1 Create `crates/data-bridge-pyloop` with `Cargo.toml` dependencies (`pyo3`, `tokio`, `futures`).
- [ ] 1.2 Configure `pyproject.toml` to include the new extension module.
- [ ] 1.3 Create `python/data_bridge/pyloop/` structure with `__init__.py`.

## 2. Core Loop Implementation (Rust)
- [ ] 2.1 Implement `PyLoop` struct with `#[pyclass]` and basic lifecycle (`close`, `is_closed`).
- [ ] 2.2 Implement `call_soon` and `call_soon_threadsafe` bridging to Tokio.
- [ ] 2.3 Implement `call_later` and `call_at` using `tokio::time`.
- [ ] 2.4 Implement `create_task` wrapping Python coroutines in Tokio tasks.
- [ ] 2.5 Implement `run_forever` and `run_until_complete` (blocking entry points).

## 3. Python Integration
- [ ] 3.1 Implement `PyLoopPolicy` to replace the default asyncio policy.
- [ ] 3.2 Create `install()` helper function in Python to activate the policy.
- [ ] 3.3 Implement `get_event_loop` logic to return the global singleton.

## 4. Testing & Verification
- [ ] 4.1 Create `tests/pyloop/` for isolated runtime tests.
- [ ] 4.2 Run `pytest-asyncio` suite against the new loop.
- [ ] 4.3 Verify zero-copy behavior (no extra serialization steps).

## 5. Migration
- [ ] 5.1 Update `crates/data-bridge-api` to remove `thread_local` loop logic.
- [ ] 5.2 Update `PyApiApp::serve` to initialize `pyloop` before starting Tokio.
- [ ] 5.3 Validate `data-bridge-api` with existing integration tests.
