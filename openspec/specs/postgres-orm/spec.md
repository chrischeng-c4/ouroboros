# Specification: PostgreSQL ORM

## Purpose
To provide a modern, async-native PostgreSQL ORM with support for type-safe relationships, eager loading, and automatic schema migrations.

## Requirements

### Requirement: Relationship API
The system SHALL support defining relationships between tables using explicit types.

#### Scenario: Forward Reference
- **WHEN** a `ForeignKey[T]` field is defined
- **THEN** it creates a foreign key column in the database
- **AND** supports lazy loading of the related object

#### Scenario: Reverse Reference
- **WHEN** a `BackReference[T]` field is defined
- **THEN** it allows accessing related objects from the other side
- **AND** supports filtering and pagination of related items

### Requirement: Eager Loading
The system SHALL support eager loading of relationships using SQL JOINs to prevent N+1 query problems.

#### Scenario: Fetch Links
- **WHEN** `fetch_links` is used in a query
- **THEN** related data is retrieved in a single SQL query using JOINs
- **AND** the result objects have their relationship fields populated

### Requirement: Upsert Support
The system SHALL support atomic "insert or update" operations.

#### Scenario: Upsert One
- **WHEN** `upsert` is called with data and a conflict target
- **THEN** a row is inserted if it doesn't exist
- **AND** updated if it does exist (based on the conflict target)

### Requirement: Auto-Migration
The system SHALL automatically generate migration files by comparing the ORM schema with the database schema.

#### Scenario: Generate Migration
- **WHEN** a new field is added to a model
- **THEN** a migration file is generated containing the `ALTER TABLE` statement
- **AND** a corresponding down migration is created

### Requirement: Performance Targets
The system SHALL meet specific performance benchmarks.

#### Scenario: Insert Performance
- **WHEN** inserting 1000 rows with foreign keys
- **THEN** it completes in under 25ms

#### Scenario: Eager Load Performance
- **WHEN** finding 1000 rows with eager loaded relationships
- **THEN** it completes in under 20ms
