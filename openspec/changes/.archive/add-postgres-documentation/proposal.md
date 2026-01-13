# Change: Add Postgres Documentation

## Why
The `data-bridge-postgres` ORM is feature-rich (relationships, inheritance, async, telemetry) but lacks comprehensive documentation. Current documentation is scattered in `docs/archive` or missing (approx. 15% coverage). To support adoption by Python developers, we need a complete documentation suite comparable to SQLAlchemy or Beanie.

## What Changes
- **Restructure**: Move valid legacy docs from `docs/archive/` to `docs/postgres/`.
- **Quickstart**: Add a modern `quickstart.md` covering async connection and basic CRUD.
- **Guides**: Add deep-dive guides for Tables, Queries, Validation, and Events.
- **API Reference**: comprehensive reference for all 90+ exports in `data_bridge.postgres`.
- **Navigation**: Update documentation structure to include these new sections.

## Impact
- **Affected Specs**: `postgres-orm`
- **Affected Docs**: `docs/postgres/`, `docs/archive/`
