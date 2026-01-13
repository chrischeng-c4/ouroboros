# CLAUDE.md

<language>
Respond in English (U.S.) by default. Use Traditional Chinese only when user writes in Traditional Chinese.
</language>

---

<decision-trees>

<tree name="Component Selection">
START: What component does this touch?
│
├─► Spreadsheet (cells, formulas, collaboration)?
│   │
│   ├─► Core data structures (cells, grid, formatting)?
│   │   └─ YES → Component: data-bridge-sheet-core
│   │            Files: crates/data-bridge-sheet-core/src/
│   │
│   ├─► Formula parsing/evaluation?
│   │   └─ YES → Component: data-bridge-sheet-formula
│   │            Files: crates/data-bridge-sheet-formula/src/
│   │
│   ├─► Undo/Redo system?
│   │   └─ YES → Component: data-bridge-sheet-history
│   │            Files: crates/data-bridge-sheet-history/src/
│   │
│   ├─► Custom database (Morton encoding, KV)?
│   │   └─ YES → Component: data-bridge-sheet-db
│   │            Files: crates/data-bridge-sheet-db/src/
│   │
│   ├─► Collaboration server (Axum, Yjs, WebSocket)?
│   │   └─ YES → Component: data-bridge-sheet-server
│   │            Files: crates/data-bridge-sheet-server/src/
│   │
│   ├─► WebAssembly bindings (WASM)?
│   │   └─ YES → Component: data-bridge-sheet-wasm
│   │            Files: crates/data-bridge-sheet-wasm/src/
│   │
│   └─► Frontend (TypeScript, Canvas, UI)?
│       └─ YES → Component: Frontend
│                Files: frontend/src/
│
├─► MongoDB ORM (CRUD, queries, aggregation)?
│   └─ YES → Component: data-bridge-mongodb (pure Rust)
│                Files: crates/data-bridge-mongodb/src/
│                PyO3: crates/data-bridge/src/mongodb.rs
│
├─► HTTP client (requests, responses, pooling)?
│   └─ YES → Component: data-bridge-http (pure Rust)
│                Files: crates/data-bridge-http/src/
│                PyO3: crates/data-bridge/src/http.rs
│
├─► Test framework (benchmarks, assertions)?
│   └─ YES → Component: data-bridge-test (pure Rust)
│                Files: crates/data-bridge-test/src/
│                PyO3: crates/data-bridge/src/test.rs
│
├─► Python API (Document, QueryBuilder, fields)?
│   └─ YES → Component: Python layer
│                Files: python/data_bridge/
│
├─► PyO3 bindings (Rust ↔ Python bridge)?
│   └─ YES → Component: PyO3 layer
│                Files: crates/data-bridge/src/
│
└─► Common utilities (errors, types)?
    └─ YES → Component: data-bridge-common
               Files: crates/data-bridge-common/src/
</tree>

<tree name="Performance Verification">
START: Feature complete?
│
├─► Does feature affect CRUD operations?
│   └─ YES → Run benchmarks/bench_comparison.py
│             Compare: data-bridge vs Beanie vs PyMongo
│             Target: ≥1.4x faster than Beanie
│
├─► Does feature affect BSON serialization?
│   └─ YES → Run micro-benchmarks
│             Measure: GIL release effectiveness
│             Target: No Python heap pressure
│
├─► Does feature add validation?
│   └─ YES → Measure overhead
│             Lazy vs eager validation
│             Target: <5% overhead
│
└─► New bulk operation?
    └─ YES → Test batch sizes (10, 100, 1000, 10000)
             Verify: Rayon parallelization ≥50 docs
             Target: Linear scaling
</tree>

