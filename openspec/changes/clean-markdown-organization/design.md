# Design: Clean Markdown Organization

## Change Type
**Infrastructure/Documentation** - This change reorganizes documentation files without modifying specifications or runtime code.

## Why No Spec Deltas?
This is a **pure documentation reorganization** that:
- Moves existing documentation files to standardized locations
- Creates new directory structure for better organization
- Does NOT add, modify, or remove any functional requirements
- Does NOT change any OpenSpec specifications

## Documentation Hierarchy

### Target Structure
```
data-bridge/
├── openspec/
│   ├── specs/              # Formal specifications (UNCHANGED)
│   │   └── spreadsheet-engine/
│   │       ├── spec.md     # Existing spec (UNCHANGED)
│   │       └── design-docs/  # NEW: Detailed design docs moved here
│   └── changes/            # Change proposals
│
├── dev-docs/               # Technical documentation
│   ├── 00-overview/        # Project overview
│   ├── 10-mongodb/         # MongoDB ORM docs
│   ├── 20-http-client/     # HTTP client docs
│   ├── 30-test-runner/     # Test framework docs
│   ├── 40-postgres/        # Postgres ORM docs
│   ├── 60-kv-store/        # KV store docs
│   ├── 70-ops/             # NEW: Operations (observability, tracing)
│   ├── 80-api-server/      # API server docs
│   └── 90-shared-internals/
│
├── docs/
│   ├── archive/            # Legacy summaries and migration reports
│   ├── postgres/           # User-facing Postgres guides
│   ├── en/                 # English user guides
│   └── zh-tw/              # Traditional Chinese user guides
│
├── benchmarks/
│   ├── reports/            # NEW: Consolidated benchmark reports
│   ├── framework_comparison/
│   └── pyloop/
│
└── examples/               # All example scripts
```

## Key Principles

1. **OpenSpec Specs Are Canonical**
   - `openspec/specs/**/*.md` contains ONLY formal specifications
   - Design documents moved to `design-docs/` subdirectories

2. **Dev-Docs for Technical Details**
   - Architecture, components, data flows, implementation details
   - Numbered directories for clear hierarchy (10-mongodb, 20-http-client, etc.)

3. **Docs for User-Facing Content**
   - Guides, quickstarts, API references
   - Separate `archive/` for legacy reports

4. **Centralized Benchmarks**
   - All benchmark reports in `benchmarks/reports/`
   - Avoids scattering reports across root and subdirectories

5. **Examples Together**
   - All `*_example.py` files in `examples/`
   - Easier discovery and reuse

## File Movements

### Category 1: Archive Legacy Summaries
**Reason**: Root-level summaries clutter the project and are historical artifacts.

**Action**: Move to `docs/archive/`
- `BATCH_CONVERSION_SUMMARY.md`
- `CONVERSION_REPORT.md`
- `CRUD_API_REFACTOR_SUMMARY.md`
- `PHASE5_IMPLEMENTATION_COMPLETE.md`
- `PYLOOP_*.md` (11 files)
- `verify_phase5.py`

### Category 2: Consolidate Benchmarks
**Reason**: Benchmark reports scattered across multiple directories.

**Action**: Centralize in `benchmarks/reports/`
- `docs/PYLOOP_BENCHMARKS.md` → `benchmarks/reports/pyloop_benchmarks.md`
- `benchmarks/API_BENCHMARK_GAP_ANALYSIS.md` → `benchmarks/reports/api_gap_analysis.md`

### Category 3: Move Sheet Specs
**Reason**: `docs/sheet-specs/` contains detailed design documents that should live with the formal spec.

**Action**: Move to `openspec/specs/spreadsheet-engine/design-docs/`
- `docs/sheet-specs/*.md` (16 files) → `openspec/specs/spreadsheet-engine/design-docs/`

**Note**: The formal `spec.md` remains at `openspec/specs/spreadsheet-engine/spec.md` (unchanged).

### Category 4: Consolidate Dev Docs
**Reason**: API, deployment, and testing docs belong in structured `dev-docs/`.

**Action**: Organize into `dev-docs/`
- `deploy/OBSERVABILITY.md` → `dev-docs/70-ops/observability.md`
- `deploy/TESTING.md` → `dev-docs/70-ops/tracing_verification.md`
- `docs/TESTING.md` → `dev-docs/00-overview/testing_guide.md`
- `docs/api/sse.md` → `dev-docs/80-api-server/sse.md`
- `docs/TEST_SERVER_PYTHON_APP.md` → `dev-docs/80-api-server/test_server_impl.md`
- `docs/PYLOOP_CRUD.md` → `dev-docs/80-api-server/pyloop_crud.md`
- `docs/MIGRATION_*.md` → `docs/archive/`

### Category 5: Consolidate Examples
**Reason**: Root-level example files make the project root cluttered.

**Action**: Move to `examples/`
- All `*_example.py` files from root → `examples/`

## Implementation Strategy

1. **Create New Directories First**
   - `docs/archive/`
   - `benchmarks/reports/`
   - `openspec/specs/spreadsheet-engine/design-docs/`
   - `dev-docs/70-ops/`

2. **Move Files (Use `git mv` for history preservation)**
   - Archive legacy summaries
   - Consolidate benchmarks
   - Move sheet specs
   - Organize dev docs
   - Consolidate examples

3. **Update References**
   - Search for broken links in:
     - `README.md`
     - `openspec/project.md`
     - Moved files (especially `sse.md`)
     - `dev-docs/*/index.md` files

4. **Clean Up Empty Directories**
   - `deploy/` (if empty)
   - `docs/sheet-specs/`
   - `docs/api/`

## Risk Assessment

**Risk**: Broken links after moving files.

**Mitigation**:
- Task 6.2 explicitly lists all files to check
- Use `grep -r "docs/sheet-specs" .` to find references
- Use `grep -r "deploy/OBSERVABILITY" .` to find references

**Impact**: Low
- No code changes
- Documentation only
- Easy to revert with `git mv`

## Success Criteria

1. ✅ All legacy summaries in `docs/archive/`
2. ✅ All benchmark reports in `benchmarks/reports/`
3. ✅ Sheet specs in `openspec/specs/spreadsheet-engine/design-docs/`
4. ✅ Operations docs in `dev-docs/70-ops/`
5. ✅ API docs in `dev-docs/80-api-server/`
6. ✅ All examples in `examples/`
7. ✅ No broken links in `README.md` or `openspec/project.md`
8. ✅ Empty directories removed
