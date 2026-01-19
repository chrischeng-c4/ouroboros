# Specification: Dynamic Typeshed

<meta>
  <constraint>NO actual implementation code - use abstractions only</constraint>
  <abstractions>Mermaid, JSON Schema, Pseudo code, WHEN/THEN</abstractions>
</meta>

## Overview

Argus requires access to high-quality type stubs for the Python standard library and third-party packages. This spec defines a dynamic system that automatically fetches, caches, and loads stubs from the official typeshed repository, ensuring complete type information without bloating the core binary.

## Requirements

### R1: Tiered Stub Loading
The system SHALL search for stubs in the following order:
1. Local `.pyi` files in the workspace.
2. User-configured custom stub directories.
3. Dynamically downloaded typeshed cache.
4. Bundled fallback stubs (minimal set).

### R2: Automatic Downloader
The system SHALL download missing stubs from the `python/typeshed` GitHub repository. It MUST use HTTP conditional requests (ETags/If-None-Match) to avoid unnecessary downloads.

### R3: In-memory Stub Cache
Once loaded, stubs SHALL be cached in memory within the Argus Daemon to ensure zero-latency access during re-analysis.

### R4: Configuration Wiring
Typeshed settings (cache directory, enabled packages, custom paths) SHALL be unified in a global `ProjectConfig` and correctly threaded from `pyproject.toml` to the `StubLoader`.

## Data Model

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["package_name", "version_hash"],
  "properties": {
    "package_name": { "type": "string" },
    "version_hash": { "type": "string", "description": "Git SHA or ETag" },
    "last_checked": { "type": "string", "format": "date-time" },
    "local_path": { "type": "string" }
  }
}
```

## Acceptance Criteria

### Scenario: First-time Package Analysis
- **WHEN** analyzing a file that imports `requests` for the first time
- **THEN** Argus should download the `requests` stubs from typeshed and use them for type inference.

### Scenario: Offline Mode
- **WHEN** no internet connection is available
- **THEN** Argus should gracefully fall back to existing cached stubs or bundled minimal stubs without crashing.

### Scenario: Version Update
- **WHEN** the remote typeshed has an update (ETag change)
- **THEN** Argus should re-download the package stubs on the next check (subject to a refresh interval).