<tree name="Testing Strategy">
START: What layer needs testing?
│
├─► Pure Rust logic (no Python)?
│   └─ YES → cargo test -p {crate}
│             Unit tests in src/ or tests/
│             No MongoDB required
│
├─► PyO3 bindings (Rust → Python)?
│   └─ YES → Use data-bridge-test (Rust runner)
│             Write test in Python, run with data-bridge-test
│             Example: tests/unit/test_*.py
│             uv run python tests/unit/test_*.py
│
├─► Integration (MongoDB required)?
│   └─ YES → Use data-bridge-test (Rust runner)
│             Requires MongoDB on localhost:27017
│             Full CRUD lifecycle
│             uv run python tests/integration/test_*.py
│
├─► Performance benchmarks?
│   └─ YES → Use data-bridge-test (Rust runner)
│             Example: tests/api/benchmarks/bench_comparison_rust.py
│             uv run python benchmarks/bench_*.py --rounds 5 --warmup 2
│
└─► Benchmark pytest vs data-bridge-test?
    └─ YES → Use pytest ONLY for comparison
             Compare: pytest-benchmark vs data-bridge-test
             Verify data-bridge-test is faster
</tree>

</decision-trees>

---

<few-shot-examples>

<example name="Commit Format">
feat(108): add email/url constraint validation
fix(103): correct GIL release in bulk operations
test(106): add complex type validation tests
perf(201): optimize bulk insert for 50K+ documents
</example>

</few-shot-examples>

---

<grounding>

<repository-structure>
data-bridge/
├── Cargo.toml                      # Workspace root
├── CLAUDE.md                       # This file
├── pyproject.toml                  # Python package config (Maturin)
├── justfile                        # Build & test commands
│
├── crates/                         # Rust workspace
│   ├── data-bridge/                # PyO3 bindings (main entry point)
│   │   └── src/
│   │       ├── lib.rs              # Module registration
│   │       ├── mongodb.rs          # MongoDB PyO3 bindings
│   │       ├── http.rs             # HTTP PyO3 bindings
│   │       ├── test.rs             # Test framework PyO3 bindings
│   │       ├── validation.rs       # Security & type validation
│   │       └── config.rs           # Security configuration
│   │
│   ├── data-bridge-mongodb/        # Pure Rust MongoDB ORM
│   ├── data-bridge-postgres/       # PostgreSQL support
│   ├── data-bridge-http/           # HTTP client
│   ├── data-bridge-test/           # Benchmarking & testing framework
│   ├── data-bridge-common/         # Shared utilities
│   ├── data-bridge-kv/             # Key-Value store
│   ├── data-bridge-kv-server/      # KV server
│   ├── data-bridge-kv-client/      # KV client
│   ├── data-bridge-api/            # API framework
│   ├── data-bridge-tasks/          # Task queue
│   │
│   ├── data-bridge-sheet-core/     # Spreadsheet core (cells, grid)
│   │   └── src/
│   │       ├── lib.rs              # Module registration
│   │       ├── cell.rs             # Cell data structures
│   │       ├── sheet.rs            # Sheet management
│   │       ├── grid.rs             # Grid operations
│   │       ├── format.rs           # Cell formatting
│   │       └── ...
│   │
│   ├── data-bridge-sheet-db/       # Custom database
│   │   └── src/
│   │       ├── lib.rs              # Module registration
│   │       ├── storage.rs          # Storage layer (Morton encoding)
│   │       ├── query.rs            # Query layer (range, spatial)
│   │       ├── crdt.rs             # CRDT operations
│   │       └── wal.rs              # Write-ahead log
│   │
│   ├── data-bridge-sheet-formula/  # Formula parser & evaluator
│   │   └── src/
│   │       ├── lib.rs              # Module registration
│   │       ├── parser.rs           # Formula parser (nom-based)
│   │       ├── evaluator.rs        # Formula evaluator
│   │       ├── functions/          # Built-in functions (24+)
│   │       └── dependency.rs       # Dependency tracking
│   │
│   ├── data-bridge-sheet-history/  # Undo/redo system
│   │   └── src/
│   │       ├── lib.rs              # Module registration
│   │       ├── command.rs          # Command pattern
│   │       └── manager.rs          # History manager
│   │
│   ├── data-bridge-sheet-server/   # Collaboration server
│   │   └── src/
│   │       ├── main.rs             # Server entry point
│   │       ├── api/                # REST API (Axum)
│   │       ├── websocket/          # WebSocket handlers
│   │       ├── crdt/               # CRDT sync (yrs)
│   │       └── db/                 # PostgreSQL models
│   │
│   └── data-bridge-sheet-wasm/     # WebAssembly bindings
│       └── src/
│           ├── lib.rs              # WASM entry point
│           ├── bindings.rs         # JavaScript bindings
│           └── bridge.rs           # Rust ↔ JS bridge
│
├── python/data_bridge/             # Python API layer
│   ├── __init__.py                 # Public API exports
│   ├── _engine.py                  # Rust backend bridge
│   ├── document.py                 # Document base class
│   ├── fields.py                   # FieldProxy & QueryExpr
│   ├── query.py                    # QueryBuilder
│   ├── bulk.py                     # Bulk operations
│   ├── types.py                    # PydanticObjectId, Indexed
│   ├── validation.py               # Type & constraint validation
│   ├── state.py                    # Copy-on-Write state tracker
│   └── connection.py               # Connection management
│
├── frontend/                       # Spreadsheet frontend
│   ├── src/                        # TypeScript source
│   │   ├── core/                   # API and state
│   │   ├── canvas/                 # Canvas rendering
│   │   ├── ui/                     # UI components
│   │   ├── collab/                 # Collaboration client
│   │   └── worker/                 # Web Worker
│   ├── pkg/                        # Built WASM package
│   ├── examples/                   # Example apps
│   ├── package.json                # npm configuration
│   └── vite.config.ts              # Vite configuration
│
├── tests/                          # Python tests (68 files, 313+ tests)
│   ├── conftest.py                 # Pytest fixtures
│   ├── unit/                       # Unit tests (no MongoDB)
│   ├── integration/                # Integration tests (MongoDB required)
│   └── mongo/benchmarks/           # Performance benchmarks
│
├── benchmarks/                     # Performance benchmarks
│   └── bench_comparison.py         # Beanie/PyMongo comparison
│
└── docs/                           # Documentation
    ├── SHEET_README.md             # Spreadsheet documentation
    ├── SHEET_ARCHITECTURE.md       # Technical architecture
    ├── SHEET_CONTRIBUTING.md       # Contribution guidelines
    └── sheet-specs/                # Specifications
