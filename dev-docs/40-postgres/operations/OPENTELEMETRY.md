# OpenTelemetry Integration

**Component**: PostgreSQL Operations
**Feature**: Distributed Tracing and Observability
**Status**: Implemented
**Last Updated**: 2026-01-06

---

## Introduction

data-bridge PostgreSQL ORM includes built-in OpenTelemetry (OTel) support for distributed tracing and observability. This enables automatic instrumentation of database operations, session management, and relationship loading with **zero code changes**.

### Why OpenTelemetry?

- **Database Performance Insights**: See exactly how long queries take
- **N+1 Query Detection**: Identify performance anti-patterns automatically
- **Session Lifecycle Tracking**: Monitor transaction boundaries
- **Relationship Loading Analysis**: Compare lazy vs eager loading strategies
- **Production Debugging**: Trace requests end-to-end in distributed systems
- **Cloud-Native Observability**: Works with Jaeger, Grafana, DataDog, New Relic, etc.

### When to Use

**Use OpenTelemetry when:**
- Running in production with distributed services
- Debugging performance issues or N+1 queries
- Building cloud-native applications
- Need observability in microservices
- Using APM tools (DataDog, New Relic, etc.)

**Don't use when:**
- Local development (unless testing traces)
- Running benchmarks (adds ~1-2ms overhead per span)
- Privacy-sensitive environments (spans may contain query details)

**Note**: When disabled, telemetry has **zero overhead** due to fast-path optimization.

---

## Quick Start (5 Minutes)

### 1. Install OpenTelemetry SDK

```bash
pip install opentelemetry-api opentelemetry-sdk \
            opentelemetry-exporter-otlp-proto-grpc
```

### 2. Start Jaeger (Local Testing)

```bash
docker run -d --name jaeger \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest
```

### 3. Configure Tracing

```python
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource

# Configure resource
resource = Resource.create({
    "service.name": "my-api",
    "service.version": "1.0.0",
    "deployment.environment": "production",
})

# Set up tracer provider
provider = TracerProvider(resource=resource)
exporter = OTLPSpanExporter(
    endpoint="http://localhost:4317",
    insecure=True,
)
provider.add_span_processor(BatchSpanProcessor(exporter))
trace.set_tracer_provider(provider)
```

### 4. Enable data-bridge Tracing

```bash
export DATA_BRIDGE_TRACING_ENABLED=true
```

### 5. Run Your Application

```python
from data_bridge.postgres import Session, Table, Column

class User(Table):
    id: int = Column(primary_key=True)
    name: str

async def main():
    session = Session("postgresql://localhost/mydb")

    # This query creates a span automatically!
    users = await session.find(User).to_list()

    await session.close()
```

### 6. View Traces

Open Jaeger UI: http://localhost:16686

You should see spans for database operations!

---

## Configuration

### Environment Variables

```bash
# Enable/disable tracing (default: true if SDK installed)
export DATA_BRIDGE_TRACING_ENABLED=true

# OTLP exporter endpoint (gRPC)
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_EXPORTER_OTLP_INSECURE=true

# Service identification
export OTEL_SERVICE_NAME=my-api
export OTEL_SERVICE_VERSION=1.0.0
export OTEL_RESOURCE_ATTRIBUTES="deployment.environment=production"

# Sampling (optional - reduce overhead in high-traffic)
export OTEL_TRACES_SAMPLER=traceidratio
export OTEL_TRACES_SAMPLER_ARG=0.1  # Sample 10% of traces
```

### Sampling Strategies

| Environment | Rate | Reason |
|-------------|------|--------|
| Development | 100% | See all traces |
| Staging | 50% | Balance cost and coverage |
| Production (low traffic) | 100% | Full visibility |
| Production (high traffic) | 10% | Reduce overhead and cost |
| Production (very high) | 1% | Minimize impact |

### Production Configuration Example

```python
import os
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.sdk.trace.sampling import TraceIdRatioBased, ParentBased
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource

def configure_telemetry():
    """Configure OpenTelemetry for production."""

    # Resource attributes
    resource = Resource.create({
        "service.name": os.getenv("SERVICE_NAME", "my-api"),
        "service.version": os.getenv("SERVICE_VERSION", "1.0.0"),
        "deployment.environment": os.getenv("ENVIRONMENT", "production"),
    })

    # 10% sampling for high-traffic production
    sampler = ParentBased(root=TraceIdRatioBased(0.1))

    # Tracer provider
    provider = TracerProvider(resource=resource, sampler=sampler)

    # OTLP exporter
    exporter = OTLPSpanExporter(
        endpoint=os.getenv("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317"),
        insecure=os.getenv("OTEL_EXPORTER_OTLP_INSECURE", "true") == "true",
    )

    # Batch processor (export every 5s or 512 spans)
    processor = BatchSpanProcessor(
        exporter,
        schedule_delay_millis=5000,
        max_export_batch_size=512,
    )
    provider.add_span_processor(processor)

    trace.set_tracer_provider(provider)

# Call at startup
configure_telemetry()
```

