# Challenge Report: ob-pg-prod-ready-2

## Summary
Proposal has multiple spec/task mismatches and missing definitions that block implementation clarity. Key gaps are around `any_`/`has` semantics, error classification, and observability scope. Needs revision before implementation.

## Internal Consistency Issues
[Check if proposal files are consistent with each other]
[Examples: Does proposal.md match tasks.md? Do Mermaid diagrams in specs/ match descriptions? Do task spec refs exist?]
[These are HIGH priority - must fix before implementation]

### Issue: `any_`/`has` semantics conflict with proposal intent
- **Severity**: High
- **Category**: Consistency
- **Description**: Proposal calls out missing `any_`/`has` filters “required by the application layer,” which in the repo are relationship filters that require EXISTS subqueries. The spec instead defines `any_` as array containment and `has` as JSON key existence, a different feature set.
- **Location**: `agentd/changes/ob-pg-prod-ready-2/proposal.md`, `agentd/changes/ob-pg-prod-ready-2/specs/features.md`, `python/ouroboros/postgres/query_ext.py`
- **Recommendation**: Align `specs/features.md` with the intended app-layer semantics (EXISTS subqueries for relationships) or update the proposal/tasks to explicitly target array/JSON filters and rename APIs to avoid collisions.

### Issue: Error classification requires new error variants but none are specified
- **Severity**: High
- **Category**: Completeness
- **Description**: `specs/robustness.md` and tasks require mapping to `DataBridgeError` variants (Conflict, ForeignKey, Deadlock), but those variants do not exist and there is no spec or task covering additions to the shared error enum.
- **Location**: `agentd/changes/ob-pg-prod-ready-2/specs/robustness.md`, `agentd/changes/ob-pg-prod-ready-2/tasks.md`
- **Recommendation**: Add a spec section defining new `DataBridgeError` variants (or clarify reuse of existing ones) and include tasks for `crates/ouroboros-common/src/error.rs` plus any Python error mapping.

### Issue: Slow query logging has no spec requirements or acceptance criteria
- **Severity**: High
- **Category**: Completeness
- **Description**: Task 3.5 points to `specs/observability.md`, but that spec only defines tracing spans and error logging. There is no requirement/AC for slow query logging.
- **Location**: `agentd/changes/ob-pg-prod-ready-2/tasks.md`, `agentd/changes/ob-pg-prod-ready-2/specs/observability.md`
- **Recommendation**: Add an explicit R3/acceptance criteria for slow query logging (threshold config, log fields), or remove the task.

## Code Alignment Issues
[Check if proposal aligns with existing codebase]
[Examples: Do file paths exist? Do APIs exist? Does architecture match patterns?]
[IMPORTANT: If proposal mentions refactoring or BREAKING changes, deviations are EXPECTED]

### Issue: INSERT RETURNING fix targets the wrong module
- **Severity**: Medium
- **Category**: Conflict
- **Description**: Task 1.1 says to modify `crates/ouroboros-postgres/src/query/mod.rs`, but INSERT/RETURNING behavior for `execute()` lives in the PyO3 bridge (`crates/ouroboros/src/postgres/relations.rs`) and other execution paths are in `crates/ouroboros-postgres/src/row.rs`.
- **Location**: `agentd/changes/ob-pg-prod-ready-2/tasks.md`, `crates/ouroboros/src/postgres/relations.rs`, `crates/ouroboros-postgres/src/row.rs`
- **Note**: No refactor note in proposal.
- **Recommendation**: Update tasks to point to the actual execution layer(s) used by the failing integration tests.

### Issue: Advanced query task overlaps with existing join/subquery support
- **Severity**: Medium
- **Category**: Conflict
- **Description**: `specs/advanced_query.md` says joins and subqueries are missing, but join/subquery builders already exist in `select.rs`, `join.rs`, and `types.rs`. Only `defer()`/`only()` are clearly missing.
- **Location**: `agentd/changes/ob-pg-prod-ready-2/specs/advanced_query.md`, `crates/ouroboros-postgres/src/query/select.rs`, `crates/ouroboros-postgres/src/query/join.rs`
- **Note**: No refactor note in proposal.
- **Recommendation**: Narrow scope to deferred loading, or clarify what join/subquery gaps remain (alias conflict resolution, correlated subqueries, etc.).

### Issue: Observability/retry tasks point to builder module instead of execution paths
- **Severity**: Medium
- **Category**: Conflict
- **Description**: Tasks 3.2–3.5 target `crates/ouroboros-postgres/src/query/mod.rs`, which is the builder and does not execute queries. Execution is in `crates/ouroboros-postgres/src/row.rs` and `crates/ouroboros/src/postgres/relations.rs`.
- **Location**: `agentd/changes/ob-pg-prod-ready-2/tasks.md`, `crates/ouroboros-postgres/src/row.rs`, `crates/ouroboros/src/postgres/relations.rs`
- **Note**: No refactor note in proposal.
- **Recommendation**: Move retry/logging/tracing tasks to the real execution modules.

## Quality Suggestions
[Missing tests, error handling, edge cases, documentation]
[These are LOW priority - nice to have improvements]

### Issue: Retry/backoff policy is underspecified
- **Severity**: Low
- **Category**: Completeness
- **Description**: `specs/robustness.md` does not define retry limits, max backoff, or total timeout, which affects production safety.
- **Recommendation**: Define default limits and whether they are configurable via `PoolConfig`.

### Issue: Decimal serialization acceptance criteria is ambiguous
- **Severity**: Low
- **Category**: Other
- **Description**: `specs/correctness.md` allows Decimal to be returned as Decimal, string, or float, which is not testable and risks precision loss.
- **Recommendation**: Specify a single, precise output type/format (e.g., Python `Decimal` via string).

### Issue: Transaction `deferrable` rules not captured
- **Severity**: Low
- **Category**: Completeness
- **Description**: PostgreSQL only allows `DEFERRABLE` with `SERIALIZABLE` and `READ ONLY`. Spec does not mention validation or error handling for invalid combos.
- **Recommendation**: Add ACs for allowed combinations and define behavior for invalid ones.

## Verdict
- [ ] APPROVED - Ready for implementation
- [x] NEEDS_REVISION - Address issues above (specify which severity levels)
- [ ] REJECTED - Fundamental problems, needs rethinking

**Next Steps**: Resolve HIGH severity spec/task mismatches, then update tasks to target actual execution paths and error model.
