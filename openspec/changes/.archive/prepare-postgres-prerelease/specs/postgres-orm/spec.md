# Specification: PostgreSQL ORM

## Purpose
To provide a modern, async-native PostgreSQL ORM with support for type-safe relationships, eager loading, and automatic schema migrations.

## MODIFIED Requirements
### Requirement: Performance Targets
The system SHALL meet specific performance benchmarks to ensure it is faster than pure Python alternatives.

#### Scenario: Insert Performance
- **WHEN** inserting 1000 rows with foreign keys
- **THEN** it completes in under 25ms (p95)

#### Scenario: Eager Load Performance
- **WHEN** finding 1000 rows with eager loaded relationships
- **THEN** it completes in under 20ms (p95)

#### Scenario: Serialization Overhead
- **WHEN** deserializing 10,000 complex rows
- **THEN** overhead is less than 5ms total

## ADDED Requirements

### Requirement: Documentation Completeness
The system SHALL provide comprehensive documentation for all public APIs.

#### Scenario: Public API Docs
- **WHEN** `cargo doc` is generated
- **THEN** all public structs, enums, and functions have documentation comments
- **AND** the crate root documentation includes at least 3 usage examples

### Requirement: Migration Verification
The migration system SHALL provide robust validation and verification mechanisms.

#### Scenario: Checksum Validation
- **WHEN** a migration file is modified after being applied
- **THEN** `MigrationRunner` detects the checksum mismatch
- **AND** prevents further migrations until resolved

#### Scenario: Rollback Consistency
- **WHEN** a migration is rolled back
- **THEN** the database schema returns exactly to the previous state
- **AND** the migration record is removed from the tracking table

### Requirement: Introspection Accuracy
The schema introspection system SHALL accurately reflect the underlying database state.

#### Scenario: Complex Type Introspection
- **WHEN** a table contains Arrays, JSONB, and Enums
- **THEN** `SchemaInspector` correctly identifies these types
- **AND** maps them to the correct internal `ColumnType` variants

#### Scenario: Foreign Key Introspection
- **WHEN** a table has foreign keys with cascade rules
- **THEN** `SchemaInspector` captures the correct `ON DELETE`/`ON UPDATE` rules
- **AND** correctly identifies the referenced table and columns
