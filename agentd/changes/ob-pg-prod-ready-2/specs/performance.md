# Specification: Postgres Performance

## Overview

Performance optimizations for `ouroboros-postgres` focusing on prepared statement caching and automatic retries.

## Requirements

### R1: Statement Caching
Utilize `sqlx` prepared statement caching to reduce parsing overhead.

### R2: Transient Error Retries
Automatically retry queries that fail with transient errors like deadlocks.

## Acceptance Criteria

### Scenario: WHEN deadlock occurs THEN auto retry
- **WHEN** A query fails with a deadlock error
- **THEN** The ORM retries the operation automatically

### Scenario: WHEN same query repeated THEN reuse statement
- **WHEN** The same query structure is executed multiple times
- **THEN** The database driver reuses the prepared statement
