# Specification: Postgres Observability

## Overview

Integration of structured logging and metrics.

## Requirements

### R1: Tracing
Trace all database operations with `tracing` spans.

### R2: Error Logging
Log failed queries with context.

## Acceptance Criteria

### Scenario: WHEN query executed THEN emit span
- **WHEN** A user executes a query
- **THEN** A span is emitted with the SQL statement

### Scenario: WHEN query fails THEN log context
- **WHEN** A query fails
- **THEN** The log contains the failed SQL
