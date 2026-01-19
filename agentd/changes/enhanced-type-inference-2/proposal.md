# Change: enhanced-type-inference-2

## Summary

Implement a robust Python environment configuration system and cross-file import resolution to provide deeper type inference and code understanding, especially for LLM-driven development.

## Why

Current type inference in Argus is limited to single-file analysis and a small set of hardcoded builtins. To accurately check types and provide meaningful context to LLMs, the system must:
1.  **Understand the project structure**: Locate source files and dependencies.
2.  **Respect environment settings**: Support various virtual environment managers (venv, poetry, pipenv).
3.  **Resolve cross-file imports**: Track types across different modules within a project and in external libraries (site-packages).
4.  **Be programmable**: Allow LLMs to inspect and modify environment settings via MCP tools.

## What Changes

- **Python Configuration System**:
    - Support for `pyproject.toml` with a new `[tool.argus]` section for project-specific settings.
    - Multi-level configuration priority: `[tool.argus]` > `PYTHONPATH` > Automatic detection.
- **Environment Detection**:
    - Automatic detection of virtual environments: `.venv/`, `venv/`, `VIRTUAL_ENV` environment variable, `poetry.lock`, `pipenv`.
- **Enhanced Import Resolution**:
    - Upgrade `ImportResolver` to search for modules in project paths and `site-packages`.
    - Support for `.py` and `.pyi` (stubs) files.
    - Incremental loading and basic caching of resolved module types.
- **MCP Tool Exposure**:
    - `argus_set_python_paths`: Configure search paths.
    - `argus_detect_environment`: Trigger auto-detection and update config.
    - `argus_configure_project`: Programmatically update `pyproject.toml`.
    - `argus_list_modules`: List available modules for resolution.

## Impact

- Affected specs: `python-env.md`, `import-resolution.md`, `mcp-tools.md`
- Affected code: `crates/argus/src/types/imports.rs`, `crates/argus/src/config.rs` (to be created/expanded), MCP server implementation in `argus-mcp`.
- Breaking changes: No. Existing single-file inference will continue to work, but with better accuracy when environment is configured.