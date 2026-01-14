# API Server Specification Changes

## ADDED Requirements

### Requirement: Standalone Validation Model
The system SHALL provide a standalone Python validation module (`ouroboros.validation`) that replaces Pydantic, allowing developers to define data models with standard Python types and enforce them using the Rust validation engine without running a server.

#### Scenario: Standalone Usage
- **WHEN** a developer defines a model `class User(BaseModel): name: Annotated[str, Field(min_length=3)]`
- **AND** instantiates it with `User(name="Jo")`
- **THEN** the system raises a `ValidationError` originating from the Rust validator

#### Scenario: Nested Validation
- **WHEN** a model contains another model `class Group(BaseModel): leader: User`
- **AND** data is validated against `Group`
- **THEN** the system recursively validates the nested `User` object using Rust logic

### Requirement: Response Validation
The system SHALL support declaring a `response_model` in route decorators to automatically validate and serialize the handler's return value.

#### Scenario: Response Filtering
- **WHEN** a route is defined with `@app.get("/user", response_model=UserPublic)`
- **AND** the handler returns a `UserDB` object containing private fields (password)
- **THEN** the system validates the data against `UserPublic`
- **AND** returns a JSON response containing only the fields defined in `UserPublic`

#### Scenario: Response Validation Error
- **WHEN** the handler returns data that does not match `response_model`
- **THEN** the system logs a server error
- **AND** returns a 500 Internal Server Error to the client (to prevent leaking invalid data)

## MODIFIED Requirements

### Requirement: Type Extraction
The system SHALL extract and validate request data (Path, Query, Body, Header) based on Python type hints, including support for `Annotated` syntax.

#### Scenario: Complex Extraction
- **WHEN** a handler is defined as `def create_item(item: Item, q: Optional[str] = None)`
- **THEN** the system parses the JSON body into the `Item` model
- **AND** extracts the `q` query parameter
- **AND** returns a 422 Unprocessable Entity error if validation fails

#### Scenario: Annotated Constraints
- **WHEN** a handler argument is defined as `id: Annotated[int, Path(ge=1)]`
- **THEN** the system enforces the `ge=1` constraint via the Rust validator