</repository-structure>

<build-commands>
# Rust build (MongoDB ORM)
maturin develop                      # Debug build
maturin develop --release            # Release build (optimized)

# Rust tests
cargo test                           # All Rust tests
cargo test -p data-bridge-mongodb    # MongoDB crate only
cargo test -p data-bridge-sheet-core # Sheet core only
cargo clippy                         # Lint check

# Python tests (use data-bridge-test, NOT pytest)
uv run python tests/unit/test_*.py                        # Unit tests (no MongoDB)
uv run python tests/integration/test_*.py                 # Integration tests (MongoDB required)
uv run python tests/api/test_*.py                         # API tests

# Performance benchmarks (use data-bridge-test, NOT pytest-benchmark)
uv run python benchmarks/bench_comparison.py --rounds 5 --warmup 2              # MongoDB ORM benchmarks
uv run python tests/api/benchmarks/bench_comparison_rust.py --rounds 5 --warmup 2  # API benchmarks

# pytest (ONLY for comparing pytest-benchmark vs data-bridge-test performance)
uv run pytest tests/ -v --benchmark-only                  # Benchmark comparison only

# Frontend/WASM build (Spreadsheet)
just build-wasm                      # Build WASM module
just build-frontend                  # Build frontend + WASM
just build-frontend-lib              # Build as library
just dev-frontend                    # Start dev server

# Frontend tests
just test-frontend                   # All frontend tests
just test-frontend-unit              # Unit tests only
just test-frontend-integration       # Integration tests
just test-frontend-e2e               # E2E tests

# Spreadsheet server
just server                          # Start collaboration server
just db-up                           # Start PostgreSQL (for workbooks)

# Security audit
cargo audit
</build-commands>

<feature-series>
Feature roadmap organization:

1xx Series: Type Validation System (COMPLETED)
  - Copy-on-Write state management ✅
  - Lazy validation ✅
  - Fast-path bulk operations ✅
  - Rust query execution ✅
  - Type schema extraction ✅
  - Basic type validation ✅
  - Complex type validation ✅
  - Constraint validation ✅

2xx Series: Performance Optimization (IN PROGRESS)
  - Bulk operation improvements
  - GIL release optimization
  - Zero-copy deserialization

9xx Series: Infrastructure (COMPLETED)
  - HTTP client ✅
  - Test framework ✅

