# Tasks: Add Data Bridge CLI

## 1. Core Infrastructure
- [ ] 1.1 Create `python/data_bridge/cli/` package structure
- [ ] 1.2 Implement main Typer app entry point in `python/data_bridge/cli/main.py`
- [ ] 1.3 Configure `project.scripts` in `pyproject.toml` to expose `data-bridge` command
- [ ] 1.4 Implement `data-bridge.toml` config loader and parser
- [ ] 1.5 Implement preset system (small/medium/large) with default paths

## 2. Commands
- [ ] 2.1 Implement `data-bridge init` command to create config file
- [ ] 2.2 Implement `data-bridge config show` command
- [ ] 2.3 Implement `data-bridge api new route` command
- [ ] 2.4 Implement `data-bridge api new model` command
- [ ] 2.5 Implement `data-bridge api new middleware` command
- [ ] 2.6 Implement `data-bridge api new dependency` command
- [ ] 2.7 Implement `data-bridge api new websocket` command
- [ ] 2.8 Implement `data-bridge api new sse` command
- [ ] 2.9 Implement `data-bridge api new module` command (combines route + model + service)

## 3. Templates
- [ ] 3.1 Create route handler templates (GET/POST/PUT/DELETE)
- [ ] 3.2 Create model templates (BaseModel with Field)
- [ ] 3.3 Create middleware template (BaseMiddleware)
- [ ] 3.4 Create dependency template (Depends)
- [ ] 3.5 Create websocket template
- [ ] 3.6 Create SSE template
- [ ] 3.7 Create module templates (for each preset)

## 4. Utilities
- [ ] 4.1 Implement template rendering engine (string substitution)
- [ ] 4.2 Implement file writer with overwrite protection
- [ ] 4.3 Implement stdout formatter for small preset

## 5. Testing
- [ ] 5.1 Add unit tests for config loading
- [ ] 5.2 Add unit tests for CLI command parsing
- [ ] 5.3 Add integration tests that generate code and verify syntax (using `ast`)
- [ ] 5.4 Verify generated code passes `ruff` checks
