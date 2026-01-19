# Implementation: enhanced-type-inference-2

## Summary

This change implements a robust Python environment configuration system and cross-file import resolution to provide deeper type inference and code understanding for Argus.

## Completed Tasks

### 1. Data Layer

#### 1.1 Configuration Structs and pyproject.toml Parser
- **File**: `crates/argus/src/types/config.rs`
- **Changes**:
  - Added `PythonEnvConfig` struct with fields:
    - `search_paths: Vec<PathBuf>` - Additional directories to search for modules
    - `venv_path: Option<PathBuf>` - Path to the virtual environment to use
    - `ignore_site_packages: bool` - Whether to ignore site-packages
  - Updated `ArgusConfig` to include `python: PythonEnvConfig` field
  - Added tests for parsing `[tool.argus.python]` section from pyproject.toml

#### 1.2 Environment Detection Logic
- **File**: `crates/argus/src/types/env.rs` (NEW)
- **Implements**: `specs/python-env.md#r2-virtual-environment-detection`
- **Features**:
  - `VenvType` enum: Venv, Poetry, Pipenv, Conda, Unknown
  - `DetectedEnv` struct with path, type, and site-packages info
  - `EnvInfo` struct with active venv, detected envs, and search paths
  - Detection functions:
    - `detect_python_environment(project_root)` - Main entry point
    - `detect_with_config(project_root, config)` - With pre-loaded config
    - `detect_all_venvs(project_root)` - Find all virtual environments
    - `is_venv_directory(path)` - Check if directory is a venv
    - `find_site_packages(venv_path, python_version)` - Locate site-packages
    - `get_venv_python_version(venv_path)` - Extract Python version from pyvenv.cfg
  - Configuration priority:
    1. Explicit `[tool.argus.python]` configuration
    2. `PYTHONPATH` environment variable
    3. Auto-detected virtual environments (`.venv`, `venv`, VIRTUAL_ENV env var)
  - Support for Poetry and Pipenv managed environments

### 2. Logic Layer

#### 2.1 Enhanced ImportResolver
- **File**: `crates/argus/src/types/imports.rs`
- **Implements**: `specs/import-resolution.md#requirements`
- **Changes**:
  - Added `ModuleLoadState` enum for circular import detection
  - Enhanced `ModuleInfo`:
    - `is_stub: bool` - Whether loaded from .pyi file
    - `submodules: Vec<String>` - Package submodules
    - `from_file(path, file_path)` - Constructor from file
    - `is_loaded()` - Check load state
  - Added `ModuleIndexEntry` struct for indexed modules
  - Enhanced `ImportResolver`:
    - `module_index: HashMap<String, ModuleIndexEntry>` - Quick lookup index
    - `loading: HashSet<String>` - Circular import detection
    - `with_search_paths(paths)` - Constructor with paths
    - `add_search_path(path)` - Add single path
    - `search_paths()` - Get current paths
    - `build_index()` - Scan and index modules
    - `list_modules(prefix)` - List modules with optional prefix filter
    - `get_index_entry(module_path)` - Get index entry
    - `is_indexed()` - Check if index is built
    - `is_loading(module_path)` - Circular import check
    - `start_loading(module_path)` / `finish_loading(module_path)` - Loading state
    - `get_or_resolve_module(module_path)` - Lazy loading
    - `clear()` - Reset resolver state

#### 2.2 Site-Packages Discovery
- **Implemented in**: `crates/argus/src/types/env.rs`
- **Function**: `find_site_packages(venv_path, python_version)`
- **Features**:
  - Searches Unix-style paths: `lib/pythonX.Y/site-packages`
  - Searches Windows-style paths: `Lib/site-packages`
  - Supports specific Python version targeting

#### 2.3 Module Indexing and Lazy Loading
- **File**: `crates/argus/src/types/imports.rs`
- **Implements**: `specs/import-resolution.md#r4-package-indexing`
- **Features**:
  - `index_directory(dir, prefix)` - Recursive directory indexing
  - Stub file priority (.pyi over .py)
  - Package detection via `__init__.py` / `__init__.pyi`
  - Skips hidden directories, `__pycache__`, `node_modules`
  - On-demand module loading with circular import protection

