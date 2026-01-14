# Change: Fix PostgreSQL Eager Loading Spec

## Why
The current `postgres-orm` spec incorrectly references a `fetch_links` API that doesn't exist in the implementation, while failing to document the actual `options()` API that is implemented. This creates a gap between spec and reality. Additionally, users expect the simple `fetch_links=True` syntax (familiar from Beanie) for common use cases.

## What Changes
- **Update Spec**: Document the `options(selectinload(...))` pattern as the primary eager loading mechanism.
- **Add Syntactic Sugar**: Introduce `fetch_links=True` to `find()` and `find_one()` methods.
- **Implementation Detail**: `fetch_links=True` will automatically apply `selectinload` for all defined relationships.
  - *Note*: The request suggested mapping to `joined`, but `JoinedLoad` is currently unimplemented in the codebase. We map to `selectinload` to ensure robust eager loading (solving the N+1 problem) without requiring a major refactor of the query builder.

## Impact
- **Affected Specs**: `postgres-orm`
- **Affected Code**: `python/ouroboros/postgres/table.py` (add `fetch_links` parameter)
