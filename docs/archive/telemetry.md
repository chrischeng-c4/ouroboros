# OpenTelemetry Integration

The `data_bridge.postgres.telemetry` module provides comprehensive OpenTelemetry integration for tracing and monitoring database operations.

## Features

- **Automatic Instrumentation**: Decorators for easy span creation
- **Manual Span Control**: Context managers for fine-grained tracing
- **Semantic Conventions**: Follows OpenTelemetry database conventions
- **Connection Pool Metrics**: Monitor pool utilization
- **Graceful Degradation**: Works without OpenTelemetry SDK installed
- **Minimal Overhead**: <1ms decorator overhead when tracing is disabled

## Quick Start

### Installation

```bash
# Install OpenTelemetry SDK (optional)
pip install opentelemetry-api opentelemetry-sdk
```

### Basic Usage

```python
from data_bridge.postgres.telemetry import (
    instrument_query,
    create_query_span,
    set_span_result,
)

# Automatic instrumentation with decorator
@instrument_query("find")
async def find_users(filters):
    users = await User.find(filters).to_list()
    return users

# Manual span creation
async def complex_query():
    with create_query_span("find", "users", filters_count=3, limit=10) as span:
        result = await execute_query()
        set_span_result(span, count=len(result))
        return result
```

## Configuration

### Environment Variables

- `DATA_BRIDGE_TRACING_ENABLED`: Enable/disable tracing (default: `true`)
  - Set to `false`, `0`, or `no` to disable

```bash
# Disable tracing
export DATA_BRIDGE_TRACING_ENABLED=false

# Enable tracing (default)
export DATA_BRIDGE_TRACING_ENABLED=true
```

### Checking Tracing Status

```python
from data_bridge.postgres.telemetry import is_tracing_enabled

if is_tracing_enabled():
    print("Tracing is active")
```

## Instrumentation Decorators

### @instrument_span

General-purpose span decorator for any function.

```python
from data_bridge.postgres.telemetry import instrument_span

@instrument_span("custom.operation", attributes={"key": "value"})
async def my_function():
    # Function body
    pass
```

**Parameters:**
- `name` (str, optional): Span name (defaults to `module.function_name`)
- `attributes` (dict, optional): Additional span attributes

### @instrument_query

Query-specific decorator that automatically adds database attributes.

```python
from data_bridge.postgres.telemetry import instrument_query

@instrument_query("find")
async def find_users(age_min: int):
    return await User.find(User.age > age_min).to_list()
```

**Parameters:**
- `operation` (str): Database operation name (e.g., "find", "insert", "update", "delete")

### @instrument_session

Session-specific decorator for session operations.

```python
from data_bridge.postgres.telemetry import instrument_session

@instrument_session("flush")
async def flush_changes(session):
    await session.flush()
```

**Parameters:**
- `operation` (str): Session operation name (e.g., "flush", "commit", "rollback")

## Span Context Managers

### create_query_span

Create a query span with standard database attributes.

```python
from data_bridge.postgres.telemetry import create_query_span, set_span_result

with create_query_span(
    operation="find",
    table="users",
    filters_count=2,
    limit=10,
    offset=0,
    order_by="created_at DESC"
) as span:
    result = await db.query(...)
    set_span_result(span, count=len(result))
```

**Parameters:**
- `operation` (str): Database operation (e.g., "find", "insert", "update")
- `table` (str, optional): Table name
- `filters_count` (int, optional): Number of filter conditions
- `limit` (int, optional): Query limit
- `offset` (int, optional): Query offset
- `order_by` (str, optional): Order by clause
- `statement` (str, optional): SQL statement (use cautiously for cardinality)
- `**attributes`: Additional custom attributes

### create_session_span

Create a session span with session state attributes.

```python
from data_bridge.postgres.telemetry import create_session_span

with create_session_span(
    operation="flush",
    pending_count=5,
    dirty_count=3,
    deleted_count=1
) as span:
    await session.flush()
```

