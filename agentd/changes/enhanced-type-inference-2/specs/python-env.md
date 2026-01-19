# Specification: Python Environment Configuration

## Overview

This specification describes how Argus detects and configures the Python environment for type resolution. It defines the priority of configuration sources, the auto-detection logic for virtual environments, and the format for project-level settings in `pyproject.toml`.

## Requirements

### R1: Configuration Priority
The system SHALL resolve Python paths using the following priority:
1.  Explicit configuration in `pyproject.toml` (`[tool.argus]`).
2.  `PYTHONPATH` environment variable.
3.  Automatic detection of local virtual environments.
4.  System Python interpreter paths.

### R2: Virtual Environment Detection
The system SHALL attempt to detect virtual environments by checking:
- `VIRTUAL_ENV` environment variable.
- `.venv/` directory in project root.
- `venv/` directory in project root.
- Presence of `poetry.lock` (implies poetry managed env).
- Presence of `Pipfile` (implies pipenv managed env).

### R3: pyproject.toml [tool.argus] Format
The system SHALL support the `[tool.argus]` section in `pyproject.toml` for persistence of configuration that is optimized for LLM readability and modification.

### R4: Site-Packages Discovery
When a virtual environment is identified, the system SHALL locate its `site-packages` directory to enable resolution of third-party libraries.

## Data Model

### [tool.argus] Schema
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "python": {
      "type": "object",
      "properties": {
        "search_paths": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Additional directories to search for modules"
        },
        "venv_path": {
          "type": "string",
          "description": "Path to the virtual environment to use"
        },
        "ignore_site_packages": {
          "type": "boolean",
          "default": false
        }
      }
    }
  }
}
```

## Interfaces

```
FUNCTION detect_python_environment(project_root: Path) -> EnvInfo
  INPUT: Project root directory
  OUTPUT: EnvInfo containing detected venv path and search paths
  SIDE_EFFECTS: None

FUNCTION load_argus_config(project_root: Path) -> ArgusConfig
  INPUT: Project root directory
  OUTPUT: Merged configuration from pyproject.toml and environment
  ERRORS: ConfigParseError (if pyproject.toml is malformed)
```

## Acceptance Criteria

### Scenario: WHEN venv_path is configured THEN use custom venv
- **WHEN** `pyproject.toml` contains `[tool.argus.python] venv_path = "./custom_env"`
- **THEN** The system uses `./custom_env/lib/pythonX.Y/site-packages` for library resolution.

### Scenario: WHEN .venv exists and no config THEN auto-detect
- **WHEN** A `.venv/` directory exists in the project root and no `venv_path` is configured in `pyproject.toml`.
- **THEN** The system automatically selects `.venv/` as the active environment.

### Scenario: WHEN PYTHONPATH is set THEN include in search paths
- **WHEN** `PYTHONPATH` environment variable is set to `/extra/lib`
- **THEN** `/extra/lib` is included in the module search paths after `pyproject.toml` paths but before auto-detected paths.
