# Change: Clean Markdown Organization

## Why
The project currently has Markdown files scattered across the root directory, `docs/`, `deploy/`, and `benchmarks/`. This makes it difficult for developers to find relevant information and violates the "Single Source of Truth" principle. To improve discoverability and maintainability, we need to establish a clear hierarchy, consolidating legacy summaries, benchmarks, and technical specifications into their respective homes.

## What Changes
- **Archive Legacy Summaries**: Move root-level status reports and summaries to `docs/archive/`.
- **Consolidate Benchmarks**: Centralize all benchmark reports into `benchmarks/reports/`.
- **Centralize Sheet Specs**: Move detailed spreadsheet engine design documents from `docs/sheet-specs/` to `openspec/specs/spreadsheet-engine/design-docs/`.
- **Organize Dev Docs**: Move API, Testing, and Deployment docs into the structured `dev-docs/` hierarchy.
- **Group Examples**: Move root-level `*_example.py` files to `examples/`.

## Impact
- **Affected Specs**: `spreadsheet-engine` (Documentation location only).
- **Affected Code**: Documentation files only. No runtime code changes.
- **File Structure**:
    - `docs/archive/` created.
    - `benchmarks/reports/` created.
    - `openspec/specs/spreadsheet-engine/design-docs/` created.
    - `dev-docs/70-ops/` created.