---

## PostgreSQL Instrumentation

### Automatically Instrumented Operations

data-bridge automatically creates spans for:

1. **Queries**: `find()`, `count()`, `aggregate()`, `exists()`, `first()`
2. **Sessions**: `open()`, `close()`, `flush()`, `commit()`, `rollback()`
3. **Relationships**: Lazy loading and eager loading
4. **Errors**: Exceptions with full stack traces

**No code changes required!**

### Query Operations

#### Find Query

```python
users = await session.find(User).filter(User.age > 18).limit(10).to_list()
```

**Span created**:
```
Name: db.query.find
Attributes:
  - db.system: postgresql
  - db.operation.name: find
  - db.collection.name: users
  - db.query.filters_count: 1
  - db.query.limit: 10
  - db.result.count: 10
```

#### Count Query

```python
count = await session.find(User).filter(User.active == True).count()
```

**Span created**:
```
Name: db.query.count
Attributes:
  - db.system: postgresql
  - db.operation.name: count
  - db.collection.name: users
  - db.query.filters_count: 1
  - db.result.count: 42
```

### Session Operations

#### Flush

```python
user1 = User(name="Alice", age=30)
user2 = User(name="Bob", age=35)
session.add(user1)
session.add(user2)
await session.flush()
```

**Span created**:
```
Name: db.session.flush
Attributes:
  - db.system: postgresql
  - db.session.operation: flush
  - db.session.pending_count: 2  # New objects
  - db.session.dirty_count: 0    # Modified objects
  - db.session.deleted_count: 0  # Deleted objects
```

#### Commit

```python
user = await User.get(1)
user.age = 31
await session.commit()
```

**Span created**:
```
Name: db.session.commit
Attributes:
  - db.system: postgresql
  - db.session.operation: commit
  - db.session.pending_count: 0
  - db.session.dirty_count: 1
  - db.session.deleted_count: 0
```

### Relationship Loading

#### Lazy Loading (N+1 Pattern)

```python
# Load user
user = await User.get(1)

# Access relationship (triggers query)
posts = await user.posts
```

**Span created**:
```
Name: db.relationship.select
Attributes:
  - db.system: postgresql
  - db.relationship.name: posts
  - db.relationship.target_model: Post
  - db.relationship.strategy: select
  - db.relationship.fk_column: author_id
  - db.relationship.cache_hit: false
  - db.result.count: 5
```

#### Eager Loading (Batch)

```python
from data_bridge.postgres import selectinload

users = await session.find(User).options(
    selectinload(User.posts)
).to_list()
```

**Span created**:
```
Name: db.relationship.selectinload
Attributes:
  - db.system: postgresql
  - db.relationship.name: posts
  - db.relationship.target_model: Post
  - db.relationship.strategy: selectinload
  - db.relationship.batch_count: 100
  - db.result.count: 250  # Total posts loaded
```

---

## Common Patterns

### N+1 Query Detection

**Problem**: 1 query to fetch parent records + N queries for related records

**Before (N+1)**:
```python
# Lazy loading: 1 + N queries
posts = await session.find(Post).to_list()  # 1 query

for post in posts:
    author = await post.author  # N queries
    print(f"Post by {author.name}")
```

**Trace shows**:
```
db.query.find (posts) [10ms]
└─ db.result.count: 10

db.relationship.select (author) [10ms] × 10 times
└─ db.relationship.cache_hit: false
```

**After (Eager Loading)**:
```python
from data_bridge.postgres import selectinload

posts = await session.find(Post).options(
    selectinload(Post.author)
).to_list()

for post in posts:
    author = await post.author  # Already loaded
    print(f"Post by {author.name}")
```

**Trace shows**:
```
db.query.find (posts) [12ms]
└─ db.result.count: 10

db.relationship.selectinload (author) [10ms]
└─ db.relationship.batch_count: 10
```

**Improvement**: 11 queries → 2 queries (5.5x reduction)

### Nested Eager Loading

```python
users = await session.find(User).options(
    selectinload(User.posts).options(
        selectinload(Post.comments)
    )
).to_list()
```

**Spans created**:
```
1. db.relationship.selectinload (posts)
   - db.relationship.depth: 0

2. db.relationship.selectinload (comments)
   - db.relationship.depth: 1  # Nested
```

---

