# Code Review Report: enhanced-type-inference-2

**Iteration**: 0

## Summary
Implementation adds env detection, config structs, import indexing, and MCP tool schemas, but test execution failed and critical runtime gaps remain in MCP tool handling and environment fallback paths. Security audit reports one vulnerability and multiple unmaintained dependencies.

## Test Results
**Overall Status**: FAIL

### Test Summary
- Total tests: Not reported (build failed before completion)
- Passed: Not reported
- Failed: Not reported (build error)
- Skipped: Not reported
- Coverage: Not reported

### Failed Tests (if any)
- Build failed while linking `ouroboros-api` example `simple_server`: undefined symbol `__Py_DecRef` for arm64 (likely missing Python framework/link flags). See `/var/folders/0y/3c3b0y3x3dz2vh35mypkps100000gq/T/tmp.BTLh4vhZh9/test_output.txt`.

## Security Scan Results
**Status**: VULNERABILITIES

### cargo audit (Dependency Vulnerabilities)
- **HIGH**: `protobuf 2.28.0` has RUSTSEC-2024-0437 (uncontrolled recursion crash). Upgrade to `>= 3.7.2`.
- **WARNINGS**: Unmaintained crates flagged: `bincode 1.3.3` (RUSTSEC-2025-0141), `instant 0.1.13` (RUSTSEC-2024-0384), `proc-macro-error 1.0.4` (RUSTSEC-2024-0370), `rustls-pemfile 2.2.0` (RUSTSEC-2025-0134), and `ouroboros 0.1.0` marked unsound (RUSTSEC-2023-0042).

### semgrep (Code Pattern Scan)
- No issues reported (empty output file).

### Linter Security Rules
- Clippy reported extensive warnings across the workspace; none were specific to the new files in this change.

## Best Practices Issues
[HIGH priority - must fix]

### Issue: Windows `PYTHONPATH` delimiter not handled
- **Severity**: Medium
- **Category**: Compatibility
- **File**: crates/argus/src/types/env.rs
- **Description**: `detect_with_config` splits `PYTHONPATH` only on `:`, which fails on Windows where `;` is the separator.
- **Recommendation**: Split on `std::path::MAIN_SEPARATOR`-appropriate delimiter or use `std::env::split_paths`.

## Requirement Compliance Issues
[HIGH priority - must fix]

### Issue: MCP tools defined but not implemented
- **Severity**: High
- **Category**: Missing Feature
- **Requirement**: `specs/mcp-tools.md` (R1–R3) and acceptance criteria
- **Description**: New tools are only added to schemas in `crates/argus/src/mcp/tools.rs`. `crates/argus/src/mcp/server.rs` does not handle `argus_get_config`, `argus_set_python_paths`, `argus_configure_venv`, `argus_detect_environment`, or `argus_list_modules`, so calls return “Unknown tool”.
- **Recommendation**: Add handlers in `crates/argus/src/mcp/server.rs` (and daemon methods as needed) to implement config inspection, pyproject updates, env detection, and module listing.

### Issue: Missing system interpreter path fallback
- **Severity**: High
- **Category**: Missing Feature
- **Requirement**: `specs/python-env.md#r1-configuration-priority`
- **Description**: `detect_with_config` never adds system interpreter paths (step 4 in priority list). Only project root and venv site-packages are added.
- **Recommendation**: Add system interpreter/library paths (e.g., stdlib + site-packages of system Python) when no venv or config paths are available.

### Issue: Config parse errors are silently ignored
- **Severity**: High
- **Category**: Wrong Behavior
- **Requirement**: `specs/python-env.md#interfaces` (`load_argus_config` errors)
- **Description**: `ArgusConfig::from_pyproject` swallows TOML parse errors and falls back to defaults without surfacing an error.
- **Recommendation**: Return a `Result` with a dedicated error type and propagate parse errors to callers.

## Consistency Issues
[MEDIUM priority - should fix]

None noted in the new/modified files.

## Test Quality Issues
[MEDIUM priority - should fix]

### Issue: Missing tests for config parse errors and system path fallback
- **Severity**: Medium
- **Category**: Coverage
- **Description**: There are no tests asserting parse-error behavior for malformed `pyproject.toml`, or verifying system interpreter fallback paths.
- **Recommendation**: Add tests that cover malformed TOML error propagation and system path inclusion when no venv/config is present.

## Verdict
- [ ] APPROVED - Ready for merge (all tests pass, no HIGH issues)
- [x] NEEDS_CHANGES - Address issues above (specify which)
- [ ] MAJOR_ISSUES - Fundamental problems (failing tests or critical security)

**Next Steps**: Fix MCP tool handling, add system interpreter fallback, surface config parse errors, and resolve the linker failure so tests can complete.