**Parameters:**
- `operation` (str): Session operation
- `pending_count` (int, optional): Number of pending objects
- `dirty_count` (int, optional): Number of dirty objects
- `deleted_count` (int, optional): Number of deleted objects
- `**attributes`: Additional custom attributes

### create_relationship_span

Create a relationship loading span.

```python
from data_bridge.postgres.telemetry import create_relationship_span

with create_relationship_span(
    name="user.posts",
    strategy="selectin",
    depth=1
) as span:
    posts = await load_relationship(...)
    set_span_result(span, count=len(posts))
```

**Parameters:**
- `name` (str): Relationship name
- `strategy` (str, optional): Loading strategy ("lazy", "eager", "selectin", "joined")
- `depth` (int, optional): Nesting depth of relationship loading
- `**attributes`: Additional custom attributes

## Helper Functions

### add_exception

Record an exception in a span.

```python
from data_bridge.postgres.telemetry import create_query_span, add_exception

with create_query_span("find", "users") as span:
    try:
        result = await db.query(...)
    except Exception as e:
        add_exception(span, e)
        raise
```

### set_span_result

Set result attributes on a span.

```python
from data_bridge.postgres.telemetry import set_span_result

with create_query_span("find", "users") as span:
    result = await db.query(...)
    set_span_result(span, count=len(result))
```

**Parameters:**
- `span` (Span): The span to update
- `count` (int, optional): Number of results returned
- `affected_rows` (int, optional): Number of rows affected

## Connection Pool Metrics

Monitor connection pool utilization with gauge metrics.

```python
from data_bridge.postgres.telemetry import get_connection_pool_metrics

metrics = get_connection_pool_metrics()

# Record pool statistics
metrics.record_pool_stats(
    in_use=5,
    idle=3,
    max_size=10
)
```

**Metrics:**
- `db.connection.pool.in_use`: Connections currently in use
- `db.connection.pool.idle`: Idle connections available
- `db.connection.pool.max`: Maximum pool size

## Semantic Conventions

