# Change: Add Rust-based ObjectId

## Why
Currently, the project relies on `pymongo`'s `bson.ObjectId` for handling ObjectIds in Python. This introduces a Python-side dependency and requires conversion overhead when passing ObjectIds between Python and Rust (serializing/deserializing).
By implementing `ObjectId` as a Rust struct (exposed via PyO3), we can:
1.  Eliminate the hard dependency on `pymongo` for core data types.
2.  Improve performance by keeping `ObjectId` as a Rust wrapper around `bson::oid::ObjectId`.
3.  Ensure "Zero Python Byte Handling" compliance for this fundamental type.

## What Changes
- Create a new module `crates/data-bridge/src/types.rs`.
- Implement `ObjectId` struct with PyO3 bindings.
- Expose `ObjectId` at `ouroboros.types.ObjectId` and `ouroboros.ObjectId`.
- Update `crates/data-bridge/src/conversion.rs` to support the new `ouroboros.ObjectId` in `extract_py_value` and `serializable_to_py_any`.
- Implement `__str__`, `__repr__`, `__eq__`, `__hash__` for Python compatibility.

## Impact
- **Affected Specs**: `data-types-objectid` (New capability)
- **Affected Code**:
    - `crates/data-bridge/src/lib.rs`
    - `crates/data-bridge/src/conversion.rs`
    - New file: `crates/data-bridge/src/types.rs`
