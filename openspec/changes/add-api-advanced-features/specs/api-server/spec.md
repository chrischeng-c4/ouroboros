# Change: Add Advanced API Features

## ADDED Requirements

### Requirement: WebSocket Support
The system SHALL support bidirectional, real-time communication via the WebSocket protocol (RFC 6455).

#### Scenario: WebSocket Handshake
- **WHEN** a client sends a request to a registered WebSocket endpoint with `Upgrade: websocket` headers
- **THEN** the server upgrades the connection
- **AND** invokes the registered Python WebSocket handler

#### Scenario: Bidirectional Communication
- **WHEN** the handler awaits `ws.receive_text()`
- **THEN** it yields the next text message from the client
- **WHEN** the handler awaits `ws.send_text("Hello")`
- **THEN** the client receives the text message "Hello"

#### Scenario: Connection Closure
- **WHEN** the handler returns
- **THEN** the WebSocket connection is closed with code 1000 (Normal Closure)

### Requirement: Server-Sent Events (SSE)
The system SHALL support Server-Sent Events (SSE) for one-way server-to-client event streaming.

#### Scenario: Event Streaming
- **WHEN** a handler returns an `EventSourceResponse` with an async generator
- **THEN** the server sends the `Content-Type: text/event-stream` header
- **AND** streams yielded data formatted as SSE events (`data: ...\n\n`)

#### Scenario: Custom Event Fields
- **WHEN** the generator yields a dictionary `{"event": "update", "data": "payload", "id": "1"}`
- **THEN** the server formats the output as:
  ```
  event: update
  id: 1
  data: payload

  ```

### Requirement: Advanced Type Extractors
The system SHALL provide type-safe extractors for Path and Query parameters using Python type hints and Rust deserialization.

#### Scenario: Path Parameter Deserialization
- **WHEN** a route is defined as `/items/{item_id}`
- **AND** the handler signature is `def get_item(item_id: int)`
- **THEN** the system automatically extracts `item_id` from the path
- **AND** converts it to an integer (returning 404/422 if invalid)

#### Scenario: Query Parameter Deserialization
- **WHEN** a handler signature is `def search(q: str, limit: int = 10)`
- **AND** the request is `GET /search?q=foo&limit=5`
- **THEN** the handler receives `q="foo"` and `limit=5`
