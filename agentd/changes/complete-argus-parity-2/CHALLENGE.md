# Challenge Report: complete-argus-parity-2

## Summary
Strong coverage of desired features, but several gaps in task coverage and integration details will block implementation or leave key config/indexing unused. Needs revisions before implementation.

## Internal Consistency Issues
[Check if proposal files are consistent with each other]
[Examples: Does proposal.md match tasks.md? Do Mermaid diagrams in specs/ match descriptions? Do task spec refs exist?]
[These are HIGH priority - must fix before implementation]

### Issue: Missing module wiring for new LSP/semantic files
- **Severity**: High
- **Category**: Completeness
- **Description**: Tasks add new Rust source files (`lsp/workspace.rs`, `lsp/code_actions.rs`, `lsp/tests.rs`, `semantic/tests.rs`) but no tasks update module declarations to compile or register tests. Without `mod` declarations (and `#[cfg(test)]` hooks), Rust will not build or tests will not run.
- **Location**: `agentd/changes/complete-argus-parity-2/tasks.md` (2.5, 3.2, 4.2, 4.4); missing updates to `crates/argus/src/lsp/mod.rs` and `crates/argus/src/semantic/mod.rs`
- **Recommendation**: Add explicit tasks to update `crates/argus/src/lsp/mod.rs` and `crates/argus/src/semantic/mod.rs` (and any test module wiring) when creating new files.

## Code Alignment Issues
[Check if proposal aligns with existing codebase]
[Examples: Do file paths exist? Do APIs exist? Does architecture match patterns?]
[IMPORTANT: If proposal mentions refactoring or BREAKING changes, deviations are EXPECTED]

### Issue: Watch mode placement conflicts with current architecture
- **Severity**: Medium
- **Category**: Conflict
- **Description**: Watch mode tasks target `crates/argus/src/lib.rs`, but actual project analysis lives in `crates/argus/src/types/project.rs` and LSP logic in `crates/argus/src/lsp/server.rs`. Adding watcher logic to `lib.rs` risks unused or unreachable code paths unless a CLI entry point uses it.
- **Location**: `agentd/changes/complete-argus-parity-2/tasks.md` (2.4, 2.6b)
- **Note**: No refactor intent noted in `agentd/changes/complete-argus-parity-2/proposal.md`
- **Recommendation**: Decide the runtime owner (ProjectAnalyzer vs LSP server) and move tasks to the appropriate module, or add a documented entry point that calls the new watch-mode code.

### Issue: Typeshed configuration may not be consumed by active analyzers
- **Severity**: Medium
- **Category**: Conflict
- **Description**: Tasks add typeshed settings to `crates/argus/src/core/config.rs` and `crates/argus/src/types/config.rs`, but project analysis uses `ProjectConfig` in `crates/argus/src/types/project.rs` and LSP uses `LintConfig`. Without wiring, the new settings will not reach `StubLoader`.
- **Location**: `agentd/changes/complete-argus-parity-2/tasks.md` (1.3, 1.5); existing code in `crates/argus/src/types/project.rs` and `crates/argus/src/lsp/server.rs`
- **Note**: No refactor intent noted in `agentd/changes/complete-argus-parity-2/proposal.md`
- **Recommendation**: Identify the configuration source of truth and add tasks to thread those settings into `ProjectAnalyzer` and `ArgusServer` (or deprecate unused config paths).

### Issue: Reference indexing missing for rename/references
- **Severity**: Medium
- **Category**: Conflict
- **Description**: Current `SymbolTable` only stores symbol definitions, not usage sites. Implementing `textDocument/references` and cross-file rename needs reference collection and resolution, which is not addressed in tasks or existing semantic APIs.
- **Location**: `crates/argus/src/semantic/symbols.rs`; `agentd/changes/complete-argus-parity-2/tasks.md` (2.5, 3.1)
- **Note**: No refactor intent noted in `agentd/changes/complete-argus-parity-2/proposal.md`
- **Recommendation**: Add tasks to extend semantic analysis to collect references (or define a separate indexing pass) and clarify how references are resolved across modules.

## Quality Suggestions
[Missing tests, error handling, edge cases, documentation]
[These are LOW priority - nice to have improvements]

### Issue: Clarify cache metadata and HTTP validation for typeshed
- **Severity**: Low
- **Category**: Completeness
- **Description**: The typeshed spec includes `etag` and `last_updated` in the cache data model, but tasks do not mention conditional requests or validation; the downloader could re-fetch large content unnecessarily.
- **Recommendation**: Add a task note to use ETag/If-None-Match (or similar) and define timeouts/size limits for downloads.

### Issue: Define debounce defaults/config for watch mode
- **Severity**: Low
- **Category**: Other
- **Description**: Watch mode spec calls for debouncing but tasks do not specify a default interval or configuration option.
- **Recommendation**: Add a default debounce interval (e.g., 100ms as in the spec flow) and consider exposing it in config.

## Verdict
- [ ] APPROVED - Ready for implementation
- [x] NEEDS_REVISION - Address issues above (specify which severity levels)
- [ ] REJECTED - Fundamental problems, needs rethinking

**Next Steps**: Resolve HIGH issue (module wiring) and address MEDIUM alignment gaps (watch mode ownership, config wiring, reference indexing), then re-run challenge.
