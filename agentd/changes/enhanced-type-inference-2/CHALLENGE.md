# Challenge Report: enhanced-type-inference-2

## Summary
Solid direction but requires revision: MCP tool naming is inconsistent across docs, and environment detection is underspecified for poetry/pipenv so site-packages discovery is not feasible as written.

## Internal Consistency Issues
### Issue: MCP tool names and coverage mismatch
- **Severity**: High
- **Category**: Consistency
- **Description**: The proposal introduces `argus_configure_project`, while the spec defines `argus_get_config` and `argus_configure_venv`. Tasks implement `argus_get_config` and omit `argus_configure_venv`, so the tool surface is not aligned across documents.
- **Location**: `agentd/changes/enhanced-type-inference-2/proposal.md`, `agentd/changes/enhanced-type-inference-2/specs/mcp-tools.md`, `agentd/changes/enhanced-type-inference-2/tasks.md`
- **Recommendation**: Choose the canonical tool set (names + behaviors), update the spec, proposal, and tasks to match, and add any missing tasks.

### Issue: Poetry/Pipenv detection does not yield usable venv paths
- **Severity**: High
- **Category**: Completeness
- **Description**: R2 says presence of `poetry.lock`/`Pipfile` implies a managed env, but no mechanism is specified to resolve the actual venv path. This blocks R4 site-packages discovery and the acceptance criteria for using detected envs.
- **Location**: `agentd/changes/enhanced-type-inference-2/specs/python-env.md`
- **Recommendation**: Specify how to resolve env paths (e.g., standard location conventions, optional tool invocation, or explicit user config requirement), and clarify fallback behavior when resolution fails.

## Code Alignment Issues
### Issue: MCP tool implementation path is incorrect
- **Severity**: Medium
- **Category**: Conflict
- **Description**: Tasks target `crates/argus-mcp/src/tools.rs`, but the repo uses `crates/argus/src/mcp/tools.rs` and no `argus-mcp` crate exists.
- **Location**: `agentd/changes/enhanced-type-inference-2/tasks.md`
- **Recommendation**: Update tasks to the correct path and confirm whether MCP tools live in the `argus` crate.

### Issue: Import resolution design ignores existing stub loader/typeshed flow
- **Severity**: Medium
- **Category**: Conflict
- **Description**: The spec assumes `ImportResolver` locates stdlib modules via search paths, but current code routes stdlib types through `StubLoader`/`typeshed`. Without an explicit integration plan, the new resolver could bypass or duplicate existing stub behavior.
- **Location**: `agentd/changes/enhanced-type-inference-2/specs/import-resolution.md`, `crates/argus/src/types/stubs.rs`, `crates/argus/src/types/infer.rs`
- **Note**: No refactor intent stated in `agentd/changes/enhanced-type-inference-2/proposal.md`.
- **Recommendation**: Define how resolver and stub loader interact (e.g., resolver consults stubs/typeshed before file system search) and reflect that in tasks.

## Quality Suggestions
### Issue: Circular import handling lacks a test case
- **Severity**: Low
- **Category**: Completeness
- **Description**: The import-resolution spec requires safe circular import handling, but tests only mention generic cross-file import resolution.
- **Recommendation**: Add a focused test for the circular import scenario in the import resolver test plan.

### Issue: Path precedence and normalization are unspecified
- **Severity**: Low
- **Category**: Completeness
- **Description**: The config priority order is defined, but there is no guidance on how relative paths are normalized or how conflicting modules across multiple search paths are resolved.
- **Recommendation**: Document path normalization rules (project-root relative vs absolute) and the exact search order used by resolution.

## Verdict
- [ ] APPROVED - Ready for implementation
- [x] NEEDS_REVISION - Address issues above (specify which severity levels)
- [ ] REJECTED - Fundamental problems, needs rethinking

**Next Steps**: Resolve HIGH severity consistency/completeness issues, then re-run challenge review.
