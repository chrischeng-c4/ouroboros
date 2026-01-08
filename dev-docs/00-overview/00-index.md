---
title: data-bridge Documentation
status: implemented
component: general
type: index
---

# data-bridge Documentation

Welcome to the documentation for **data-bridge**.

## Mission

**data-bridge** is a collection of high-performance Rust solutions for Python scenarios.
The goal is to provide consistent, safe, and extremely fast implementations for common Python bottlenecks by offloading work to Rust.

## Solutions

### [1. MongoDB Solution](../10-mongodb/index.md)
A high-performance ORM/ODM that replaces Beanie/Motor.
- **Components**: [Rust Core Engine](../10-mongodb/01-core-engine/index.md), [Python API](../10-mongodb/02-python-api/index.md)
- **Key Features**: Zero-copy BSON, Rayon parallelism, Connection pooling.

### [2. HTTP Solution](../20-http-client/index.md)
A high-performance async HTTP client.
- **Replaces**: `httpx`, `aiohttp` (for specific use cases)
- **Key Features**: GIL-free request processing, Error sanitization.

### [3. Test Runner](../30-test-runner/index.md)
A specialized test runner for mixed Rust/Python projects.
- **Replaces**: `pytest` (for discovery/execution, not assertions)
- **Key Features**: Parallel test execution, Fast discovery via Rust `walkdir`.

### [4. PostgreSQL Solution](../40-postgres/index.md)
A high-performance async PostgreSQL ORM with Rust backend.
- **Components**: [Rust Core Engine](../40-postgres/01-core-engine/index.md), [Python API](../40-postgres/02-python-api/index.md)
- **Key Features**: CRUD operations, Transactions, Migrations, Connection pooling.
- **Driver**: sqlx with compile-time query validation.

### Future Solutions
- **Redis**: Async cache/queue interface.
- **MySQL**: Async driver.

## Getting Started

### Architecture
- [Architecture Principles](./02-architecture-principles.md)
- [Roadmap](./01-roadmap.md)

### Development
- See `GEMINI.md` in the root directory for the active development context and workflows.