Future Series:
  - 3xx: Relations & References
  - 4xx: Query Builder Enhancements
  - 5xx: Embedded Documents
  - 6xx: Document Inheritance
  - 7xx: Schema Migrations
  - 8xx: Tooling & Developer Experience
</feature-series>

<performance-targets>
data-bridge performance goals (vs Beanie):

Inserts (1000 docs):
  - Beanie: 57.53ms (17,381 ops/sec)
  - Target: <20ms (≥2.8x faster)
  - Current: 17.76ms (56,309 ops/sec) ✅ 3.2x faster

Finds (1000 docs):
  - Beanie: 8.58ms (116,517 ops/sec)
  - Target: <7ms (≥1.2x faster)
  - Current: 6.32ms (158,247 ops/sec) ✅ 1.4x faster

Memory:
  - Python heap: Minimal (BSON processed in Rust)
  - GIL contention: None (released during BSON ops)
  - State tracking: 50% reduction (Copy-on-Write)

Bulk Operations:
  - Parallel threshold: ≥50 documents (Rayon)
  - Vector pre-allocation: Always
  - Two-phase conversion: Extract → Convert
</performance-targets>

<architecture-principles>
1. Python Do Less, Rust Do More (Validation)
   - Python: Type hints ONLY (for IDE/editor, NOT runtime validation)
   - Rust: ALL runtime validation (type checking, security, BSON conversion)
   - Same developer experience as Pydantic, but 10x faster
   - Errors at save() instead of __init__ (acceptable for async ORM)
   - Zero Python validation overhead

2. Zero Python Byte Handling
   - All BSON serialization/deserialization in Rust
   - Python receives only typed, validated objects
   - Minimizes Python heap pressure

3. GIL Release Strategy
   - Release GIL during BSON conversion
   - Release GIL during network I/O
   - Hold GIL only for Python object construction

4. Parallel Processing
   - Rayon for batches ≥50 documents
   - Two-phase pattern: extract Python objects → convert in parallel
   - Vector pre-allocation to avoid reallocation

5. Copy-on-Write State Management
   - Field-level change tracking (not full deepcopy)
   - 10x faster than deepcopy
   - 50% memory reduction

6. Validation Architecture
   - Type validation during BSON conversion (conversion.rs)
   - Security validation before MongoDB operations (validation.rs)
   - Structure validation (document size, nesting depth)
   - Validation CANNOT be skipped (even with fast-path)

7. Security First
   - Collection name validation (no system collections)
   - Field name validation (no $ operators)
   - NoSQL injection prevention
   - Context-aware ObjectId parsing
   - Validation at native code boundary
</architecture-principles>

</grounding>

---

<negative-constraints>

<rule severity="NEVER">Skip performance verification → Regressions undetected → Run benchmarks before PR</rule>
<rule severity="NEVER">Hold GIL during BSON conversion → Blocks Python threads → Use py.allow_threads()</rule>
<rule severity="NEVER">Process Python bytes in Python → Defeats purpose → All BSON in Rust</rule>
<rule severity="NEVER">Skip security validation → NoSQL injection risk → Validate collection/field names</rule>
<rule severity="NEVER">Use deepcopy for state tracking → 10x slower → Use Copy-on-Write state manager</rule>
<rule severity="NEVER">Commit without running tests → Broken code enters repo → Run cargo test + pytest first</rule>
<rule severity="NEVER">Skip clippy → Lints accumulate → Run cargo clippy before commit</rule>
<rule severity="NEVER">Add unwrap() in production code → Panics crash Python → Use proper error handling</rule>
<rule severity="NEVER">Break Beanie compatibility → Users can't migrate → Maintain compatible API</rule>
<rule severity="NEVER">Bypass type validation → Security risk → Validate at PyO3 boundary</rule>
<rule severity="NEVER">Read code in main thread → Defeats orchestrator pattern → Delegate to explorer agent</rule>
<rule severity="NEVER">Write code in main thread → Defeats orchestrator pattern → Delegate to implementer agent</rule>
<rule severity="NEVER">Use Read/Edit/Write tools directly → Main thread is PM only → Spawn appropriate agent</rule>

