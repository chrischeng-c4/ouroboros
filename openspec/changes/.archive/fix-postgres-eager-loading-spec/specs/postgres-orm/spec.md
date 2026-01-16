## MODIFIED Requirements
### Requirement: Eager Loading
The system SHALL support eager loading of relationships to prevent N+1 query problems.

#### Scenario: Explicit Options
- **WHEN** `options(selectinload("field"))` is applied to a query
- **THEN** the specified relationship is loaded efficiently (e.g., using batch IN queries)
- **AND** the relationship field is populated on the result objects

#### Scenario: Fetch Links Sugar
- **WHEN** `fetch_links=True` is passed to the find method
- **THEN** all defined relationships are eager loaded automatically
- **AND** no additional queries are needed to access them
