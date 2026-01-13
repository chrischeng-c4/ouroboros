# Capability: CLI Scaffolding

The `cli-scaffold` capability provides a command-line interface to generate standardized boilerplate code for the `data-bridge` project, with support for different project scales via presets.

## ADDED Requirements

### Requirement: Project Initialization
The system SHALL provide a command to initialize a project with a configuration file.

#### Scenario: Initialize New Project
- **WHEN** the user runs `data-bridge init`
- **THEN** the system prompts for preset selection (small/medium/large)
- **AND** creates `data-bridge.toml` with appropriate defaults

#### Scenario: Initialize with Preset Flag
- **WHEN** the user runs `data-bridge init --preset medium`
- **THEN** the system creates `data-bridge.toml` with medium preset defaults without prompting

### Requirement: Configuration Management
The system SHALL read project configuration from `data-bridge.toml`.

#### Scenario: Show Configuration
- **WHEN** the user runs `data-bridge config show`
- **THEN** the system displays current preset and path settings

#### Scenario: Missing Config File
- **WHEN** the user runs any `api new` command without `data-bridge.toml`
- **THEN** the system defaults to `small` preset (stdout output)

### Requirement: Preset System
The system SHALL support three presets that determine output structure.

#### Scenario: Small Preset Output
- **WHEN** preset is `small`
- **AND** user runs `data-bridge api new route users`
- **THEN** the system outputs code to stdout (not files)

#### Scenario: Medium Preset Output
- **WHEN** preset is `medium`
- **AND** user runs `data-bridge api new route users`
- **THEN** the system creates `{routes_path}/users.py`

#### Scenario: Large Preset Output
- **WHEN** preset is `large`
- **AND** user runs `data-bridge api new route users`
- **THEN** the system creates `{routes_path}/users.py`
- **AND** creates `{services_path}/user_service.py` stub

### Requirement: Generate Route Handlers
The system SHALL provide a command to generate API route handlers.

#### Scenario: Generate GET Route
- **WHEN** the user runs `data-bridge api new route users --methods GET`
- **THEN** the system generates code containing:
  - Import from `data_bridge.api`
  - An `@app.get("/users")` decorated async function
  - Correct type hints using `Annotated`

#### Scenario: Generate CRUD Routes
- **WHEN** the user runs `data-bridge api new route users --methods GET,POST,PUT,DELETE`
- **THEN** the system generates all four HTTP method handlers

### Requirement: Generate Data Models
The system SHALL provide a command to generate Pydantic models.

#### Scenario: Generate Model with Fields
- **WHEN** the user runs `data-bridge api new model User --fields name:str,age:int,email:str`
- **THEN** the system generates a class `User(BaseModel)` with:
  - `name: str = Field(...)`
  - `age: int = Field(...)`
  - `email: str = Field(...)`
  - Imports from `data_bridge.api`

#### Scenario: Generate Model with Constraints
- **WHEN** the user runs `data-bridge api new model User --fields "name:str:min=1:max=100"`
- **THEN** the system generates `name: str = Field(min_length=1, max_length=100)`

### Requirement: Generate Middleware
The system SHALL provide a command to generate middleware classes.

#### Scenario: Generate Middleware
- **WHEN** the user runs `data-bridge api new middleware RateLimit`
- **THEN** the system generates a class `RateLimitMiddleware(BaseMiddleware)` implementing `__call__`

### Requirement: Generate Dependencies
The system SHALL provide a command to generate dependency providers.

#### Scenario: Generate Dependency
- **WHEN** the user runs `data-bridge api new dependency get_current_user`
- **THEN** the system generates an async function with usage example in docstring

### Requirement: Generate Real-time Handlers
The system SHALL provide commands to generate WebSocket and SSE handlers.

#### Scenario: Generate WebSocket Endpoint
- **WHEN** the user runs `data-bridge api new websocket chat`
- **THEN** the system generates an `@app.websocket("/chat")` handler

#### Scenario: Generate SSE Endpoint
- **WHEN** the user runs `data-bridge api new sse notifications`
- **THEN** the system generates a handler returning `EventSourceResponse`

### Requirement: Generate Feature Module
The system SHALL provide a command to generate a complete feature module.

#### Scenario: Generate Module (Medium Preset)
- **WHEN** preset is `medium`
- **AND** user runs `data-bridge api new module products --with-crud`
- **THEN** the system creates `products/` directory containing:
  - `__init__.py`
  - `routes.py` with CRUD handlers
  - `models.py` with Product model

#### Scenario: Generate Module (Large Preset)
- **WHEN** preset is `large`
- **AND** user runs `data-bridge api new module products --with-crud`
- **THEN** the system creates:
  - `{routes_path}/products.py`
  - `{models_path}/product.py`
  - `{services_path}/product_service.py`

### Requirement: Override Preset
The system SHALL allow temporary preset override via flag.

#### Scenario: Override to Stdout
- **WHEN** user runs `data-bridge api new route users --stdout`
- **THEN** the system outputs to stdout regardless of config preset

#### Scenario: Override Preset
- **WHEN** user runs `data-bridge api new route users --preset large`
- **THEN** the system uses large preset behavior for this command only
