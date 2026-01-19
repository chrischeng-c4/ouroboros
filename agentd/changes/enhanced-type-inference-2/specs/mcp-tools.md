# Specification: MCP Configuration Tools

## Overview

This specification defines the Model Context Protocol (MCP) tools that allow an LLM or external agent to programmatically inspect and configure the Python environment for Argus.

## Requirements

### R1: Configuration Inspection
The system SHALL provide a tool to list the current Python search paths and the active virtual environment.

### R2: Persistent Configuration Update
The system SHALL provide a tool to update the `[tool.argus]` section in `pyproject.toml` to ensure settings persist across sessions.

### R3: Environment Auto-Discovery Trigger
The system SHALL provide a tool to trigger the automatic environment detection logic and return the findings.

## Interfaces

### Tool: `argus_get_config`
- **INPUT**: none
- **OUTPUT**: Current active configuration (merged from all sources)

### Tool: `argus_set_python_paths`
- **INPUT**: `paths: String[]`
- **OUTPUT**: Success status
- **SIDE_EFFECTS**: Updates `pyproject.toml` with the new search paths.

### Tool: `argus_configure_venv`
- **INPUT**: `venv_path: String`
- **OUTPUT**: Success status
- **SIDE_EFFECTS**: Validates the path and updates `pyproject.toml`.

### Tool: `argus_detect_environment`
- **INPUT**: none
- **OUTPUT**: `detected_envs: {path: String, type: String}[]`
- **DESCRIPTION**: Runs the auto-detection logic and returns potential virtual environments found in the project.

### Tool: `argus_list_modules`
- **INPUT**: `prefix: String` (optional)
- **OUTPUT**: `modules: String[]`
- **DESCRIPTION**: Lists all modules currently discoverable by the `ImportResolver`.

## Acceptance Criteria

### Scenario: WHEN paths set via MCP THEN update config
- **WHEN** LLM calls `argus_set_python_paths(paths=["./lib"])`
- **THEN** The `pyproject.toml` file is updated to include `[tool.argus.python] search_paths = ["./lib"]`.

### Scenario: WHEN modules listed THEN show available modules
- **WHEN** LLM calls `argus_list_modules(prefix="django.")`
- **THEN** The system returns a list of all submodules of `django` available in the current environment.

### Scenario: WHEN environment detection triggered THEN return found venvs
- **WHEN** LLM calls `argus_detect_environment()`
- **THEN** The system scans the project root and returns a list of detected virtual environments (e.g., `.venv`, `venv`).