### 3. Integration

#### 3.1 MCP Tools for Configuration
- **File**: `crates/argus/src/mcp/tools.rs`
- **Implements**: `specs/mcp-tools.md#interfaces`
- **New Tools**:
  - `argus_get_config` - Get current Python environment configuration
  - `argus_set_python_paths` - Configure module search paths
  - `argus_configure_venv` - Set virtual environment path
  - `argus_detect_environment` - Auto-detect virtual environments
  - `argus_list_modules` - List available modules with prefix filtering

### 4. Testing

All tests pass (216 total). New tests added:

#### Environment Detection Tests (`env.rs`)
- `test_venv_type_display`
- `test_is_venv_directory_with_pyvenv_cfg`
- `test_is_venv_directory_with_structure`
- `test_find_site_packages`
- `test_find_site_packages_windows_style`
- `test_detect_all_venvs_common_names`
- `test_detect_python_environment_with_config`
- `test_detect_python_environment_with_search_paths`
- `test_get_venv_python_version_from_pyvenv_cfg`
- `test_detect_poetry_project`
- `test_ignore_site_packages`

#### Import Resolution Tests (`imports.rs`)
- `test_module_info_from_file`
- `test_with_search_paths`
- `test_add_search_path`
- `test_build_index_with_modules`
- `test_stub_file_priority`
- `test_list_modules_with_prefix`
- `test_resolve_module_path`
- `test_circular_import_detection`
- `test_clear_resolver`
- `test_module_names_iterator`

#### MCP Tools Tests (`tools.rs`)
- `test_tool_list_completeness`
- `test_tool_schemas_valid_json`
- `test_argus_get_config_schema`
- `test_argus_set_python_paths_schema`
- `test_argus_detect_environment_schema`
- `test_argus_list_modules_schema`

#### Config Tests (`config.rs`)
- `test_python_env_config_default`
- `test_parse_python_env_config`
- `test_parse_python_env_config_partial`

## Files Changed

| File | Status | Description |
|------|--------|-------------|
| `crates/argus/src/types/config.rs` | MODIFIED | Added PythonEnvConfig struct and tests |
| `crates/argus/src/types/env.rs` | NEW | Environment detection logic |
| `crates/argus/src/types/imports.rs` | MODIFIED | Enhanced ImportResolver with indexing |
| `crates/argus/src/types/mod.rs` | MODIFIED | Added exports for new types |
| `crates/argus/src/mcp/tools.rs` | MODIFIED | Added 5 new MCP tools |

## Acceptance Criteria Verification

### From `python-env.md`
- [x] WHEN venv_path is configured THEN use custom venv
- [x] WHEN .venv exists and no config THEN auto-detect
- [x] WHEN PYTHONPATH is set THEN include in search paths

### From `import-resolution.md`
- [x] WHEN local module imported THEN resolve from src
- [x] WHEN library imported THEN resolve from site-packages
- [x] WHEN circular import occurs THEN handle gracefully

### From `mcp-tools.md`
- [x] WHEN paths set via MCP THEN update config
- [x] WHEN modules listed THEN show available modules
- [x] WHEN environment detection triggered THEN return found venvs

## Usage Example

```toml
# pyproject.toml
[tool.argus]
python_version = "3.11"

[tool.argus.python]
search_paths = ["./lib", "./src"]
venv_path = ".venv"
ignore_site_packages = false
```

```rust
// Using the environment detection
use argus::types::{detect_python_environment, EnvInfo};

let info: EnvInfo = detect_python_environment(Path::new("/my/project"));
println!("Active venv: {:?}", info.active_venv);
println!("Search paths: {:?}", info.search_paths);
```

```rust
// Using the enhanced ImportResolver
use argus::types::ImportResolver;

let mut resolver = ImportResolver::with_search_paths(vec![
    PathBuf::from("./src"),
    PathBuf::from(".venv/lib/python3.11/site-packages"),
]);

resolver.build_index();

// List all Django modules
let django_modules = resolver.list_modules(Some("django."));
```