<bad-example name="Skip performance verification">
User: "Optimize bulk insert"
Assistant: *Implements optimization but doesn't run benchmarks*
❌ WRONG: No verification that optimization actually improved performance
</bad-example>

<bad-example name="Hold GIL during conversion">
```rust
// ❌ WRONG: GIL held during expensive operation
fn insert_many(py: Python, docs: Vec<PyObject>) -> PyResult<()> {
    let bson_docs: Vec<Document> = docs.iter()
        .map(|doc| python_to_bson(py, doc))  // GIL held here
        .collect();
}

// ✅ CORRECT: Release GIL during conversion
fn insert_many(py: Python, docs: Vec<PyObject>) -> PyResult<()> {
    let py_docs = extract_python_data(py, docs)?;
    let bson_docs = py.allow_threads(|| {  // GIL released
        convert_to_bson(py_docs)
    });
}
```
</bad-example>

<bad-example name="Process bytes in Python">
```python
# ❌ WRONG: BSON bytes in Python heap
def find_many(query):
    bson_bytes = mongo_client.query(query)
    documents = [bson.decode(b) for b in bson_bytes]  # Python heap pressure
    return [Document(**d) for d in documents]

# ✅ CORRECT: All BSON processing in Rust
def find_many(query):
    return _engine.find_many(query)  # Returns typed Python objects directly
```
</bad-example>

<bad-example name="Read code directly in main thread">
User: "What validation patterns exist?"
Assistant: *Uses Read tool to read validation.rs directly*
❌ WRONG: Main thread should not read code directly

✅ CORRECT:
Assistant: *Spawns explorer agent* → "Find and summarize validation patterns in crates/data-bridge/src/validation.rs"
</bad-example>

<bad-example name="Write code directly in main thread">
User: "Add email validation"
Assistant: *Uses Edit tool to modify validation.rs directly*
❌ WRONG: Main thread should not write code directly

✅ CORRECT:
Assistant: *Spawns implementer agent* → "Add email validation to validation.rs following existing patterns"
</bad-example>

</negative-constraints>

---

<delimiters>
Use these markers in workflow updates:

<marker name="IMPLEMENTING">
Task: Add email regex validation
File: crates/data-bridge/src/validation.rs
Layer: Rust validation engine
</marker>

<marker name="TESTING">
Rust: cargo test test_email_validation
Python: pytest tests/unit/test_constraints.py::test_email_field
Integration: pytest tests/integration/test_validation.py
</marker>

<marker name="BUILD CHECK">
cargo build: PASS
cargo test: PASS (315 tests)
cargo clippy: PASS
pytest: PASS (313 tests)
</marker>

<marker name="BENCHMARK CHECK">
Benchmark: benchmarks/bench_validation.py
Overhead: <5% ✅
Target met: YES
</marker>

<marker name="READY FOR PR">
All tasks complete
Tests pass (Rust + Python)
Performance verified
Beanie compatibility maintained
</marker>
</delimiters>

---

<crate-todos>
## Crate-Level Todo Tracking

Each crate maintains its own `todos.md` file for tracking implementation progress.

**Location**: `crates/{crate-name}/todos.md`

**Update Frequency**:
- Update todos.md after EVERY significant change
- Mark completed items with ✅
- Add new discovered tasks immediately

**Format**:
```markdown
# {Crate Name} - Implementation Todos

## In Progress
- [ ] Current task description

## Pending
- [ ] Future task

## Completed
- [x] ✅ Completed task (YYYY-MM-DD)
```

**Rule**: Before committing, ensure todos.md reflects current state.
</crate-todos>

---

<output-structure>
After each work session, report in this format:

