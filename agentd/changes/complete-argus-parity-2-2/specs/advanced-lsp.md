# Specification: Advanced LSP & Semantic Indexing

<meta>
  <constraint>NO actual implementation code - use abstractions only</constraint>
  <abstractions>Mermaid, JSON Schema, Pseudo code, WHEN/THEN</abstractions>
</meta>

## Overview

To reach parity with mypy and pyright, Argus must support sophisticated LSP features like workspace-wide rename, find-references, and context-aware code actions. This requires a dedicated semantic indexing pass that tracks not just definitions, but all usage sites (references) across the entire project.

## Requirements

### R1: Global Reference Index
The system SHALL maintain a `ReferenceIndex` within the Argus Daemon that maps every unique symbol to a list of its usage locations (file, line, column).

### R2: Workspace-wide Rename
The system SHALL support `textDocument/rename`, ensuring that changing a symbol name updates all references in the workspace, including cross-module imports.

### R3: Cross-file Find References
The system SHALL support `textDocument/references` by querying the `ReferenceIndex` and returning all locations where a symbol is used.

### R4: Incremental Index Updates
When a file is modified, the `ReferenceIndex` SHALL be partially updated to remove old references from that file and add new ones, without a full workspace re-scan.

### R5: Context-Aware Code Actions
The system SHALL support `textDocument/codeAction` to provide quick fixes for common errors (e.g., "Import missing symbol", "Add type annotation") based on the current diagnostic context.

## Interfaces

```
FUNCTION find_references(symbol_id: SymbolId) -> List<Location>
  INPUT: Unique ID of a symbol (definition)
  OUTPUT: List of all files and ranges where this symbol is used

FUNCTION prepare_rename(location: Location) -> RenameResult
  INPUT: Location of the symbol to rename
  OUTPUT: Symbol metadata and confirmation that it is renameable
  ERRORS: CannotRename (e.g., for built-ins or third-party code)

FUNCTION perform_rename(symbol_id: SymbolId, new_name: str) -> WorkspaceEdit
  INPUT: Symbol to rename and the new name
  OUTPUT: A collection of file changes to be applied by the LSP client

FUNCTION get_code_actions(range: Range, diagnostics: List<Diagnostic>) -> List<CodeAction>
  INPUT: Selected range and active diagnostics
  OUTPUT: List of available quick fixes (e.g., "Import 'os'")
```

## Acceptance Criteria

### Scenario: Renaming a Class
- **WHEN** renaming a class `UserManager` to `AccountManager`
- **THEN** all occurrences of the class name, including its usage in type hints and imports in other files, should be updated.

### Scenario: Finding References of a Method
- **WHEN** requesting references for `User.get_email()`
- **THEN** it should return all call sites, even those where the instance type was inferred.

### Scenario: Import Quick Fix
- **WHEN** using `Path` without importing it
- **THEN** a code action "Import 'pathlib.Path'" should be available.

### Scenario: Reference Persistence
- **WHEN** the daemon restarts
- **THEN** it should re-build the reference index from the workspace so that find-references works immediately.