## ADDED Requirements

### Requirement: Pure Rust Event Loop
The system SHALL provide a Python `asyncio` event loop implementation written entirely in Rust, utilizing `tokio` as the underlying reactor.

#### Scenario: Runtime Initialization
- **WHEN** `data_bridge.pyloop.install()` is called
- **THEN** the default `asyncio` event loop policy is replaced with `PyLoopPolicy`
- **AND** subsequent calls to `asyncio.get_event_loop()` return a Rust-backed `PyLoop` instance.

### Requirement: Unified Execution Model
The system SHALL execute both Rust-native futures and Python coroutines on the same global `tokio::Runtime`, ensuring a 1:1 mapping between the process and the event loop.

#### Scenario: Cross-Language Await
- **WHEN** a Python coroutine `await`s a Rust function
- **THEN** the Rust future is polled by the same Tokio worker thread
- **AND** the GIL is released during the poll if the Rust future yields.

### Requirement: Zero-Copy Task Scheduling
The system SHALL schedule Python coroutines onto the Tokio runtime without serializing/deserializing task data or creating intermediate Python thread wrappers.

#### Scenario: High-Throughput Scheduling
- **WHEN** 10,000 "no-op" Python tasks are scheduled via `loop.create_task()`
- **THEN** the memory overhead SHALL NOT exceed that of standard `uvloop`.
