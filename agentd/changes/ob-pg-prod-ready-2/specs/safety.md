# Specification: Safety Audit

## Overview

Audit and harden the codebase by replacing unsafe `unwrap()` and `expect()` calls with proper error propagation in critical paths (Connection, CRUD, Transaction).

## Requirements

### R1: Critical Path Hardening
The following modules MUST NOT panic on runtime errors:
- `connection.rs`
- `transaction.rs`
- `query/` (Builders and execution)
- `row.rs` (Data extraction)

### R2: Error Propagation
All potential failures (parsing, type conversion, DB errors) MUST return `Result<T, DataBridgeError>`.

## Acceptance Criteria

### Scenario: WHEN malformed URI provided THEN return error
- **WHEN** `Connection::new` is called with a malformed URI
- **THEN** it returns `Err(DataBridgeError::Connection)` instead of panicking

### Scenario: WHEN wrong type extracted THEN return error
- **WHEN** extracting a field from a row with the wrong type
- **THEN** it returns `Err(DataBridgeError::Deserialization)` instead of panicking

### Scenario: WHEN invalid query built THEN return error
- **WHEN** building a query with invalid operators or fields
- **THEN** it returns `Result::Err` instead of unwrapping internal state