## 1. Archive Legacy Summaries
- [ ] 1.1 Create directory `docs/archive/`.
- [ ] 1.2 Move `BATCH_CONVERSION_SUMMARY.md` to `docs/archive/`.
- [ ] 1.3 Move `CONVERSION_REPORT.md` to `docs/archive/`.
- [ ] 1.4 Move `CRUD_API_REFACTOR_SUMMARY.md` to `docs/archive/`.
- [ ] 1.5 Move `PHASE5_IMPLEMENTATION_COMPLETE.md` to `docs/archive/`.
- [ ] 1.6 Move `PYLOOP_*.md` (excluding valid docs if any, verify content) to `docs/archive/`.
    - `PYLOOP_COMPILATION_FIXES.md` -> `docs/archive/`
    - `PYLOOP_PHASE*.md` -> `docs/archive/`
- [ ] 1.7 Move `verify_phase5.py` to `docs/archive/` (seems like a one-off script).

## 2. Consolidate Benchmarks
- [ ] 2.1 Create directory `benchmarks/reports/`.
- [ ] 2.2 Move `docs/PYLOOP_BENCHMARKS.md` to `benchmarks/reports/pyloop_benchmarks.md`.
- [ ] 2.3 Move `benchmarks/API_BENCHMARK_GAP_ANALYSIS.md` to `benchmarks/reports/api_gap_analysis.md`.

## 3. Move Sheet Specs
- [ ] 3.1 Create directory `openspec/specs/spreadsheet-engine/design-docs/`.
- [ ] 3.2 Move `docs/sheet-specs/*.md` to `openspec/specs/spreadsheet-engine/design-docs/`.
- [ ] 3.3 Remove empty `docs/sheet-specs/` directory.

## 4. Consolidate Dev Docs
- [ ] 4.1 Create directory `dev-docs/70-ops/`.
- [ ] 4.2 Move `deploy/OBSERVABILITY.md` to `dev-docs/70-ops/observability.md`.
- [ ] 4.3 Move `deploy/TESTING.md` to `dev-docs/70-ops/tracing_verification.md`.
- [ ] 4.4 Move `docs/TESTING.md` to `dev-docs/00-overview/testing_guide.md`.
- [ ] 4.5 Move `docs/api/*.md` (if any) or content to `dev-docs/80-api-server/`.
    - *Note*: `docs/api/` is currently empty or contains files? (Need to check during execution). If empty, remove it.
- [ ] 4.6 Move `docs/MIGRATION_*.md` to `docs/archive/` (Legacy migration logs).
- [ ] 4.7 Move `docs/TEST_SERVER_PYTHON_APP.md` to `dev-docs/80-api-server/test_server_impl.md`.
- [ ] 4.8 Move `docs/PYLOOP_CRUD.md` to `dev-docs/80-api-server/pyloop_crud.md`.

## 5. Consolidate Examples
- [ ] 5.1 Move `*_example.py` from root to `examples/`.
    - `api_models_example.py`
    - `api_sse_example.py`
    - `api_type_extraction_demo.py`
    - `background_tasks_example.py`
    - `fastapi_otel_example.py`
    - `form_upload_example.py`
    - `health_example.py`
    - `http_client_example.py`
    - `inheritance_example.py`
    - `openapi_demo.py`
    - `postgres_*.py`
    - `pyloop_*.py`
    - `run_example.py`
    - `shutdown_example.py`
    - `telemetry_example.py`
    - `test_fastapi_otel.py`
    - `test_server_example.py`

## 6. Cleanup
- [ ] 6.1 Remove empty directories if any (`deploy/`, `docs/sheet-specs/`, `docs/api/`).
- [ ] 6.2 Verify and update links in:
    - `README.md`
    - `openspec/project.md`
    - Moved files (e.g., `sse.md` pointing to examples)
    - `dev-docs/00-overview/index.md` (if it exists)