<report>
  <feature>{NNN}-{name}</feature>
  <component>{MongoDB|HTTP|Test|Python|PyO3|Common}</component>

  <tasks-completed>
    <task status="done">Task 1: Add Rust validation function</task>
    <task status="done">Task 2: Add PyO3 binding</task>
    <task status="next">Task 3: Integrate with Document.save()</task>
  </tasks-completed>

  <tests>
    <test name="test_email_validation" status="PASS" layer="Rust"/>
    <test name="test_email_field_integration" status="PASS" layer="Python"/>
    <test name="test_invalid_email_rejected" status="PASS" layer="Integration"/>
  </tests>

  <build-status>
    <check name="cargo build" status="PASS"/>
    <check name="cargo test" status="PASS" note="315 tests"/>
    <check name="cargo clippy" status="PASS"/>
    <check name="data-bridge-test" status="PASS" note="313 tests"/>
  </build-status>

  <performance>
    <benchmark name="validation_overhead" result="3.2ms" target="<5ms" status="PASS"/>
    <comparison baseline="without_validation" improvement="4.1% overhead" acceptable="YES"/>
  </performance>

  <next-steps>
    <step>Implement Task 4: Add Python API wrapper</step>
    <step>Implement Task 5: Update documentation</step>
    <step>Run full benchmark suite</step>
    <step>Create PR</step>
  </next-steps>
</report>
</output-structure>

---

<self-correction>
Before committing or creating PR, verify ALL items:

<checklist name="Code Quality - Rust">
  <item>cargo build passes?</item>
  <item>cargo test passes?</item>
  <item>cargo clippy clean?</item>
  <item>No unwrap() in production code?</item>
  <item>Proper error handling with thiserror?</item>
  <item>GIL released during expensive operations?</item>
</checklist>

<checklist name="Code Quality - Python">
  <item>Type hints complete?</item>
  <item>Beanie API compatibility maintained?</item>
  <item>Documentation strings added?</item>
  <item>No direct BSON manipulation in Python?</item>
</checklist>

<checklist name="Testing">
  <item>Rust unit tests written (cargo test)?</item>
  <item>Python unit tests written (data-bridge-test)?</item>
  <item>Integration tests cover CRUD lifecycle?</item>
  <item>All tests pass (Rust + Python via data-bridge-test)?</item>
  <item>Edge cases covered?</item>
  <item>Unit tests work without MongoDB?</item>
</checklist>

<checklist name="Performance">
  <item>Benchmarks run and results recorded?</item>
  <item>Performance targets met?</item>
  <item>No regression vs previous version?</item>
  <item>GIL release verified?</item>
  <item>Memory usage checked?</item>
</checklist>

<checklist name="Security">
  <item>Input validation at PyO3 boundary?</item>
  <item>Collection name validation?</item>
  <item>Field name validation (no $ operators)?</item>
  <item>ObjectId parsing safe?</item>
  <item>No SQL/NoSQL injection vectors?</item>
</checklist>

<checklist name="Commit">
  <item>Commit message format: feat(NNN): description?</item>
  <item>Changes are focused (not mixed features)?</item>
  <item>PR size reasonable (&lt;500 lines ideal)?</item>
  <item>Documentation updated?</item>
</checklist>

If ANY item is NO, fix it before proceeding.
</self-correction>

---

<quick-reference>
DEVELOPMENT WORKFLOW:
  1. Understand requirements and create plan
  2. Implement feature/fix
  3. Run tests (cargo test + data-bridge-test)
  4. Run benchmarks (if performance-related)
  5. Create commit and PR

BUILD CYCLE:
  maturin develop                    # Build Python extension
  cargo test                         # Rust tests
  uv run python tests/unit/test_*.py # Python tests (data-bridge-test)
  cargo clippy                       # Lint check

PERFORMANCE CHECK:
  uv run python benchmarks/bench_comparison.py --rounds 5 --warmup 2

COMMIT FORMAT:
  feat(NNN): description
  fix(NNN): description
  test(NNN): description
  perf(NNN): description

TEST MODES:
  Unit: uv run python tests/unit/test_*.py
  Integration: uv run python tests/integration/test_*.py
  API: uv run python tests/api/test_*.py
  Benchmarks: uv run python benchmarks/bench_*.py --rounds 5 --warmup 2
</quick-reference>

---

<technologies>
  <tech>Rust 1.70+ (edition 2021)</tech>
  <tech>PyO3 0.24+ (Python bindings, stable ABI)</tech>
  <tech>Maturin 1.x (build system)</tech>
  <tech>Python 3.12+</tech>
  <tech>MongoDB 3.1 (Rust driver)</tech>
  <tech>BSON 2.13</tech>
  <tech>Tokio 1.40 (async runtime)</tech>
  <tech>Rayon 1.10 (parallel processing)</tech>
  <tech>Reqwest 0.12 (HTTP client)</tech>
  <tech>pytest (testing)</tech>
  <tech>uv (Python package manager)</tech>