## OTLP Backends

### Jaeger (Local Development)

```bash
docker run -d --name jaeger \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest
```

**Configuration**:
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_EXPORTER_OTLP_INSECURE=true
```

**View traces**: http://localhost:16686

### Grafana Cloud (Production)

1. Sign up at https://grafana.com/
2. Get credentials: Connections → OpenTelemetry → Send Traces

**Configuration**:
```bash
CREDENTIALS=$(echo -n 'instance_id:api_token' | base64)

export OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp-gateway-prod-us-central-0.grafana.net/otlp
export OTEL_EXPORTER_OTLP_INSECURE=false
export OTEL_EXPORTER_OTLP_HEADERS="Authorization=Basic ${CREDENTIALS}"
```

### DataDog APM

```bash
docker run -d --name datadog-agent \
  -e DD_API_KEY=<your-api-key> \
  -e DD_SITE=datadoghq.com \
  -e DD_OTLP_CONFIG_RECEIVER_PROTOCOLS_GRPC_ENDPOINT=0.0.0.0:4317 \
  -p 4317:4317 \
  gcr.io/datadoghq/agent:latest
```

**Configuration**:
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_EXPORTER_OTLP_INSECURE=true
export DD_SERVICE=my-api
export DD_ENV=production
export DD_VERSION=1.0.0
```

**View traces**: https://app.datadoghq.com/apm/traces

### Environment Variable Template

```bash
# Service identification
SERVICE_NAME=my-api
SERVICE_VERSION=1.0.0
DEPLOYMENT_ENVIRONMENT=production

# OTLP configuration
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_EXPORTER_OTLP_INSECURE=true
# OTEL_EXPORTER_OTLP_HEADERS=api-key=your-key

# data-bridge tracing
DATA_BRIDGE_TRACING_ENABLED=true

# Sampling (optional)
OTEL_TRACES_SAMPLER=traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1  # 10%
```

---

## Troubleshooting

### Issue: No traces appearing

**Check OpenTelemetry SDK is installed**:
```python
from data_bridge.postgres.telemetry import OTEL_AVAILABLE
print(f"OpenTelemetry available: {OTEL_AVAILABLE}")
```

If `False`:
```bash
pip install opentelemetry-api opentelemetry-sdk
```

**Check tracing is enabled**:
```python
from data_bridge.postgres.telemetry import is_tracing_enabled
print(f"Tracing enabled: {is_tracing_enabled()}")
```

If `False`:
```bash
export DATA_BRIDGE_TRACING_ENABLED=true
```

**Verify tracer provider is set**:
```python
from opentelemetry import trace
provider = trace.get_tracer_provider()
print(f"Provider: {provider}")
```

Should not be `NoOpTracerProvider`.

### Issue: Connection errors to OTLP collector

**Check collector is running**:
```bash
docker ps | grep jaeger
```

**Verify endpoint**:
```bash
echo $OTEL_EXPORTER_OTLP_ENDPOINT
```

**Test connection**:
```bash
nc -zv localhost 4317
```

### Issue: Performance degradation

**Check sampling rate**:
```python
from opentelemetry import trace
provider = trace.get_tracer_provider()
print(f"Sampler: {provider.sampler}")
```

**Increase batch size**:
```python
processor = BatchSpanProcessor(
    exporter,
    schedule_delay_millis=10000,  # Export every 10s
    max_export_batch_size=1024,   # Larger batches
)
```

**Disable tracing temporarily**:
```bash
export DATA_BRIDGE_TRACING_ENABLED=false
```

### Debug Logging

```python
import logging

# Enable OpenTelemetry debug logs
logging.basicConfig(level=logging.DEBUG)
logging.getLogger("opentelemetry").setLevel(logging.DEBUG)
logging.getLogger("data_bridge.postgres.telemetry").setLevel(logging.DEBUG)
```

### Console Exporter (Development)

```python
from opentelemetry.sdk.trace.export import ConsoleSpanExporter, SimpleSpanProcessor

provider = TracerProvider()
provider.add_span_processor(SimpleSpanProcessor(ConsoleSpanExporter()))
trace.set_tracer_provider(provider)

# Spans will print to console
```

---

## See Also

- **Comprehensive Guide**: `/docs/OPENTELEMETRY.md` - Full documentation with API reference
- **FastAPI Integration**: `/examples/fastapi_otel_example.py` - Complete example
- **Logging**: `./LOGGING.md` - Structured logging with tracing
- **PostgreSQL ORM**: `/docs/postgres_orm_design.md` - ORM architecture
- **Relationships**: `/docs/postgres/relationships.md` - Relationship patterns

---

**Last Updated**: 2026-01-06
