# Specification: HTTP Client

## Purpose
To provide a thread-safe, GIL-free HTTP client that uses connection pooling and background execution for high-concurrency network operations.

## Requirements

### Requirement: Connection Pooling
The system SHALL maintain a pool of long-lived HTTP connections to reuse for multiple requests.

#### Scenario: Multiple Requests
- **WHEN** multiple requests are made to the same host
- **THEN** the underlying TCP/TLS connection is reused
- **AND** handshake latency is incurred only once

### Requirement: GIL-Free Execution
The system SHALL execute HTTP requests in background threads without holding the Python GIL.

#### Scenario: Blocking Request
- **WHEN** a Python script makes a synchronous-looking HTTP request
- **THEN** the GIL is released immediately
- **AND** the request executes in a Rust Tokio task
- **AND** the GIL is re-acquired only when processing the response

### Requirement: Error Sanitization
The system SHALL sanitize error messages to prevent leaking sensitive information.

#### Scenario: API Key in URL
- **WHEN** a request with an API key query parameter fails
- **THEN** the error message replaces the key value with `[REDACTED]`

#### Scenario: Basic Auth
- **WHEN** a request with Basic Auth credentials fails
- **THEN** the username and password are redacted from the error log

### Requirement: Shared State Architecture
The client SHALL use thread-safe shared state to allow efficient cloning and sharing across tasks.

#### Scenario: Client Cloning
- **WHEN** the client object is cloned
- **THEN** it shares the same internal connection pool
- **AND** the operation is lightweight (pointer copy)