</technologies>

---

## Project Context

### What is data-bridge?

High-performance MongoDB ORM for Python with Rust backend achieving **1.4-5.4x faster performance than Beanie** through zero Python byte handling.

**Key Innovation**: All BSON serialization/deserialization happens in Rust, minimizing Python heap pressure.

```
Traditional: MongoDB → BSON bytes → Python bytes → PyMongo objects → Beanie models
data-bridge: MongoDB → BSON bytes → Rust structs → Python objects
```

### Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│         Python API Layer (Beanie-compatible)            │
│   document.py, fields.py, query.py, bulk.py            │
└──────────────────┬──────────────────────────────────────┘
                   │ PyO3 Bridge
┌──────────────────▼──────────────────────────────────────┐
│              Rust Engine Layer                          │
│  • data-bridge/src/mongodb.rs (3,162 lines)            │
│  • BSON Serialization/Deserialization                  │
│  • Type Validation (runtime)                           │
│  • Security Validation                                 │
│  • GIL Release for CPU-intensive ops                   │
│  • Parallel Processing (Rayon)                         │
└──────────────────┬──────────────────────────────────────┘
                   │ Rust MongoDB Driver
┌──────────────────▼──────────────────────────────────────┐
│         data-bridge-mongodb (Pure Rust ORM)             │
│  • connection.rs (6,252 lines) - Connection pooling    │
│  • document.rs (8,320 lines) - Document operations     │
│  • query.rs (4,315 lines) - Query builder              │
└─────────────────────────────────────────────────────────┘
```

### Crate Organization

**MongoDB ORM:**
- **data-bridge**: PyO3 bindings (cdylib) - Python entry point
- **data-bridge-mongodb**: Pure Rust MongoDB ORM - Core engine

**Infrastructure:**
- **data-bridge-http**: Pure Rust HTTP client - HTTP operations
- **data-bridge-postgres**: PostgreSQL support
- **data-bridge-test**: Rust test framework - Benchmarking & testing
- **data-bridge-common**: Shared utilities - Error types
- **data-bridge-kv**: Key-Value store
- **data-bridge-kv-server**: KV server
- **data-bridge-kv-client**: KV client
- **data-bridge-api**: API framework
- **data-bridge-tasks**: Task queue (NATS/Redis)

**Spreadsheet Engine:**
- **data-bridge-sheet-core**: Core data structures (cells, sheets, formatting)
- **data-bridge-sheet-db**: Custom database with Morton encoding
- **data-bridge-sheet-formula**: Formula parser and evaluator (24+ functions)
- **data-bridge-sheet-history**: Undo/redo command system
- **data-bridge-sheet-server**: Collaboration server (Axum + Yjs)
- **data-bridge-sheet-wasm**: WebAssembly bindings

### Current Status

- **Version**: 0.1.0 (Alpha)
- **Tests**: 313+ passing (Python), 85% coverage
- **Performance**: 1.4-5.4x faster than Beanie
- **API**: Beanie-compatible (drop-in replacement)
- **Security**: Fixed RUSTSEC-2025-0020 (PyO3 0.24+)

### Key Principles

1. **Zero Python Byte Handling**: All BSON in Rust
2. **GIL Release**: Parallel processing without contention
3. **Copy-on-Write State**: 10x faster change tracking
4. **Lazy Validation**: Defer until save()
5. **Security First**: Validate all inputs at PyO3 boundary
6. **Beanie Compatibility**: Easy migration path

---

## Active Technologies
- Rust 1.70+ with PyO3 0.24+, Tokio 1.40, MongoDB 3.1, Rayon 1.10
- Python 3.12+ with Maturin build system
- uv package manager for Python dependencies
- pytest for Python testing, cargo test for Rust testing
- Rust 1.70+ (edition 2021), Python 3.12+ (201-gil-free-bson-conversion)
- MongoDB 4.0+ (primary data store) (201-gil-free-bson-conversion)

## Recent Changes
- Series 1xx (Type Validation): All completed ✅
- Series 9xx (Infrastructure): HTTP client and test framework ✅
- Series 2xx (Performance): In progress
