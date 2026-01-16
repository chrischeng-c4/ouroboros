## ADDED Requirements

### Requirement: Unified CLI Entry Point
The system SHALL provide a single command-line interface named `ob` to access all development and runtime tools.

#### Scenario: CLI Help
- **WHEN** user runs `ob --help`
- **THEN** it displays available subcommands including `qc`

### Requirement: Subcommand Structure
The CLI SHALL support hierarchical subcommands to group related functionality.

#### Scenario: QC Subcommand
- **WHEN** user runs `ob qc --help`
- **THEN** it displays quality control related commands like `run` and `collect`
