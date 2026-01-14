## 1. Implementation
- [x] 1.1 Create `crates/data-bridge/src/types.rs`
- [x] 1.2 Implement `ObjectId` struct wrapping `bson::oid::ObjectId`
- [x] 1.3 Implement `ObjectId::new()` (static method)
- [x] 1.4 Implement `ObjectId::is_valid(str)` (static method)
- [x] 1.5 Implement `ObjectId::from_str(str)` (static method, replacing `__init__` parsing or supplementing it)
- [x] 1.6 Implement `__str__`, `__repr__`, `__eq__`, `__hash__`
- [x] 1.7 Register `types` module in `crates/data-bridge/src/lib.rs` and export `ObjectId`
- [x] 1.8 Update `crates/data-bridge/src/conversion.rs` `extract_py_value` to detect `ouroboros.ObjectId`
- [x] 1.9 Update `crates/data-bridge/src/conversion.rs` `serializable_to_py_any` to return `ouroboros.ObjectId`

## 2. Testing
- [x] 2.1 Add Rust unit tests for `types.rs`
- [x] 2.2 Add Python tests verifying `ObjectId` creation and validation
- [x] 2.3 Add Python tests verifying BSON serialization/deserialization with the new type

## 3. Python Integration (Additional)
- [x] 3.1 Update `python/ouroboros/__init__.py` to export `ObjectId`
- [x] 3.2 Update `python/ouroboros/mongodb/types.py` to use `ouroboros.ObjectId`
- [x] 3.3 Update `python/ouroboros/mongodb/links.py` to use `ouroboros.ObjectId`
- [x] 3.4 Update tests to use `ouroboros.ObjectId` instead of `bson.ObjectId`
