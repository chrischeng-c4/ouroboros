# data-bridge-sheet-db - Implementation Todos

## In Progress
- [ ] None currently

## Pending

### Storage Layer
- [ ] Implement Morton encoding/decoding functions
  - [ ] `MortonKey::encode()` - Interleave bits of row/col
  - [ ] `MortonKey::decode()` - Deinterleave bits back to row/col
  - [ ] `MortonKey::range_for_rect()` - Calculate Morton ranges for rectangle
  - [ ] Add tests for spatial locality preservation

- [ ] Implement WriteAheadLog
  - [ ] `WriteAheadLog::new()` - Open/create WAL file
  - [ ] `WriteAheadLog::write_entry()` - Write and flush entry
  - [ ] `WriteAheadLog::read_entries()` - Read all entries
  - [ ] `WriteAheadLog::replay()` - Replay entries for crash recovery
  - [ ] `WriteAheadLog::checkpoint()` - Write checkpoint marker
  - [ ] `WriteAheadLog::truncate()` - Truncate old entries
  - [ ] Add tests for crash recovery

- [ ] Implement CellStore
  - [ ] `CellStore::new()` - Initialize store with KV engine and WAL
  - [ ] `CellStore::get_cell()` - Retrieve cell by coordinates
  - [ ] `CellStore::set_cell()` - Update cell value
  - [ ] `CellStore::delete_cell()` - Delete cell
  - [ ] `CellStore::query_range()` - Query rectangular range
  - [ ] `CellStore::flush()` - Flush WAL and sync
  - [ ] `CellStore::stats()` - Collect store statistics
  - [ ] Add integration tests with KV engine

### Query Layer
- [ ] Implement RangeQuery execution
  - [ ] `RangeQuery::matches_filter()` - Filter matching logic
  - [ ] `RangeQuery::apply_sort()` - Sort results
  - [ ] Add tests for filter combinations

- [ ] Implement SpatialQuery execution
  - [ ] `SpatialQuery::execute()` - Dispatch to query handlers
  - [ ] `SpatialQuery::find_nearest_neighbors()` - K-NN search
  - [ ] `SpatialQuery::find_within_radius()` - Radius search
  - [ ] `SpatialQuery::detect_clusters()` - DBSCAN clustering
  - [ ] `Cluster::new()` - Calculate centroid and bounds
  - [ ] Add tests for spatial queries

### CRDT Layer
- [ ] Implement CRDT operations
  - [ ] `CrdtOperation::compare_lww()` - Last-Write-Wins comparison
  - [ ] `CrdtOperation::happened_before()` - Causality check
  - [ ] `VectorClock::happened_before()` - Vector clock comparison
  - [ ] `merge_operations()` - Merge conflicting operations
  - [ ] `apply_operation()` - Apply operation to cell
  - [ ] Add tests for conflict resolution

### Integration
- [ ] Add integration tests
  - [ ] Test full CRUD lifecycle
  - [ ] Test concurrent operations
  - [ ] Test WAL recovery
  - [ ] Test range queries with Morton encoding
  - [ ] Benchmark performance vs. naive storage

### Documentation
- [ ] Add module-level examples
- [ ] Document performance characteristics
- [ ] Add architecture diagrams
- [ ] Document CRDT conflict resolution strategy

## Completed
- [x] ✅ Created crate structure (2026-01-08)
- [x] ✅ Created all module files with documentation (2026-01-08)
- [x] ✅ Added placeholder types and functions (2026-01-08)
- [x] ✅ Fixed workspace dependencies (2026-01-08)
- [x] ✅ Verified crate compiles (2026-01-08)
