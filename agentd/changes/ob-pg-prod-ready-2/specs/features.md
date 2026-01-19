# Specification: Missing Features

## Overview

Implement missing query filters (`any_`, `has`) and transaction options to reach feature parity with the MongoDB backend and support application requirements.

## Requirements

### R1: Array Containment (`any_`)
Implement the `any_` operator for array columns.
- SQL Equivalent: `val = ANY(column)` or `column && ARRAY[val]` depending on context.

### R2: JSON Existence (`has`)
Implement the `has` operator for JSONB/Map columns.
- SQL Equivalent: `column ? key`

### R3: Transaction Options
Support `read_only` and `deferrable` flags when starting a transaction.
- SQL: `BEGIN TRANSACTION READ ONLY DEFERRABLE`

## Interfaces

```
FUNCTION build_any(col: str, val: Any) -> Condition
  OUTPUT: "col = ANY($1)" or similar

FUNCTION start_transaction(read_only: bool, deferrable: bool) -> Transaction
  SIDE_EFFECTS: Executes BEGIN with options
```

## Acceptance Criteria

### Scenario: WHEN any filter used THEN generate sql
- **WHEN** filtering with `any_("tags", "urgent")`
- **THEN** generates SQL `... WHERE 'urgent' = ANY(tags)`

### Scenario: WHEN has filter used THEN generate sql
- **WHEN** filtering with `has("metadata", "author")`
- **THEN** generates SQL `... WHERE metadata ? 'author'`

### Scenario: WHEN read only transaction THEN execute read only
- **WHEN** starting a transaction with `read_only=True`
- **THEN** executes `BEGIN READ ONLY`