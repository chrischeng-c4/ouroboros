## MODIFIED Requirements
### Requirement: HTTP Server
The system SHALL provide a high-performance HTTP server capable of handling API requests with minimal latency.

#### Scenario: Hybrid Dispatch
- **WHEN** the server receives a request
- **THEN** it MUST route the request entirely in Rust without acquiring the Python GIL
- **AND** if the handler is native Rust, execute it on the Tokio thread pool
- **AND** if the handler is Python, spawn it on the PyLoop event loop

### Requirement: Request Handling
The system SHALL support both function-based handlers and class-based controllers.

#### Scenario: Python Async Handler
- **WHEN** a route is mapped to a Python `async def` function
- **THEN** the system SHALL invoke the function using the PyLoop task system
- **AND** await the result asynchronously without blocking the HTTP acceptor thread

## ADDED Requirements
### Requirement: Declarative CRUD
The system SHALL support automatically generating CRUD endpoints from data models.

#### Scenario: Auto-Generated Routes
- **WHEN** a user decorates a Pydantic model or class with `@app.crud`
- **THEN** the system SHALL automatically register 5 endpoints (GET list, GET id, POST, PUT, DELETE)
- **AND** these endpoints SHALL be implemented in pure Rust using the MongoDB ORM
- **AND** they SHALL NOT execute any Python code during request processing (Zero-Python Path)

### Requirement: Unified Runtime
The system SHALL use a single asynchronous runtime for both I/O and Python execution.

#### Scenario: Shared Tokio Runtime
- **WHEN** the application starts
- **THEN** it SHALL initialize a single Tokio runtime
- **AND** the HTTP server SHALL run on this runtime
- **AND** the Python `asyncio` loop SHALL be backed by this same runtime
