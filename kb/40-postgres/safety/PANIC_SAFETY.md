# Panic Boundary Protection

## Overview

The data-bridge-postgres PyO3 bindings include panic boundary protection to prevent Rust panics from crashing the entire Python process. This is critical for FFI safety because any unhandled panic that crosses the Rust/Python boundary will terminate the Python interpreter.

## Problem Statement

When Rust code panics and that panic propagates across an FFI boundary (like PyO3), it causes undefined behavior and typically crashes the entire process. In a Python application, this means:

1. No graceful error handling
2. Loss of all application state
3. No opportunity for Python exception handlers to catch the error
4. Difficult debugging (stack traces may be incomplete)

## Solution

We provide two wrapper functions that catch panics and convert them to Python exceptions:

### 1. `safe_call` - For Synchronous Operations

```rust
fn safe_call<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce() -> PyResult<T> + std::panic::UnwindSafe,
```

**Usage Example:**
```rust
#[pyfunction]
fn risky_sync_operation(value: i32) -> PyResult<i32> {
    safe_call(|| {
        // Code that might panic
        if value < 0 {
            panic!("Value must be non-negative");
        }
        Ok(value * 2)
    })
}
```

**How it works:**
- Uses `std::panic::catch_unwind` to catch panics
- Converts panic payload to a descriptive error message
- Returns `PyRuntimeError` with panic details

### 2. `safe_call_async` - For Async Operations

```rust
fn safe_call_async<'py, F, T>(py: Python<'py>, f: F) -> PyResult<Bound<'py, PyAny>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPyObject<'py, Target = PyAny> + Send + 'static,
    T::Error: Into<PyErr>,
```

**Usage Example:**
```rust
#[pyfunction]
fn risky_async_operation<'py>(py: Python<'py>, value: i32) -> PyResult<Bound<'py, PyAny>> {
    safe_call_async(py, async move {
        // Async code that might panic
        if value < 0 {
            panic!("Value must be non-negative");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(value * 2)
    })
}
```

**How it works:**
- Uses `futures::FutureExt::catch_unwind` to catch panics in async code
- Wraps the future with `AssertUnwindSafe`
- Converts panic to `PyRuntimeError` before returning to Python
- Integrates with `pyo3_async_runtimes::tokio::future_into_py`

## When to Use

### Critical Entry Points (MUST USE)

These are the most important functions to protect because they are frequently called from Python:

1. **Database operations**: `insert_one`, `insert_many`, `update_one`, `delete_one`, etc.
2. **Query execution**: `execute`, `fetch_one`, `fetch_all`, `find_many`
3. **Transaction operations**: `begin_transaction`, `commit`, `rollback`
4. **Connection management**: `init`, `close`, `is_connected`
5. **Migration operations**: `migration_apply`, `migration_rollback`

### Helper Functions (OPTIONAL)

Internal helper functions that are only called from Rust don't strictly need panic protection, but it's still good practice for:

- Functions that perform complex transformations
- Functions that might have bugs during development
- Functions that handle user-provided data

## Implementation Status

Currently, the panic boundary protection helpers are defined in `crates/data-bridge/src/postgres.rs` but **not yet applied to all entry points**.

### TODO: Wrap Critical Entry Points

The following functions should be wrapped with panic protection:

- [ ] `init` - Connection initialization
- [ ] `execute` - Raw SQL execution
- [ ] `insert_one`, `insert_many` - Insert operations
- [ ] `upsert_one`, `upsert_many` - Upsert operations
- [ ] `fetch_one`, `fetch_all` - Fetch operations
- [ ] `fetch_one_with_relations`, `fetch_many_with_relations` - Relation fetching
- [ ] `fetch_one_eager` - Eager loading
- [ ] `update_one`, `update_many` - Update operations
- [ ] `delete_one`, `delete_many` - Delete operations
- [ ] `delete_with_cascade`, `delete_checked` - Advanced delete operations
- [ ] `count` - Count queries
- [ ] `begin_transaction` - Transaction start
- [ ] `PyTransaction::commit` - Transaction commit
- [ ] `PyTransaction::rollback` - Transaction rollback
- [ ] `PyTransaction::execute` - Transaction execute
- [ ] Migration functions: `migration_init`, `migration_status`, `migration_apply`, `migration_rollback`, `migration_create`, `autogenerate_migration`
- [ ] Schema introspection: `list_tables`, `table_exists`, `get_columns`, `get_indexes`, `get_foreign_keys`, `get_backreferences`, `inspect_table`
- [ ] `find_by_foreign_key` - Foreign key lookup
- [ ] `find_many` - Multi-record fetch

## Best Practices

### 1. Minimize UnwindSafe Violations

Some types are not `UnwindSafe` (like `Rc`, `RefCell`, etc.). If you need to use them:

```rust
// Explicitly mark as UnwindSafe if you're sure it's safe
use std::panic::AssertUnwindSafe;

safe_call(|| {
    let data = AssertUnwindSafe(my_refcell);
    // ... use data
    Ok(())
})
```

### 2. Provide Context in Error Messages

The panic boundary provides basic panic messages, but you can add more context:

```rust
safe_call(|| {
    do_risky_operation()
        .map_err(|e| PyRuntimeError::new_err(
            format!("Failed to process user data: {}", e)
        ))
})
```

### 3. Don't Catch Panics Too Broadly

While panic protection is important at FFI boundaries, internal Rust-only code should still panic normally for programming errors. Use panic protection at:

- PyO3 `#[pyfunction]` entry points
- PyO3 `#[pymethods]` implementations
- Callbacks from Python code

### 4. Test Panic Scenarios

Add tests that verify panics are properly caught:

```rust
#[test]
fn test_panic_boundary() {
    Python::with_gil(|py| {
        let result = risky_operation(py, -1);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("panic"));
    });
}
```

## Error Message Format

Panics are converted to Python exceptions with descriptive messages:

```
RuntimeError: Rust panic in data-bridge-postgres: Value must be non-negative
RuntimeError: Rust panic in async data-bridge-postgres operation: Connection closed unexpectedly
RuntimeError: Rust panic in data-bridge-postgres: unknown error
```

The message includes:
1. Origin: "Rust panic in data-bridge-postgres" or "Rust panic in async data-bridge-postgres operation"
2. Details: The panic message if available, otherwise "unknown error"

## Performance Impact

Panic protection has minimal performance impact:

- `catch_unwind` has near-zero overhead when no panic occurs
- Only adds a few nanoseconds to function call time
- No heap allocations unless a panic actually occurs

The safety benefits far outweigh the negligible performance cost.

## References

- [PyO3 Error Handling](https://pyo3.rs/latest/function/error-handling.html)
- [Rust std::panic::catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- [futures::FutureExt::catch_unwind](https://docs.rs/futures/latest/futures/future/trait.FutureExt.html#method.catch_unwind)
- [UnwindSafe trait](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html)
