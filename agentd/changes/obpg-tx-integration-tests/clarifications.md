---
change: obpg-tx-integration-tests
date: 2026-01-20
issue: "#72"
---

# Clarifications

## Q1: Database Setup
- **Question**: How should the integration tests connect to PostgreSQL?
- **Answer**: Local PostgreSQL (via `brew services`)
- **Rationale**: Developer already has local PostgreSQL running, simpler setup without Docker overhead

## Q2: Isolation Levels
- **Question**: What isolation levels should be tested?
- **Answer**: All levels (READ COMMITTED, REPEATABLE READ, SERIALIZABLE)
- **Rationale**: Comprehensive coverage of PostgreSQL isolation semantics

## Q3: Git Workflow
- **Question**: What's your preferred git workflow for this change?
- **Answer**: New branch (`agentd/obpg-tx-integration-tests`)
- **Rationale**: Isolated development for clean PR
