## 1. Implementation
- [ ] 1.1 Create `crates/data-bridge/src/types.rs`
- [ ] 1.2 Implement `ObjectId` struct wrapping `bson::oid::ObjectId`
- [ ] 1.3 Implement `ObjectId::new()` (static method)
- [ ] 1.4 Implement `ObjectId::is_valid(str)` (static method)
- [ ] 1.5 Implement `ObjectId::from_str(str)` (static method, replacing `__init__` parsing or supplementing it)
- [ ] 1.6 Implement `__str__`, `__repr__`, `__eq__`, `__hash__`
- [ ] 1.7 Register `types` module in `crates/data-bridge/src/lib.rs` and export `ObjectId`
- [ ] 1.8 Update `crates/data-bridge/src/conversion.rs` `extract_py_value` to detect `ouroboros.ObjectId`
- [ ] 1.9 Update `crates/data-bridge/src/conversion.rs` `serializable_to_py_any` to return `ouroboros.ObjectId`

## 2. Testing
- [ ] 2.1 Add Rust unit tests for `types.rs`
- [ ] 2.2 Add Python tests verifying `ObjectId` creation and validation
- [ ] 2.3 Add Python tests verifying BSON serialization/deserialization with the new type