The module follows [OpenTelemetry Semantic Conventions for Database Calls](https://opentelemetry.io/docs/specs/semconv/database/).

### SpanAttributes

Standard database span attributes:

```python
from data_bridge.postgres.telemetry import SpanAttributes

# System
SpanAttributes.DB_SYSTEM                    # "db.system" = "postgresql"
SpanAttributes.DB_OPERATION_NAME            # "db.operation.name"
SpanAttributes.DB_COLLECTION_NAME           # "db.collection.name" (table)
SpanAttributes.DB_STATEMENT                 # "db.statement"

# Query
SpanAttributes.DB_QUERY_FILTERS_COUNT       # "db.query.filters_count"
SpanAttributes.DB_QUERY_LIMIT               # "db.query.limit"
SpanAttributes.DB_QUERY_OFFSET              # "db.query.offset"
SpanAttributes.DB_QUERY_ORDER_BY            # "db.query.order_by"

# Result
SpanAttributes.DB_RESULT_COUNT              # "db.result.count"
SpanAttributes.DB_RESULT_AFFECTED_ROWS      # "db.result.affected_rows"

# Session
SpanAttributes.DB_SESSION_OPERATION         # "db.session.operation"
SpanAttributes.DB_SESSION_PENDING_COUNT     # "db.session.pending_count"
SpanAttributes.DB_SESSION_DIRTY_COUNT       # "db.session.dirty_count"
SpanAttributes.DB_SESSION_DELETED_COUNT     # "db.session.deleted_count"

# Relationship
SpanAttributes.DB_RELATIONSHIP_NAME         # "db.relationship.name"
SpanAttributes.DB_RELATIONSHIP_STRATEGY     # "db.relationship.strategy"
SpanAttributes.DB_RELATIONSHIP_DEPTH        # "db.relationship.depth"
```

### MetricNames

Standard metric names:

```python
from data_bridge.postgres.telemetry import MetricNames

# Connection pool
MetricNames.CONNECTION_POOL_IN_USE          # "db.connection.pool.in_use"
MetricNames.CONNECTION_POOL_IDLE            # "db.connection.pool.idle"
MetricNames.CONNECTION_POOL_MAX             # "db.connection.pool.max"

# Query
MetricNames.QUERY_DURATION                  # "db.query.duration"
MetricNames.QUERY_COUNT                     # "db.query.count"
```

## Performance Considerations

### Low Cardinality Span Names

Span names use low-cardinality values to avoid excessive metric explosion:

✅ **Good** (low cardinality):
```python
# Span name: "db.query.find"
with create_query_span("find", table="users"):
    ...
```

❌ **Bad** (high cardinality):
```python
# Don't include unique values in span names
# Use attributes instead
```

### Fast Path Optimization

When tracing is disabled, decorators have minimal overhead (<1ms):

```python
# Fast path: immediate return if tracing disabled
@instrument_query("find")
async def find_users():
    # No span overhead when DATA_BRIDGE_TRACING_ENABLED=false
    ...
```

### Lazy Span Creation

Spans are only created if tracing is enabled:

```python
def is_tracing_enabled() -> bool:
    """Check before creating spans."""
    if not OTEL_AVAILABLE:
        return False
    return os.environ.get("DATA_BRIDGE_TRACING_ENABLED", "true").lower() not in ("false", "0", "no")
```

## Examples

See [`examples/telemetry_example.py`](../examples/telemetry_example.py) for comprehensive usage examples.

## Integration with OpenTelemetry SDK

### Console Exporter (Development)

```python
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import ConsoleSpanExporter, SimpleSpanProcessor

# Set up tracer provider
provider = TracerProvider()
processor = SimpleSpanProcessor(ConsoleSpanExporter())
provider.add_span_processor(processor)
trace.set_tracer_provider(provider)
```

### Jaeger Exporter (Production)

```bash
pip install opentelemetry-exporter-jaeger
```

```python
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.jaeger.thrift import JaegerExporter

# Configure Jaeger exporter
jaeger_exporter = JaegerExporter(
    agent_host_name="localhost",
    agent_port=6831,
)

# Set up tracer provider
provider = TracerProvider()
processor = BatchSpanProcessor(jaeger_exporter)
provider.add_span_processor(processor)
trace.set_tracer_provider(provider)
```

### OTLP Exporter (OpenTelemetry Collector)

```bash
pip install opentelemetry-exporter-otlp
```

```python
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter

# Configure OTLP exporter
otlp_exporter = OTLPSpanExporter(
    endpoint="http://localhost:4317",
    insecure=True,
)

# Set up tracer provider
provider = TracerProvider()
processor = BatchSpanProcessor(otlp_exporter)
provider.add_span_processor(processor)
trace.set_tracer_provider(provider)
```

## Troubleshooting

### Tracing Not Working

1. Check if OpenTelemetry SDK is installed:
   ```python
   from data_bridge.postgres.telemetry import OTEL_AVAILABLE
   print(f"OpenTelemetry available: {OTEL_AVAILABLE}")
   ```

2. Check if tracing is enabled:
   ```python
   from data_bridge.postgres.telemetry import is_tracing_enabled
   print(f"Tracing enabled: {is_tracing_enabled()}")
   ```

3. Verify tracer provider is set up:
   ```python
   from opentelemetry import trace
   tracer_provider = trace.get_tracer_provider()
   print(f"Tracer provider: {tracer_provider}")
   ```

### No Spans Exported

Ensure you've set up a span processor:

```python
from opentelemetry.sdk.trace.export import ConsoleSpanExporter, SimpleSpanProcessor

# This will print spans to console
processor = SimpleSpanProcessor(ConsoleSpanExporter())
provider.add_span_processor(processor)
```

## API Reference

See the module docstrings for complete API documentation:

```python
from data_bridge.postgres import telemetry
help(telemetry)
```
