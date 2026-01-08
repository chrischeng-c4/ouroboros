# Specification: Task Queue

## Purpose
To offer a high-performance distributed task queue using NATS JetStream and Redis, supporting complex workflows and async execution.

## Requirements

### Requirement: High Performance
The system SHALL provide high-throughput task execution, significantly faster than Celery.

#### Scenario: Submission Throughput
- **WHEN** submitting tasks in batch
- **THEN** the system handles at least 100,000 operations per second

### Requirement: Reliable Backend
The system SHALL use NATS JetStream for guaranteed message delivery and Redis for result storage.

#### Scenario: Task Persistence
- **WHEN** a task is published
- **THEN** it is persisted in NATS until acknowledged
- **AND** data loss is prevented even if workers crash

### Requirement: Workflow Primitives
The system SHALL support complex task workflows.

#### Scenario: Chain Execution
- **WHEN** a chain of tasks is submitted
- **THEN** they execute sequentially, passing results to the next task

#### Scenario: Group Execution
- **WHEN** a group of tasks is submitted
- **THEN** they execute in parallel
- **AND** the results are aggregated

### Requirement: Async Native
The system SHALL be designed for Python's `asyncio` ecosystem.

#### Scenario: Async Task
- **WHEN** a task is defined with `async def`
- **THEN** it is executed in an event loop
- **AND** non-blocking I/O is supported
