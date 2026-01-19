# Tasks

## 1. Data Layer

- [x] 1.1 Define configuration structs and `pyproject.toml` parser
  - File: `crates/argus/src/types/config.rs` (MODIFIED)
  - Spec: `specs/python-env.md#data-model`
  - Done: Added `PythonEnvConfig` struct with search_paths, venv_path, ignore_site_packages
  - Depends: none

- [x] 1.2 Implement environment detection logic
  - File: `crates/argus/src/types/env.rs` (CREATED)
  - Spec: `specs/python-env.md#r2-virtual-environment-detection`
  - Done: Implemented detection for venv, Poetry, Pipenv, VIRTUAL_ENV env var
  - Depends: 1.1

## 2. Logic Layer

- [x] 2.1 Enhance `ImportResolver` with search path support
  - File: `crates/argus/src/types/imports.rs` (MODIFIED)
  - Spec: `specs/import-resolution.md#requirements`
  - Done: Enhanced with module indexing, stub priority, lazy loading
  - Depends: 1.2

- [x] 2.2 Implement `site-packages` location logic
  - File: `crates/argus/src/types/env.rs` (INCLUDED IN 1.2)
  - Spec: `specs/python-env.md#r4-site-packages-discovery`
  - Done: `find_site_packages()` function supports Unix and Windows paths
  - Depends: 1.2

- [x] 2.3 Implement module indexing and lazy loading
  - File: `crates/argus/src/types/imports.rs` (MODIFIED)
  - Spec: `specs/import-resolution.md#r4-package-indexing`
  - Done: `build_index()`, `list_modules()`, circular import detection
  - Depends: 2.1

## 3. Integration

- [x] 3.1 Expose configuration via MCP tools
  - File: `crates/argus/src/mcp/tools.rs` (MODIFIED)
  - Spec: `specs/mcp-tools.md#interfaces`
  - Done: Added argus_get_config, argus_set_python_paths, argus_configure_venv, argus_detect_environment
  - Depends: 2.2

- [x] 3.2 Add module listing MCP tool
  - File: `crates/argus/src/mcp/tools.rs` (MODIFIED)
  - Spec: `specs/mcp-tools.md#tool-argus_list_modules`
  - Done: Added argus_list_modules with prefix filtering
  - Depends: 2.3

## 4. Testing

- [x] 4.1 Test environment detection
  - File: `crates/argus/src/types/env.rs` (TESTS INCLUDED)
  - Verify: `specs/python-env.md#acceptance-criteria`
  - Done: 11 tests covering venv detection, site-packages, config priority
  - Depends: 1.2

- [x] 4.2 Test cross-file import resolution
  - File: `crates/argus/src/types/imports.rs` (TESTS INCLUDED)
  - Verify: `specs/import-resolution.md#acceptance-criteria`
  - Done: 14 tests covering indexing, stub priority, circular imports
  - Depends: 2.3

- [x] 4.3 Test MCP tools
  - File: `crates/argus/src/mcp/tools.rs` (TESTS INCLUDED)
  - Verify: `specs/mcp-tools.md#acceptance-criteria`
  - Done: 6 tests covering all new MCP tools
  - Depends: 3.2
