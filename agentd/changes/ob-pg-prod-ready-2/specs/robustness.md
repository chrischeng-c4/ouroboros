# Specification: Postgres Robustness

## Overview

Enhancements to ensure `ouroboros-postgres` is resilient to common production issues like database unavailability on startup and specific database constraint violations.

## Requirements

### R1: Connection Retries
The system MUST attempt to reconnect to the database with exponential backoff if the initial connection fails.

### R2: Advanced Pool Configuration
Users MUST be able to configure pool limits, timeouts, and lifetimes via `PoolConfig`.

### R3: Error Classification
Map PostgreSQL-specific error codes (Unique Violation, Foreign Key, Deadlock) to `DataBridgeError` variants.

## Acceptance Criteria

### Scenario: WHEN Postgres is unreachable at start THEN retry connection
- **WHEN** Application starts but Postgres is not yet reachable
- **THEN** Application retries connection with increasing delays until success or timeout

### Scenario: WHEN unique constraint is violated THEN return Conflict error
- **WHEN** An INSERT query violates a UNIQUE constraint
- **THEN** The returned error identifies as `DataBridgeError::Conflict`

### Scenario: WHEN pool is exhausted THEN return timeout error
- **WHEN** All connections are busy and `acquire_timeout` is reached
- **THEN** Return a specific timeout error
