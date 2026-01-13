# Change: Add Data Bridge CLI

## Why
The `data-bridge` project enforces strict architectural patterns (Zero Python Byte Handling, Rust-backed execution) that can be complex to implement correctly from scratch. Developers (both human and AI) often need to replicate boilerplate code for routes, models, and middleware.

A dedicated CLI tool will:
1. **Standardize Code**: Ensure all new components follow project conventions (e.g., correct imports, type hints).
2. **Accelerate Development**: Reduce time spent on boilerplate.
3. **Guide AI Agents**: Provide a deterministic way for AI coding agents to generate correct project structures.
4. **Scale with Projects**: Support small, medium, and large projects with appropriate structures.

## What Changes

### Core Features
- **New Capability**: `cli-scaffold` to define requirements for code generation.
- **New Python Module**: `data_bridge.cli` containing the Typer-based application.
- **Entry Point**: A `data-bridge` command available in the development environment.

### Preset System (Small/Medium/Large)
Three project presets to handle different scales:

| Preset | Structure | Output Behavior |
|--------|-----------|-----------------|
| **small** | Single-file (`main.py`, `models.py`) | stdout only, user copies code |
| **medium** | Feature modules (`users/`, `orders/`) | Creates module directories |
| **large** | Layered architecture (routes/services/repos) | Full separation of concerns |

### Configuration File
`data-bridge.toml` in project root:
```toml
[cli]
preset = "medium"  # small | medium | large

[cli.paths]
routes = "app/routes"
models = "app/models"
services = "app/services"
middleware = "app/middleware"
```

### Command Structure
```
data-bridge
├── init                    # Initialize data-bridge.toml
├── config show             # Show current configuration
└── api
    └── new
        ├── route <name>       # Route handlers
        ├── model <name>       # BaseModel classes
        ├── middleware <name>  # Middleware classes
        ├── dependency <name>  # Dependency providers
        ├── websocket <name>   # WebSocket endpoints
        ├── sse <name>         # SSE endpoints
        └── module <name>      # Complete feature module
```

### Templates
Pre-defined templates for each component type, with preset-aware output:
- Route Handlers (`@app.get`, `@app.post`)
- Pydantic Models (`BaseModel`, `Field`)
- Middleware (`BaseMiddleware`)
- Dependencies (`Depends`)
- WebSocket handlers
- SSE handlers
- Full Modules (CRUD scaffolding)

## Impact
- **Specs**: Adds `cli-scaffold` capability.
- **Codebase**: Adds `python/data_bridge/cli/` directory.
- **Configuration**: Updates `pyproject.toml` to register the CLI script.
- **New File**: `data-bridge.toml` config format for user projects.
