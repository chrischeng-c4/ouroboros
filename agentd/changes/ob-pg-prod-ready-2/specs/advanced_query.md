# Specification: Advanced Query Support

## Overview

Implement missing advanced query strategies including deferred column loading, complex join handling, and subqueries to support the full ORM feature set.

## Requirements

### R1: Deferred Column Loading
Support loading specific columns only when requested, allowing for lighter initial queries.
- Implementation must support `defer()` and `only()` query options.

### R2: Join Building Strategies
Implement robust SQL generation for `INNER`, `LEFT`, and `RIGHT` joins, handling alias generation and conflict resolution.

### R3: Subquery Support
Enable the use of subqueries within `WHERE` clauses and `SELECT` lists.

## Acceptance Criteria

### Scenario: WHEN defer used THEN exclude column
- **WHEN** a query specifies `defer("large_blob")`
- **THEN** the generated SQL SELECT list excludes "large_blob"

### Scenario: WHEN join called THEN generate join sql
- **WHEN** building a query with `join("related_table")`
- **THEN** the generated SQL includes `LEFT JOIN related_table ON ...` with correct aliases

### Scenario: WHEN subquery in where THEN generate in clause
- **WHEN** filtering with a subquery `where("id", "in", subquery)`
- **THEN** the generated SQL includes `WHERE id IN (SELECT ...)`