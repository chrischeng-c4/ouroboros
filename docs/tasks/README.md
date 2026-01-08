# data-bridge-tasks

High-performance distributed task queue for Python, powered by Rust.

## Features

- **High Performance**: 5-10x faster than Celery
- **Type Safe**: Rust core with Python type hints
- **Async Native**: Full async/await support
- **Workflows**: Chain, Group, Chord primitives
- **Observability**: Prometheus metrics, OpenTelemetry tracing
- **Reliable**: NATS JetStream for guaranteed delivery

## Quick Start

### Installation

```bash
pip install data-bridge[tasks]
```

### Basic Usage

```python
from data_bridge.tasks import task, init

# Initialize
await init(
    nats_url="nats://localhost:4222",
    redis_url="redis://localhost:6379"
)

# Define a task
@task(name="add", queue="math")
async def add(x: int, y: int) -> int:
    return x + y

# Execute asynchronously
result = await add.delay(1, 2)

# Get result
value = await result.get(timeout=30)
print(value)  # 3
```

### Workflows

```python
from data_bridge.tasks import Chain, Group, Chord

# Chain: Sequential execution
chain = Chain([
    add.s(1, 2),      # Returns 3
    add.s(10),        # Receives 3, returns 13
])
result = await chain.apply_async()

# Group: Parallel execution
group = Group([
    add.s(1, 2),
    add.s(3, 4),
    add.s(5, 6),
])
results = await group.apply_async()
print(await results.get())  # [3, 7, 11]

# Chord: Parallel + callback
chord = Chord(
    Group([add.s(i, i) for i in range(10)]),
    add.s(0),  # Sum callback
)
result = await chord.apply_async()
```

### Delayed Execution

```python
# Execute in 60 seconds
result = await add.apply_async(1, 2, countdown=60)

# Execute at specific time
from datetime import datetime, timedelta
eta = datetime.now() + timedelta(hours=1)
result = await add.apply_async(1, 2, eta=eta.isoformat())
```

### Retry Configuration

```python
@task(
    name="flaky_task",
    max_retries=5,
    retry_delay=1.0,  # Initial delay in seconds
)
async def flaky_task():
    # Will retry up to 5 times with exponential backoff
    ...
```

## Configuration

### NATS Configuration

```python
await init(
    nats_url="nats://user:pass@nats.example.com:4222",
    redis_url="redis://localhost:6379"
)
```

### Environment Variables

```bash
export NATS_URL="nats://localhost:4222"
export REDIS_URL="redis://localhost:6379"
export TASK_QUEUE_PREFIX="myapp"
```

## Observability

### Prometheus Metrics

Enable with `metrics` feature:

```python
from data_bridge.tasks import get_metrics

# Get metrics in Prometheus format
metrics_text = get_metrics()
```

Available metrics:
- `tasks_published_total` - Total tasks published
- `tasks_executed_total` - Total tasks executed (by status)
- `task_duration_seconds` - Task execution histogram
- `tasks_in_progress` - Currently executing tasks
- `task_retries_total` - Total retry attempts
- `task_failures_total` - Total failures (by error type)

### OpenTelemetry Tracing

Enable with `tracing-otel` feature:

```python
from data_bridge.tasks import init_tracing

init_tracing(
    service_name="my-worker",
    otlp_endpoint="http://jaeger:4317"
)
```

## Performance

Benchmarks vs Celery (1000 tasks):

| Metric | data-bridge-tasks | Celery | Speedup |
|--------|------------------|--------|---------|
| Submit (sync) | 50,000 ops/s | 5,000 ops/s | 10x |
| Submit (batch) | 100,000 ops/s | 10,000 ops/s | 10x |
| Result fetch | 20,000 ops/s | 3,000 ops/s | 6.7x |

## Architecture

```
┌─────────────────────────────────────────┐
│           Python Application            │
│  @task decorator, delay(), apply_async()│
└────────────────┬────────────────────────┘
                 │ PyO3 FFI
┌────────────────▼────────────────────────┐
│         data-bridge-tasks (Rust)        │
│  • Zero-copy serialization              │
│  • Async I/O with Tokio                 │
│  • Memory-safe concurrency              │
└────────────────┬────────────────────────┘
                 │
    ┌────────────┴────────────┐
    ▼                         ▼
┌─────────┐              ┌─────────┐
│  NATS   │              │  Redis  │
│JetStream│              │ Backend │
└─────────┘              └─────────┘
```

## Advanced Features

### Periodic Tasks (Cron-like)

```python
from data_bridge.tasks import PeriodicTask, PeriodicSchedule

# Define periodic task
@task(name="cleanup")
async def cleanup():
    print("Running cleanup...")

# Schedule to run every hour
schedule = PeriodicSchedule.cron("0 * * * *")  # Every hour
periodic = PeriodicTask(cleanup, schedule)
await periodic.start()
```

### Worker Configuration

```python
from data_bridge.tasks import Worker, WorkerConfig

config = WorkerConfig(
    queues=["default", "high-priority"],
    concurrency=10,
    prefetch_count=20,
)

worker = Worker(config)
await worker.start()
```

### Task Cancellation

```python
# Submit task
result = await long_task.delay()

# Cancel before completion
await result.cancel()
```

## Testing

### Unit Testing Tasks

```python
import pytest
from data_bridge.tasks import task

@task(name="add")
async def add(x: int, y: int) -> int:
    return x + y

@pytest.mark.asyncio
async def test_add():
    # Direct invocation (no broker)
    result = await add(1, 2)
    assert result == 3
```

### Integration Testing

```python
@pytest.mark.asyncio
async def test_task_execution():
    # Initialize with test backend
    await init(
        nats_url="nats://localhost:4222",
        redis_url="redis://localhost:6379"
    )

    # Submit task
    result = await add.delay(1, 2)

    # Wait for result
    value = await result.get(timeout=5)
    assert value == 3
```

## Troubleshooting

### Connection Issues

```python
# Check NATS connection
from data_bridge.tasks import health_check

status = await health_check()
print(f"Broker: {status['broker']}")
print(f"Backend: {status['backend']}")
```

### Task Not Executing

1. Ensure worker is running: `python -m data_bridge.tasks worker`
2. Check queue name matches
3. Verify NATS/Redis connectivity
4. Check logs for errors

### Performance Issues

1. Increase worker concurrency
2. Use batch operations for multiple tasks
3. Enable metrics to identify bottlenecks
4. Check NATS/Redis resource usage

## Migration from Celery

data-bridge-tasks is designed to be a drop-in replacement for most Celery use cases:

```python
# Celery
from celery import Celery
app = Celery('myapp', broker='redis://localhost')

@app.task
def add(x, y):
    return x + y

# data-bridge-tasks (equivalent)
from data_bridge.tasks import task, init

await init(redis_url='redis://localhost')

@task(name='add')
async def add(x: int, y: int) -> int:
    return x + y
```

Key differences:
- data-bridge-tasks is async-first (use `async def`)
- Must call `init()` before using tasks
- Results use `await result.get()` instead of `result.get()`

## License

MIT

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development setup and guidelines.
