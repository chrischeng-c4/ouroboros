# OpenTelemetry Integration Guide

Comprehensive guide to distributed tracing and observability in data-bridge PostgreSQL ORM.

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture Overview](#architecture-overview)
3. [Quick Start](#quick-start)
4. [Installation](#installation)
5. [Configuration](#configuration)
6. [Instrumented Operations](#instrumented-operations)
7. [Span Attributes Reference](#span-attributes-reference)
8. [OTLP Backends](#otlp-backends)
9. [Performance Considerations](#performance-considerations)
10. [Best Practices](#best-practices)
11. [N+1 Query Detection](#n1-query-detection)
12. [Troubleshooting](#troubleshooting)
13. [Integration Examples](#integration-examples)
14. [API Reference](#api-reference)
15. [Future Enhancements](#future-enhancements)

---

## Introduction

### What is OpenTelemetry?

OpenTelemetry (OTel) is an open-source observability framework that provides:

- **Traces**: Track requests across distributed systems
- **Metrics**: Collect performance and resource utilization data
- **Logs**: Structured logging with correlation to traces
- **Vendor-Neutral**: Works with Jaeger, Grafana, DataDog, New Relic, and more

### Why Add OTel to data-bridge?

data-bridge includes built-in OpenTelemetry support to provide:

1. **Database Performance Insights**: See exactly how long queries take and what they do
2. **N+1 Query Detection**: Identify performance anti-patterns automatically
3. **Session Lifecycle Tracking**: Monitor transaction boundaries and state changes
4. **Relationship Loading Analysis**: Compare lazy vs eager loading strategies
5. **Production Debugging**: Trace requests end-to-end in distributed systems
6. **Cloud-Native Observability**: Integrate with modern observability platforms

### Cloud-Native Observability Benefits

- **Distributed Tracing**: Follow requests across microservices
- **Performance Profiling**: Identify bottlenecks in database operations
- **Error Tracking**: Capture exceptions with full context
- **SLO Monitoring**: Track service level objectives and latency percentiles
- **Cost Optimization**: Identify expensive queries for optimization

### When to Use Telemetry

**Use OpenTelemetry when:**
- Running in production with distributed services
- Debugging performance issues or N+1 queries
- Building cloud-native applications
- Need observability in microservices
- Using APM tools (DataDog, New Relic, etc.)

**Don't use telemetry when:**
- Local development (unless explicitly testing traces)
- Running benchmarks (adds ~1-2ms overhead per span)
- Privacy-sensitive environments (spans may contain query details)
- Resource-constrained environments (minimal overhead, but not zero)

**Note**: When disabled, telemetry has **zero overhead** due to fast-path optimization.

---

## Architecture Overview

### Trace Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     HTTP Request                            │
│                  (FastAPI, Flask, etc.)                     │
└─────────────────┬───────────────────────────────────────────┘
                  │ Trace ID: abc123
                  │ Span ID: def456
                  ▼
┌─────────────────────────────────────────────────────────────┐
│         HTTP Span (from FastAPI auto-instrumentation)       │
│         ┌─────────────────────────────────────────┐         │
│         │ http.method: GET                        │         │
│         │ http.route: /users/{user_id}            │         │
│         │ http.status_code: 200                   │         │
│         └─────────────────────────────────────────┘         │
└─────────────────┬───────────────────────────────────────────┘
                  │ Parent Span ID: def456
                  │ Child Span ID: ghi789
                  ▼
┌─────────────────────────────────────────────────────────────┐
│         data-bridge ORM Spans                               │
│                                                              │
│  ┌──────────────────────────────────────────────┐          │
│  │ db.query.find                                │          │
│  │ ├─ db.system: postgresql                     │          │
│  │ ├─ db.collection.name: users                 │          │
│  │ ├─ db.operation.name: find                   │          │
│  │ ├─ db.query.filters_count: 1                 │          │
│  │ └─ db.result.count: 1                        │          │
│  └──────────────────────────────────────────────┘          │
│           │                                                  │
│           ▼                                                  │
│  ┌──────────────────────────────────────────────┐          │
│  │ db.relationship.select (lazy loading)        │          │
│  │ ├─ db.relationship.name: posts               │          │
│  │ ├─ db.relationship.strategy: select          │          │
│  │ ├─ db.relationship.cache_hit: false          │          │
│  │ └─ db.result.count: 5                        │          │
│  └──────────────────────────────────────────────┘          │
│           │                                                  │
│           ▼                                                  │
│  ┌──────────────────────────────────────────────┐          │
│  │ db.session.flush                             │          │
│  │ ├─ db.session.pending_count: 3               │          │
│  │ ├─ db.session.dirty_count: 2                 │          │
│  │ └─ db.session.deleted_count: 0               │          │
│  └──────────────────────────────────────────────┘          │
│           │                                                  │
│           ▼                                                  │
│  ┌──────────────────────────────────────────────┐          │
│  │ db.session.commit                            │          │
│  └──────────────────────────────────────────────┘          │
│                                                              │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│         PostgreSQL Database                                 │
│         (Actual SQL execution)                              │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│         OTLP Exporter (gRPC or HTTP)                        │
│         - BatchSpanProcessor                                │
│         - Async export every 5s or 512 spans                │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│         Observability Backend                               │
│         Jaeger / Grafana / DataDog / etc.                   │
└─────────────────────────────────────────────────────────────┘
```

### Component Interaction

```
┌──────────────────┐
│ data-bridge ORM  │
└────────┬─────────┘
         │
         │ Uses telemetry module
         ▼
┌──────────────────────────────────────┐
│ telemetry.py                         │
│ ├─ get_tracer()                      │
│ ├─ create_query_span()               │
│ ├─ create_session_span()             │
│ ├─ create_relationship_span()        │
│ └─ ConnectionPoolMetrics             │
└────────┬─────────────────────────────┘
         │
         │ Calls OpenTelemetry API
         ▼
┌──────────────────────────────────────┐
│ OpenTelemetry SDK (optional)         │
│ ├─ TracerProvider                    │
│ ├─ MeterProvider                     │
│ └─ SpanProcessor                     │
└────────┬─────────────────────────────┘
         │
         │ Exports spans
         ▼
┌──────────────────────────────────────┐
│ OTLP Exporter (gRPC/HTTP)            │
│ ├─ Batch processing                  │
│ ├─ Compression (gzip)                │
│ └─ Retry logic                       │
└────────┬─────────────────────────────┘
         │
         │ Sends to backend
         ▼
┌──────────────────────────────────────┐
│ Backend (Jaeger/Grafana/etc.)        │
└──────────────────────────────────────┘
```

### What Gets Instrumented Automatically

When you use data-bridge ORM operations, spans are **automatically created** for:

1. **Queries**: `find()`, `count()`, `aggregate()`, `exists()`, `first()`
2. **Sessions**: `open()`, `close()`, `flush()`, `commit()`, `rollback()`
3. **Relationships**: Lazy loading (on-demand) and eager loading (batch)
4. **Errors**: Exceptions are automatically recorded with stack traces

**No code changes required** - just enable tracing via environment variable!

### Integration Points

data-bridge telemetry integrates at these layers:

```python
# Layer 1: Query execution (query.py)
query = session.find(User).filter(User.age > 18)
# → Creates: db.query.find span

# Layer 2: Session management (session.py)
await session.flush()
# → Creates: db.session.flush span

# Layer 3: Relationship loading (relationships.py, options.py)
user = await User.get(1)
posts = await user.posts  # Lazy load
# → Creates: db.relationship.select span

# Layer 4: Eager loading (options.py)
users = await session.find(User).options(selectinload(User.posts)).to_list()
# → Creates: db.relationship.selectinload span
```

---

## Quick Start

Get tracing working in **5 minutes**:

### Step 1: Install OpenTelemetry SDK

```bash
pip install opentelemetry-api opentelemetry-sdk \
            opentelemetry-exporter-otlp-proto-grpc
```

### Step 2: Configure OTLP Exporter

```python
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource

# Configure resource attributes
resource = Resource.create({
    "service.name": "my-api",
    "service.version": "1.0.0",
    "deployment.environment": "production",
})

# Set up tracer provider
provider = TracerProvider(resource=resource)

# Configure OTLP exporter (to Jaeger, Grafana, etc.)
otlp_exporter = OTLPSpanExporter(
    endpoint="http://localhost:4317",  # OTLP gRPC endpoint
    insecure=True,  # Use TLS in production
)

# Add batch processor for async export
processor = BatchSpanProcessor(otlp_exporter)
provider.add_span_processor(processor)

# Set as global tracer provider
trace.set_tracer_provider(provider)
```

### Step 3: Enable data-bridge Tracing

```bash
export DATA_BRIDGE_TRACING_ENABLED=true
```

### Step 4: Run Your Application

```python
from data_bridge.postgres import Session, Table, Column

class User(Table):
    id: int = Column(primary_key=True)
    name: str

# Normal ORM usage - tracing happens automatically!
async def main():
    session = Session("postgresql://localhost/mydb")

    # This query creates a span automatically
    users = await session.find(User).to_list()

    print(f"Found {len(users)} users")

    await session.close()
```

### Step 5: View Traces

Open your observability backend:

- **Jaeger**: http://localhost:16686
- **Grafana Cloud**: Your Grafana instance
- **DataDog**: app.datadoghq.com/apm/traces

**That's it!** Your database operations are now traced.

---

## Installation

### Required Packages

**Minimum (API only)**:
```bash
pip install opentelemetry-api
```

This allows data-bridge to import OpenTelemetry types, but **no spans will be exported** without the SDK.

**Recommended (SDK + OTLP Exporter)**:
```bash
pip install opentelemetry-api \
            opentelemetry-sdk \
            opentelemetry-exporter-otlp-proto-grpc
```

### Optional Dependencies

**For specific backends**:

```bash
# Jaeger (deprecated, use OTLP instead)
pip install opentelemetry-exporter-jaeger

# Console output (debugging)
# (included in opentelemetry-sdk)

# HTTP/JSON exporter (alternative to gRPC)
pip install opentelemetry-exporter-otlp-proto-http

# Prometheus metrics
pip install opentelemetry-exporter-prometheus

# FastAPI auto-instrumentation
pip install opentelemetry-instrumentation-fastapi
```

### Version Compatibility

| Package | Minimum Version | Tested Version |
|---------|----------------|----------------|
| `opentelemetry-api` | 1.20.0 | 1.28.0 |
| `opentelemetry-sdk` | 1.20.0 | 1.28.0 |
| `opentelemetry-exporter-otlp-proto-grpc` | 1.20.0 | 1.28.0 |
| `data-bridge` | 0.1.0 | latest |
| Python | 3.12+ | 3.12 |

### Installation Verification

```python
# Check if OpenTelemetry is available
from data_bridge.postgres.telemetry import OTEL_AVAILABLE, is_tracing_enabled

print(f"OpenTelemetry SDK available: {OTEL_AVAILABLE}")
print(f"Tracing enabled: {is_tracing_enabled()}")
```

Expected output:
```
OpenTelemetry SDK available: True
Tracing enabled: True
```

---

## Configuration

### Environment Variables

data-bridge uses standard OpenTelemetry environment variables plus one custom variable:

#### data-bridge Specific

```bash
# Enable/disable tracing (default: true if SDK installed)
export DATA_BRIDGE_TRACING_ENABLED=true

# Disable tracing
export DATA_BRIDGE_TRACING_ENABLED=false
# Or: export DATA_BRIDGE_TRACING_ENABLED=0
# Or: export DATA_BRIDGE_TRACING_ENABLED=no
```

#### Standard OpenTelemetry Variables

```bash
# OTLP exporter endpoint (gRPC)
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Use insecure connection (no TLS)
export OTEL_EXPORTER_OTLP_INSECURE=true

# Authentication headers (for cloud backends)
export OTEL_EXPORTER_OTLP_HEADERS="api-key=your-key-here"

# Service identification
export OTEL_SERVICE_NAME=my-api
export OTEL_SERVICE_VERSION=1.0.0
export OTEL_RESOURCE_ATTRIBUTES="deployment.environment=production,team=backend"

# Sampling configuration
export OTEL_TRACES_SAMPLER=traceidratio
export OTEL_TRACES_SAMPLER_ARG=0.1  # Sample 10% of traces

# Span limits
export OTEL_SPAN_ATTRIBUTE_VALUE_LENGTH_LIMIT=4096
export OTEL_SPAN_ATTRIBUTE_COUNT_LIMIT=128
```

### Programmatic Configuration

#### Resource Attributes

Define service metadata attached to all spans:

```python
from opentelemetry.sdk.resources import Resource

resource = Resource.create({
    # Service identification (recommended)
    "service.name": "user-api",
    "service.version": "2.3.1",
    "service.namespace": "production",

    # Deployment info
    "deployment.environment": "production",  # development, staging, production

    # Infrastructure
    "host.name": "app-server-01",
    "host.type": "ec2",
    "cloud.provider": "aws",
    "cloud.region": "us-east-1",

    # Team/ownership
    "team": "backend-team",
    "owner": "john@example.com",

    # Custom attributes
    "app.feature.flags": "feature-x-enabled",
})
```

#### OTLP Exporter Configuration

```python
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter

# Basic configuration
exporter = OTLPSpanExporter(
    endpoint="http://localhost:4317",
    insecure=True,  # Set to False for TLS in production
)

# Advanced configuration
exporter = OTLPSpanExporter(
    endpoint="https://otlp.example.com:4317",
    insecure=False,  # Use TLS
    headers={
        "api-key": "your-api-key",
        "tenant-id": "abc123",
    },
    timeout=10,  # Request timeout in seconds
    compression="gzip",  # Compress payloads
)
```

#### BatchSpanProcessor Settings

Optimize span export performance:

```python
from opentelemetry.sdk.trace.export import BatchSpanProcessor

processor = BatchSpanProcessor(
    exporter,

    # How often to export (milliseconds)
    schedule_delay_millis=5000,  # Export every 5 seconds

    # Max batch size before forcing export
    max_export_batch_size=512,   # Export when 512 spans collected

    # Max queue size (drop spans if exceeded)
    max_queue_size=2048,

    # Export timeout
    export_timeout_millis=30000,  # 30 seconds
)
```

**Production recommendations**:
- `schedule_delay_millis`: 5000-10000 (balance latency vs overhead)
- `max_export_batch_size`: 512-1024 (reduce network calls)
- `max_queue_size`: 2048-4096 (prevent memory issues)

#### Sampling Configuration

Reduce overhead by sampling a subset of traces:

```python
from opentelemetry.sdk.trace.sampling import (
    TraceIdRatioBased,  # Sample by ratio
    ParentBased,        # Follow parent decision
    ALWAYS_ON,          # Sample everything
    ALWAYS_OFF,         # Sample nothing
)

# Sample 10% of traces (recommended for high-traffic production)
sampler = TraceIdRatioBased(0.1)

# Always sample (development)
sampler = ALWAYS_ON

# Never sample (disable tracing via SDK)
sampler = ALWAYS_OFF

# Parent-based (follow parent span's sampling decision)
sampler = ParentBased(root=TraceIdRatioBased(0.1))

# Use sampler in tracer provider
provider = TracerProvider(sampler=sampler, resource=resource)
```

**Sampling strategies**:
- **Development**: `ALWAYS_ON` (100%)
- **Low-traffic production**: `TraceIdRatioBased(1.0)` (100%)
- **Medium-traffic production**: `TraceIdRatioBased(0.5)` (50%)
- **High-traffic production**: `TraceIdRatioBased(0.1)` (10%)
- **Very high-traffic**: `TraceIdRatioBased(0.01)` (1%)

### Complete Configuration Example

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

    # 1. Define resource attributes
    resource = Resource.create({
        "service.name": os.getenv("SERVICE_NAME", "my-api"),
        "service.version": os.getenv("SERVICE_VERSION", "1.0.0"),
        "deployment.environment": os.getenv("ENVIRONMENT", "production"),
    })

    # 2. Configure sampler (10% sampling for production)
    sampler = ParentBased(root=TraceIdRatioBased(0.1))

    # 3. Create tracer provider
    provider = TracerProvider(
        resource=resource,
        sampler=sampler,
    )

    # 4. Configure OTLP exporter
    exporter = OTLPSpanExporter(
        endpoint=os.getenv("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317"),
        insecure=os.getenv("OTEL_EXPORTER_OTLP_INSECURE", "true") == "true",
    )

    # 5. Add batch processor
    processor = BatchSpanProcessor(
        exporter,
        schedule_delay_millis=5000,
        max_export_batch_size=512,
    )
    provider.add_span_processor(processor)

    # 6. Set as global provider
    trace.set_tracer_provider(provider)

    print(f"OpenTelemetry configured: {resource.attributes}")

# Call this at application startup
configure_telemetry()
```

---

## Instrumented Operations

### Queries

All query operations automatically create spans with detailed attributes.

#### `find()`

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

#### `count()`

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

#### `first()`

```python
user = await session.find(User).filter(User.email == "alice@example.com").first()
```

**Span created**:
```
Name: db.query.find
Attributes:
  - db.system: postgresql
  - db.operation.name: find
  - db.collection.name: users
  - db.query.filters_count: 1
  - db.query.limit: 1
  - db.result.count: 1
```

#### `exists()`

```python
exists = await session.find(User).filter(User.id == 123).exists()
```

**Span created**:
```
Name: db.query.exists
Attributes:
  - db.system: postgresql
  - db.operation.name: exists
  - db.collection.name: users
  - db.query.filters_count: 1
  - db.result.count: 1  # (0 or 1)
```

#### `aggregate()`

```python
results = await session.find(User).aggregate([
    {"$group": {"_id": "$country", "count": {"$sum": 1}}}
])
```

**Span created**:
```
Name: db.query.aggregate
Attributes:
  - db.system: postgresql
  - db.operation.name: aggregate
  - db.collection.name: users
  - db.result.count: 5  # Number of aggregation results
```

### Sessions

Session operations track transaction lifecycle and state changes.

#### `open()` / Session Context Manager

```python
async with Session("postgresql://localhost/mydb") as session:
    # Session operations
    pass
```

**Span created**:
```
Name: db.session.open
Attributes:
  - db.system: postgresql
  - db.session.operation: open
```

**On exit**:
```
Name: db.session.close
Attributes:
  - db.system: postgresql
  - db.session.operation: close
```

#### `flush()`

```python
user1 = User(name="Alice", age=30)
user2 = User(name="Bob", age=35)

session.add(user1)
session.add(user2)

await session.flush()  # Persist pending changes
```

**Span created**:
```
Name: db.session.flush
Attributes:
  - db.system: postgresql
  - db.session.operation: flush
  - db.session.pending_count: 2  # New objects to insert
  - db.session.dirty_count: 0    # Modified objects
  - db.session.deleted_count: 0  # Objects to delete
```

#### `commit()`

```python
user = await User.get(1)
user.age = 31
await session.commit()  # Flush + commit transaction
```

**Span created**:
```
Name: db.session.commit
Attributes:
  - db.system: postgresql
  - db.session.operation: commit
  - db.session.pending_count: 0
  - db.session.dirty_count: 1  # Modified object
  - db.session.deleted_count: 0
```

#### `rollback()`

```python
try:
    user.age = -1  # Invalid
    await session.commit()
except Exception:
    await session.rollback()  # Undo changes
```

**Span created**:
```
Name: db.session.rollback
Attributes:
  - db.system: postgresql
  - db.session.operation: rollback
```

### Relationships

Relationship loading is automatically instrumented to help detect N+1 queries.

#### Lazy Loading (`select` strategy)

**Individual load per access**:

```python
# Load user
user = await User.get(1)

# Access relationship (triggers lazy load)
posts = await user.posts  # Span created here
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
  - db.relationship.cache_hit: false  # Not in session cache
  - db.result.count: 5  # 5 posts loaded
```

**Cache hit example**:

```python
user1 = await User.get(1)
posts1 = await user1.posts  # Loads from database

# Later in same session
user1_again = await User.get(1)  # From identity map
posts1_again = await user1_again.posts  # From cache
```

**Span created (cache hit)**:
```
Name: db.relationship.select
Attributes:
  - db.relationship.name: posts
  - db.relationship.cache_hit: true  # Served from cache!
  - db.result.count: 5
```

#### Eager Loading (`selectinload` strategy)

**Batch load for multiple instances**:

```python
from data_bridge.postgres import selectinload

# Load users with posts in one batch query
users = await session.find(User).options(
    selectinload(User.posts)
).to_list()

# Access posts - no query needed, already loaded
for user in users:
    posts = await user.posts  # No span, already in memory
```

**Span created**:
```
Name: db.relationship.selectinload
Attributes:
  - db.system: postgresql
  - db.relationship.name: posts
  - db.relationship.target_model: Post
  - db.relationship.strategy: selectinload
  - db.relationship.fk_column: author_id
  - db.relationship.batch_count: 100  # Loaded for 100 users
  - db.result.count: 250  # 250 total posts loaded
```

**Depth tracking** (nested eager loading):

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
   - db.relationship.batch_count: 100

2. db.relationship.selectinload (comments)
   - db.relationship.depth: 1  # Nested relationship
   - db.relationship.batch_count: 250
```

---

## Span Attributes Reference

Complete reference of all span attributes used by data-bridge.

### Standard OpenTelemetry Attributes

Following [OpenTelemetry Semantic Conventions for Database Calls](https://opentelemetry.io/docs/specs/semconv/database/):

| Attribute | Type | Description | Example |
|-----------|------|-------------|---------|
| `db.system` | string | Database system (always "postgresql") | `postgresql` |
| `db.operation.name` | string | Operation type | `find`, `insert`, `update`, `delete` |
| `db.collection.name` | string | Table name | `users`, `posts` |
| `db.statement` | string | SQL statement (optional, use sparingly) | `SELECT * FROM users WHERE age > $1` |

### Query Attributes

| Attribute | Type | Description | Example | Cardinality |
|-----------|------|-------------|---------|-------------|
| `db.query.filters_count` | int | Number of WHERE conditions | `3` | Low (0-100) |
| `db.query.limit` | int | LIMIT clause value | `10` | Low (common values) |
| `db.query.offset` | int | OFFSET clause value | `20` | Medium (pagination) |
| `db.query.order_by` | string | ORDER BY clause | `created_at DESC` | Medium |

**Cardinality note**: Avoid high-cardinality values (e.g., unique IDs) in attributes.

### Result Attributes

| Attribute | Type | Description | Example |
|-----------|------|-------------|---------|
| `db.result.count` | int | Number of rows returned | `42` |
| `db.result.affected_rows` | int | Number of rows modified | `5` |

### Session Attributes

| Attribute | Type | Description | Example |
|-----------|------|-------------|---------|
| `db.session.operation` | string | Session operation | `flush`, `commit`, `rollback` |
| `db.session.pending_count` | int | New objects to insert | `3` |
| `db.session.dirty_count` | int | Modified objects to update | `2` |
| `db.session.deleted_count` | int | Objects to delete | `1` |

### Relationship Attributes

| Attribute | Type | Description | Example |
|-----------|------|-------------|---------|
| `db.relationship.name` | string | Relationship attribute name | `posts`, `author` |
| `db.relationship.target_model` | string | Target model class name | `Post`, `User` |
| `db.relationship.strategy` | string | Loading strategy | `select`, `selectinload`, `joined` |
| `db.relationship.fk_column` | string | Foreign key column | `author_id` |
| `db.relationship.cache_hit` | bool | Loaded from session cache | `true`, `false` |
| `db.relationship.batch_count` | int | Instances in batch load (eager only) | `100` |
| `db.relationship.depth` | int | Nesting level (0 = top-level) | `0`, `1`, `2` |

### Error Attributes

When exceptions occur, spans automatically include:

| Attribute | Type | Description |
|-----------|------|-------------|
| `exception.type` | string | Exception class name |
| `exception.message` | string | Exception message |
| `exception.stacktrace` | string | Full stack trace |
| `otel.status_code` | string | `ERROR` when exception occurs |

### Complete Example Span

```json
{
  "trace_id": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "span_id": "q7r8s9t0u1v2",
  "parent_span_id": "w3x4y5z6a7b8",
  "name": "db.query.find",
  "kind": "INTERNAL",
  "start_time": "2026-01-06T10:30:00.123Z",
  "end_time": "2026-01-06T10:30:00.145Z",
  "duration_ms": 22,
  "attributes": {
    "db.system": "postgresql",
    "db.operation.name": "find",
    "db.collection.name": "users",
    "db.query.filters_count": 2,
    "db.query.limit": 10,
    "db.query.offset": 0,
    "db.query.order_by": "created_at DESC",
    "db.result.count": 10
  },
  "status": {
    "code": "OK"
  }
}
```

---

## OTLP Backends

data-bridge supports any OpenTelemetry-compatible backend via OTLP (OpenTelemetry Protocol).

### Jaeger (Local Development)

**Best for**: Local development, testing traces

**Setup**:
```bash
docker run -d --name jaeger \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 16686:16686 \
  -p 4317:4317 \
  -p 4318:4318 \
  jaegertracing/all-in-one:latest
```

**Configuration**:
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_EXPORTER_OTLP_INSECURE=true
```

**Python**:
```python
exporter = OTLPSpanExporter(
    endpoint="http://localhost:4317",
    insecure=True,
)
```

**View traces**: http://localhost:16686

---

### Grafana Cloud (SaaS)

**Best for**: Production, integrated observability stack

**Setup**:
1. Sign up at https://grafana.com/
2. Get credentials from: Connections → OpenTelemetry → Send Traces
3. Format: `instance_id:api_token`

**Configuration**:
```bash
# Encode credentials
CREDENTIALS=$(echo -n 'instance_id:api_token' | base64)

export OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp-gateway-prod-us-central-0.grafana.net/otlp
export OTEL_EXPORTER_OTLP_INSECURE=false
export OTEL_EXPORTER_OTLP_HEADERS="Authorization=Basic ${CREDENTIALS}"
```

**Python**:
```python
import base64

credentials = base64.b64encode(b"instance_id:api_token").decode()

exporter = OTLPSpanExporter(
    endpoint="https://otlp-gateway-prod-us-central-0.grafana.net/otlp",
    insecure=False,
    headers={"Authorization": f"Basic {credentials}"},
)
```

**View traces**: Your Grafana instance → Explore → Tempo

---

### DataDog APM

**Best for**: Production APM, full observability suite

**Setup**:
1. Get API key from: https://app.datadoghq.com/organization-settings/api-keys
2. Install DataDog agent with OTLP support

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

**Python**:
```python
exporter = OTLPSpanExporter(
    endpoint="http://localhost:4317",
    insecure=True,
)

# Add DataDog-specific resource attributes
resource = Resource.create({
    "service.name": "my-api",
    "deployment.environment": "production",
    "service.version": "1.0.0",
})
```

**View traces**: https://app.datadoghq.com/apm/traces

---

### New Relic

**Best for**: Production APM, alerting

**Setup**:
1. Get license key from: https://one.newrelic.com/api-keys

**Configuration**:
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp.nr-data.net:4317
export OTEL_EXPORTER_OTLP_INSECURE=false
export OTEL_EXPORTER_OTLP_HEADERS="api-key=<your-license-key>"
```

**Python**:
```python
exporter = OTLPSpanExporter(
    endpoint="https://otlp.nr-data.net:4317",
    insecure=False,
    headers={"api-key": "your-license-key"},
)
```

**View traces**: https://one.newrelic.com/distributed-tracing

---

### Honeycomb.io

**Best for**: Deep trace analysis, query-based exploration

**Setup**:
1. Get API key from: https://ui.honeycomb.io/account

**Configuration**:
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=https://api.honeycomb.io:443
export OTEL_EXPORTER_OTLP_INSECURE=false
export OTEL_EXPORTER_OTLP_HEADERS="x-honeycomb-team=<your-api-key>"
```

**Python**:
```python
exporter = OTLPSpanExporter(
    endpoint="https://api.honeycomb.io:443",
    insecure=False,
    headers={"x-honeycomb-team": "your-api-key"},
)
```

**View traces**: https://ui.honeycomb.io/

---

### AWS X-Ray (via ADOT Collector)

**Best for**: AWS environments, integrated AWS observability

**Setup**:
1. Deploy AWS Distro for OpenTelemetry (ADOT) collector
2. Configure collector to export to X-Ray

**Configuration**:
```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_EXPORTER_OTLP_INSECURE=true
export AWS_REGION=us-east-1
```

**Python**: Same as Jaeger (ADOT collector handles X-Ray export)

**View traces**: AWS Console → X-Ray → Traces

---

### Environment Variable Template

Copy this to your `.env` file:

```bash
# Service identification
SERVICE_NAME=my-api
SERVICE_VERSION=1.0.0
DEPLOYMENT_ENVIRONMENT=production

# OTLP configuration (choose one backend)
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_EXPORTER_OTLP_INSECURE=true
# OTEL_EXPORTER_OTLP_HEADERS=api-key=your-key-here

# data-bridge tracing
DATA_BRIDGE_TRACING_ENABLED=true

# Sampling (optional)
OTEL_TRACES_SAMPLER=traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1  # 10% sampling
```

---

## Performance Considerations

### Overhead When Enabled vs Disabled

| Scenario | Overhead | Notes |
|----------|----------|-------|
| **Tracing disabled** | **0ms** | Fast-path check, no span creation |
| **Tracing enabled, no export** | ~0.5ms per span | Span creation only |
| **Tracing enabled, with export** | ~1-2ms per span | Includes serialization + async export |
| **Batch export** | Amortized <0.5ms | Export every 5s or 512 spans |

**Key insight**: When `DATA_BRIDGE_TRACING_ENABLED=false`, there is **zero overhead** due to fast-path optimization:

```python
def create_query_span(...):
    # Fast path: immediate return if disabled
    tracer = get_tracer()
    if tracer is None:
        yield None
        return

    # Instrumented path only runs if tracing enabled
    with tracer.start_as_current_span(...) as span:
        yield span
```

### Sampling Strategies

**Recommendation by traffic volume**:

| Traffic | Sampling Rate | Config |
|---------|--------------|--------|
| Development | 100% | `ALWAYS_ON` |
| <100 req/min | 100% | `TraceIdRatioBased(1.0)` |
| <1000 req/min | 50% | `TraceIdRatioBased(0.5)` |
| <10k req/min | 10% | `TraceIdRatioBased(0.1)` |
| 10k+ req/min | 1-5% | `TraceIdRatioBased(0.01)` |

**Head-based vs Tail-based sampling**:

- **Head-based** (used by data-bridge): Decision made at trace creation
  - Pros: Low overhead, simple configuration
  - Cons: May miss interesting traces (e.g., errors)

- **Tail-based** (requires collector): Decision after trace completes
  - Pros: Can sample based on duration, errors, attributes
  - Cons: Higher overhead, complex setup

**Recommendation**: Start with head-based sampling, add tail-based for advanced use cases.

### Batch Exporting Configuration

Optimize span export to reduce network overhead:

```python
processor = BatchSpanProcessor(
    exporter,

    # Export every 5 seconds (balance latency vs overhead)
    schedule_delay_millis=5000,

    # Or when 512 spans collected (reduce network calls)
    max_export_batch_size=512,

    # Max queue size (prevent memory issues)
    max_queue_size=2048,
)
```

**Impact**:

| Setting | Network Calls/sec | Memory Usage | Latency |
|---------|-------------------|--------------|---------|
| `schedule_delay_millis=1000` | 1/sec | Low | 1s |
| `schedule_delay_millis=5000` | 0.2/sec | Medium | 5s |
| `schedule_delay_millis=10000` | 0.1/sec | Higher | 10s |
| `max_export_batch_size=512` | Variable | Low | Variable |

**Production recommendation**:
- `schedule_delay_millis`: 5000 (5 seconds)
- `max_export_batch_size`: 512
- `max_queue_size`: 2048

### Fast-Path Optimization

data-bridge uses fast-path checks to minimize overhead when tracing is disabled:

```python
# Before any span logic
if not is_tracing_enabled():
    # Original logic without instrumentation
    return await _execute_query()

# Instrumented path (only if tracing enabled)
with create_query_span(...) as span:
    result = await _execute_query()
    set_span_result(span, count=len(result))
    return result
```

**Benchmark**:

| Configuration | Query latency | Overhead |
|---------------|---------------|----------|
| No telemetry code | 10.0ms | - |
| Telemetry disabled | 10.0ms | 0ms |
| Telemetry enabled | 11.5ms | 1.5ms |

### Cardinality Considerations

**Low-cardinality span names** (good):
```python
# Span name: db.query.find (low cardinality)
with create_query_span("find", table="users"):
    ...
```

**High-cardinality span names** (bad):
```python
# DON'T: Include unique values in span names
span_name = f"db.query.find.user.{user_id}"  # ❌ High cardinality
```

**High-cardinality attributes**:

Use attributes for unique values, but be aware of limits:

```python
# OK: Attributes can have high cardinality
with create_query_span("find", table="users") as span:
    span.set_attribute("user.id", user_id)  # ✅ OK in attributes
```

**Limits**:
- Default attribute value length: 4096 characters
- Default attribute count: 128 per span

Configure limits:
```bash
export OTEL_SPAN_ATTRIBUTE_VALUE_LENGTH_LIMIT=4096
export OTEL_SPAN_ATTRIBUTE_COUNT_LIMIT=128
```

---

## Best Practices

### Development vs Production Configuration

**Development**:
```python
# Simple console exporter
from opentelemetry.sdk.trace.export import ConsoleSpanExporter, SimpleSpanProcessor

provider = TracerProvider()
provider.add_span_processor(SimpleSpanProcessor(ConsoleSpanExporter()))
trace.set_tracer_provider(provider)

# Or use Jaeger locally
exporter = OTLPSpanExporter(endpoint="http://localhost:4317", insecure=True)
provider.add_span_processor(BatchSpanProcessor(exporter))
```

**Production**:
```python
# OTLP exporter with sampling and batching
from opentelemetry.sdk.trace.sampling import ParentBased, TraceIdRatioBased

resource = Resource.create({
    "service.name": os.getenv("SERVICE_NAME"),
    "service.version": os.getenv("SERVICE_VERSION"),
    "deployment.environment": "production",
})

sampler = ParentBased(root=TraceIdRatioBased(0.1))  # 10% sampling

provider = TracerProvider(resource=resource, sampler=sampler)

exporter = OTLPSpanExporter(
    endpoint=os.getenv("OTEL_EXPORTER_OTLP_ENDPOINT"),
    insecure=False,  # Use TLS
)

processor = BatchSpanProcessor(
    exporter,
    schedule_delay_millis=5000,
    max_export_batch_size=512,
)

provider.add_span_processor(processor)
trace.set_tracer_provider(provider)
```

### Sampling Rates

| Environment | Rate | Reason |
|-------------|------|--------|
| Development | 100% | See all traces |
| Staging | 50% | Balance cost and coverage |
| Production (low traffic) | 100% | Full visibility |
| Production (high traffic) | 10% | Reduce overhead and cost |
| Production (very high) | 1% | Minimize impact |

**Adjust based on**:
- Traffic volume
- Backend costs (some charge per span)
- Performance requirements
- Debugging needs

### Resource Attribute Naming

**Good practices**:

```python
resource = Resource.create({
    # Standard attributes (follow semantic conventions)
    "service.name": "user-api",
    "service.version": "2.3.1",
    "deployment.environment": "production",

    # Namespace custom attributes
    "app.team": "backend",
    "app.feature.flags": "feature-x",

    # Use lowercase with dots
    "db.connection.pool.size": "10",
})
```

**Avoid**:
```python
# ❌ Bad: Inconsistent naming
"ServiceName": "user-api"  # Use lowercase
"team_name": "backend"     # Use dots, not underscores
"feature_x_enabled": "true"  # No namespace
```

### Span Attribute Limits

**Default limits**:
- Attribute value length: 4096 characters
- Attribute count: 128 per span
- Event count: 128 per span

**Configure**:
```bash
export OTEL_SPAN_ATTRIBUTE_VALUE_LENGTH_LIMIT=4096
export OTEL_SPAN_ATTRIBUTE_COUNT_LIMIT=128
export OTEL_SPAN_EVENT_COUNT_LIMIT=128
```

**Best practices**:
- Don't include large payloads in attributes (e.g., full request bodies)
- Truncate long strings (e.g., SQL statements)
- Use events for detailed information

**Example**:
```python
with create_query_span("find", table="users") as span:
    # ✅ Good: Short, meaningful attributes
    span.set_attribute("user.role", "admin")

    # ❌ Bad: Large payload
    # span.set_attribute("request.body", json.dumps(large_dict))

    # ✅ Better: Use events for details
    span.add_event("query_details", {
        "filters": str(filters)[:100],  # Truncate
    })
```

### Security Considerations (PII in Spans)

**Never include**:
- Passwords
- Credit card numbers
- Social security numbers
- API keys/tokens
- Personal identifiable information (PII)

**Avoid including**:
- Email addresses (use hashed IDs instead)
- Full names (use user IDs)
- Phone numbers
- IP addresses (unless necessary)

**Example**:
```python
# ❌ Bad: Includes PII
with create_query_span("find", table="users") as span:
    span.set_attribute("user.email", "alice@example.com")  # PII
    span.set_attribute("user.password", password)  # Never!

# ✅ Good: Use IDs instead
with create_query_span("find", table="users") as span:
    span.set_attribute("user.id", 12345)  # Safe
    span.set_attribute("user.role", "admin")  # Safe
```

**SQL statement sanitization**:

data-bridge uses parameterized queries, so SQL statements in spans are safe:

```python
# Safe: Parameters not included
db.statement: "SELECT * FROM users WHERE email = $1"
# $1 is a placeholder, actual email not exposed
```

**Disable statement logging** (if needed):
```python
# Don't set db.statement attribute
with create_query_span("find", table="users"):
    # statement parameter omitted
    ...
```

### Graceful Degradation

data-bridge handles missing OpenTelemetry SDK gracefully:

```python
# Without OpenTelemetry SDK installed
from data_bridge.postgres import Session, User

session = Session("postgresql://localhost/mydb")
users = await session.find(User).to_list()  # Works fine, no tracing
```

**No errors, no crashes** - telemetry is optional!

**Check availability**:
```python
from data_bridge.postgres.telemetry import OTEL_AVAILABLE, is_tracing_enabled

if OTEL_AVAILABLE:
    print("OpenTelemetry SDK installed")
else:
    print("OpenTelemetry SDK not available (install opentelemetry-sdk)")

if is_tracing_enabled():
    print("Tracing is active")
else:
    print("Tracing is disabled")
```

---

## N+1 Query Detection

### How to Identify N+1 Queries in Traces

**N+1 pattern**: 1 query to fetch parent records + N queries to fetch related records

**Example trace (N+1 problem)**:

```
GET /posts
├─ db.query.find (posts) [1 query]
│  └─ db.result.count: 10
│
├─ db.relationship.select (author) [Query 1]
│  └─ db.relationship.cache_hit: false
│
├─ db.relationship.select (author) [Query 2]
│  └─ db.relationship.cache_hit: false
│
├─ db.relationship.select (author) [Query 3]
│  └─ db.relationship.cache_hit: false
│
... (7 more relationship spans)
│
└─ db.relationship.select (author) [Query 10]
   └─ db.relationship.cache_hit: false

Total: 11 queries (1 + 10)
```

**Indicators of N+1**:
1. Multiple `db.relationship.select` spans with same name
2. `db.relationship.cache_hit: false` for each
3. Number of relationship spans ≈ number of parent records

### Using Span Counts to Detect Patterns

**In Jaeger UI**:

1. Search for traces with high span counts:
   - Filter: `min_spans > 10`
   - Look for repeated span names

2. Check relationship spans:
   - Count `db.relationship.select` spans
   - If count = parent record count → N+1 pattern

**Programmatic detection** (custom span processor):

```python
class N1Detector:
    def on_end(self, span):
        # Count relationship spans
        if span.name == "db.relationship.select":
            relationship_name = span.attributes.get("db.relationship.name")
            cache_hit = span.attributes.get("db.relationship.cache_hit")

            if not cache_hit:
                # Log potential N+1
                print(f"Potential N+1: {relationship_name}")
```

### Eager Loading as Solution

**Before (N+1)**:

```python
# Lazy loading: 1 + N queries
posts = await session.find(Post).to_list()  # 1 query

for post in posts:
    author = await post.author  # N queries (1 per post)
    print(f"Post by {author.name}")
```

**Trace**:
```
db.query.find (posts)
├─ db.relationship.select (author) × N times
```

**After (Eager loading)**:

```python
from data_bridge.postgres import selectinload

# Eager loading: 2 queries total
posts = await session.find(Post).options(
    selectinload(Post.author)
).to_list()

for post in posts:
    author = await post.author  # Already loaded, no query
    print(f"Post by {author.name}")
```

**Trace**:
```
db.query.find (posts)
└─ db.relationship.selectinload (author) × 1 time
   └─ db.relationship.batch_count: 10
```

### Before/After Trace Examples

**Before (N+1 with 10 posts)**:

```
Trace: GET /posts
Duration: 120ms
Spans: 12

db.query.find (posts)               [10ms]
├─ db.result.count: 10
│
db.relationship.select (author #1)  [10ms]
db.relationship.select (author #2)  [10ms]
db.relationship.select (author #3)  [11ms]
db.relationship.select (author #4)  [10ms]
db.relationship.select (author #5)  [11ms]
db.relationship.select (author #6)  [10ms]
db.relationship.select (author #7)  [10ms]
db.relationship.select (author #8)  [11ms]
db.relationship.select (author #9)  [10ms]
db.relationship.select (author #10) [10ms]

Total: 11 database queries
Duration: 120ms
```

**After (Eager loading with selectinload)**:

```
Trace: GET /posts
Duration: 25ms
Spans: 3

db.query.find (posts)               [12ms]
├─ db.result.count: 10
│
db.relationship.selectinload (author)  [10ms]
├─ db.relationship.batch_count: 10
└─ db.result.count: 5  # 5 unique authors

Total: 2 database queries
Duration: 25ms
Improvement: 4.8x faster, 5.5x fewer queries
```

**Key improvements**:
- Queries: 11 → 2 (5.5x reduction)
- Duration: 120ms → 25ms (4.8x faster)
- Spans: 12 → 3 (4x fewer)

### Automatic N+1 Detection (Future)

**Planned feature**: Emit warning spans when N+1 threshold exceeded

```python
# Future: Automatic detection
posts = await session.find(Post).to_list()

for post in posts:
    author = await post.author  # After N accesses, warning span emitted

# Warning span:
# Name: db.n1_warning
# Attributes:
#   - db.relationship.name: author
#   - db.n1.query_count: 10
#   - db.n1.recommendation: "Use selectinload(Post.author)"
```

---

## Troubleshooting

### Common Issues and Solutions

#### Issue: "No traces appearing in backend"

**Symptoms**:
- Application runs fine
- No traces in Jaeger/Grafana/etc.

**Debugging steps**:

1. **Check if OpenTelemetry SDK is installed**:
   ```python
   from data_bridge.postgres.telemetry import OTEL_AVAILABLE
   print(f"OpenTelemetry available: {OTEL_AVAILABLE}")
   ```

   If `False`: Install SDK
   ```bash
   pip install opentelemetry-api opentelemetry-sdk
   ```

2. **Check if tracing is enabled**:
   ```python
   from data_bridge.postgres.telemetry import is_tracing_enabled
   print(f"Tracing enabled: {is_tracing_enabled()}")
   ```

   If `False`: Enable tracing
   ```bash
   export DATA_BRIDGE_TRACING_ENABLED=true
   ```

3. **Verify tracer provider is set**:
   ```python
   from opentelemetry import trace
   provider = trace.get_tracer_provider()
   print(f"Tracer provider: {provider}")
   ```

   If `<class 'opentelemetry.trace.NoOpTracerProvider'>`: Set up tracer provider
   ```python
   from opentelemetry.sdk.trace import TracerProvider
   trace.set_tracer_provider(TracerProvider())
   ```

4. **Check span processor is configured**:
   ```python
   from opentelemetry import trace
   provider = trace.get_tracer_provider()
   print(f"Span processors: {provider._active_span_processor}")
   ```

   If empty: Add span processor
   ```python
   from opentelemetry.sdk.trace.export import BatchSpanProcessor
   provider.add_span_processor(BatchSpanProcessor(exporter))
   ```

#### Issue: "Connection errors to OTLP collector"

**Symptoms**:
```
ERROR: Failed to export spans to http://localhost:4317
ConnectionError: [Errno 111] Connection refused
```

**Solutions**:

1. **Check collector is running**:
   ```bash
   # For Jaeger
   docker ps | grep jaeger

   # If not running
   docker start jaeger
   ```

2. **Verify endpoint**:
   ```bash
   echo $OTEL_EXPORTER_OTLP_ENDPOINT
   # Should match collector address
   ```

3. **Check port**:
   - gRPC: 4317 (default)
   - HTTP: 4318

   ```bash
   # Test connection
   nc -zv localhost 4317
   ```

4. **Check firewall/network**:
   ```bash
   # In Docker network
   docker network inspect bridge
   ```

#### Issue: "Performance degradation"

**Symptoms**:
- Application slower than expected
- High CPU usage

**Debugging**:

1. **Check sampling rate**:
   ```python
   from opentelemetry import trace
   provider = trace.get_tracer_provider()
   print(f"Sampler: {provider.sampler}")
   ```

   If `ALWAYS_ON`: Consider sampling
   ```python
   from opentelemetry.sdk.trace.sampling import TraceIdRatioBased
   sampler = TraceIdRatioBased(0.1)  # 10% sampling
   ```

2. **Check batch processor settings**:
   ```python
   # Increase batch size to reduce export frequency
   processor = BatchSpanProcessor(
       exporter,
       schedule_delay_millis=10000,  # Export every 10s
       max_export_batch_size=1024,   # Larger batches
   )
   ```

3. **Disable tracing temporarily**:
   ```bash
   export DATA_BRIDGE_TRACING_ENABLED=false
   ```

4. **Profile span creation**:
   ```python
   import time
   start = time.time()
   with create_query_span("find", "users") as span:
       pass
   print(f"Span overhead: {(time.time() - start) * 1000}ms")
   ```

#### Issue: "Missing spans"

**Symptoms**:
- Some operations don't create spans
- Incomplete traces

**Solutions**:

1. **Check fast-path optimization**:
   ```python
   # If tracer is None, spans won't be created
   from data_bridge.postgres.telemetry import get_tracer
   tracer = get_tracer()
   print(f"Tracer: {tracer}")  # Should not be None
   ```

2. **Verify span context propagation**:
   ```python
   # Check if parent span exists
   from opentelemetry import trace
   current_span = trace.get_current_span()
   print(f"Current span: {current_span}")
   ```

3. **Enable debug logging**:
   ```python
   import logging
   logging.basicConfig(level=logging.DEBUG)
   logging.getLogger("opentelemetry").setLevel(logging.DEBUG)
   ```

#### Issue: "High cardinality warnings"

**Symptoms**:
```
WARNING: High cardinality detected in span name: db.query.find.user.12345
```

**Solution**:

Don't include unique values in span names:

```python
# ❌ Bad
span_name = f"db.query.find.user.{user_id}"

# ✅ Good
span_name = "db.query.find"
span.set_attribute("user.id", user_id)  # Use attributes instead
```

### Debugging Tips

#### Enable Verbose Logging

```python
import logging

# Enable OpenTelemetry debug logs
logging.basicConfig(level=logging.DEBUG)
logging.getLogger("opentelemetry").setLevel(logging.DEBUG)

# Enable data-bridge telemetry logs
logging.getLogger("data_bridge.postgres.telemetry").setLevel(logging.DEBUG)
```

#### Console Exporter (Development)

See spans immediately in console:

```python
from opentelemetry.sdk.trace.export import ConsoleSpanExporter, SimpleSpanProcessor

provider = TracerProvider()
provider.add_span_processor(SimpleSpanProcessor(ConsoleSpanExporter()))
trace.set_tracer_provider(provider)

# Spans will print to console
```

**Output**:
```json
{
    "name": "db.query.find",
    "context": {
        "trace_id": "0x...",
        "span_id": "0x...",
        "trace_state": "[]"
    },
    "kind": "SpanKind.INTERNAL",
    "parent_id": "0x...",
    "start_time": "2026-01-06T10:30:00.123Z",
    "end_time": "2026-01-06T10:30:00.145Z",
    "status": {
        "status_code": "OK"
    },
    "attributes": {
        "db.system": "postgresql",
        "db.operation.name": "find",
        "db.collection.name": "users",
        "db.result.count": 10
    }
}
```

#### Verify Span Export

```python
# Add custom processor to count exported spans
class CountingProcessor:
    def __init__(self):
        self.count = 0

    def on_start(self, span, parent_context):
        pass

    def on_end(self, span):
        self.count += 1
        print(f"Span exported: {span.name} (total: {self.count})")

    def shutdown(self):
        pass

    def force_flush(self, timeout_millis=None):
        pass

counter = CountingProcessor()
provider.add_span_processor(counter)
```

#### Test OTLP Connectivity

```bash
# Test gRPC endpoint
grpcurl -plaintext localhost:4317 list

# Test HTTP endpoint
curl -X POST http://localhost:4318/v1/traces \
  -H "Content-Type: application/json" \
  -d '{"resourceSpans": []}'
```

---

## Integration Examples

### FastAPI Integration

See complete example: [`examples/fastapi_otel_example.py`](../examples/fastapi_otel_example.py)

**Quick setup**:

```python
from fastapi import FastAPI
from opentelemetry import trace
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from data_bridge.postgres import Session, Table, Column

# Configure OpenTelemetry
provider = TracerProvider()
exporter = OTLPSpanExporter(endpoint="http://localhost:4317", insecure=True)
provider.add_span_processor(BatchSpanProcessor(exporter))
trace.set_tracer_provider(provider)

# Create FastAPI app
app = FastAPI()

# Auto-instrument FastAPI
FastAPIInstrumentor.instrument_app(app)

# Define model
class User(Table):
    id: int = Column(primary_key=True)
    name: str

    class Settings:
        table_name = "users"

# Database session
session = Session("postgresql://localhost/mydb")

@app.get("/users/{user_id}")
async def get_user(user_id: int):
    # Automatically creates trace:
    # HTTP span (from FastAPI) → ORM span (from data-bridge)
    user = await session.get(User, user_id)
    return user
```

**Trace structure**:
```
GET /users/1
├─ FastAPI span
│  ├─ http.method: GET
│  ├─ http.route: /users/{user_id}
│  └─ http.status_code: 200
│
└─ data-bridge span
   ├─ db.query.find
   ├─ db.collection.name: users
   └─ db.result.count: 1
```

**See also**:
- [FastAPI Quick Start](../examples/QUICKSTART_FASTAPI_OTEL.md)
- [Full FastAPI Example](../examples/README_FASTAPI_OTEL.md)
- [Trace Examples](../examples/TRACE_EXAMPLES.md)

### Standalone Scripts

```python
import asyncio
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import ConsoleSpanExporter, SimpleSpanProcessor
from data_bridge.postgres import Session, Table, Column

# Configure tracing
provider = TracerProvider()
provider.add_span_processor(SimpleSpanProcessor(ConsoleSpanExporter()))
trace.set_tracer_provider(provider)

# Define model
class User(Table):
    id: int = Column(primary_key=True)
    name: str

    class Settings:
        table_name = "users"

async def main():
    session = Session("postgresql://localhost/mydb")

    # Traced query
    users = await session.find(User).to_list()
    print(f"Found {len(users)} users")

    await session.close()

asyncio.run(main())
```

**Output** (console exporter):
```json
{
    "name": "db.query.find",
    "attributes": {
        "db.system": "postgresql",
        "db.operation.name": "find",
        "db.collection.name": "users",
        "db.result.count": 5
    },
    "start_time": "...",
    "end_time": "..."
}
```

### Pytest Integration (Future)

**Planned feature**: Pytest plugin for trace collection during tests

```python
# Future: pytest-opentelemetry plugin
pytest --trace-export=jaeger tests/

# Or programmatic in conftest.py
@pytest.fixture(scope="session", autouse=True)
def configure_tracing():
    from opentelemetry import trace
    from opentelemetry.sdk.trace import TracerProvider

    provider = TracerProvider()
    # ... configure exporter
    trace.set_tracer_provider(provider)
```

---

## API Reference

### `telemetry` Module

Complete API documentation for `data_bridge.postgres.telemetry`.

#### Configuration Functions

##### `is_tracing_enabled() -> bool`

Check if tracing is currently enabled.

**Returns**: `True` if OpenTelemetry SDK is installed and tracing is not explicitly disabled.

**Example**:
```python
from data_bridge.postgres.telemetry import is_tracing_enabled

if is_tracing_enabled():
    print("Tracing is active")
```

##### `get_tracer() -> Optional[Tracer]`

Get the global tracer for data-bridge.

**Returns**: `Tracer` instance if available, `None` otherwise.

**Example**:
```python
from data_bridge.postgres.telemetry import get_tracer

tracer = get_tracer()
if tracer:
    with tracer.start_as_current_span("custom.operation"):
        # Custom span
        pass
```

##### `get_meter() -> Optional[Meter]`

Get the global meter for metrics.

**Returns**: `Meter` instance if available, `None` otherwise.

**Example**:
```python
from data_bridge.postgres.telemetry import get_meter

meter = get_meter()
if meter:
    counter = meter.create_counter("custom.counter")
    counter.add(1)
```

#### Span Context Managers

##### `create_query_span(...)`

Create a query span with database attributes.

**Signature**:
```python
@contextmanager
def create_query_span(
    operation: str,
    table: Optional[str] = None,
    filters_count: Optional[int] = None,
    limit: Optional[int] = None,
    offset: Optional[int] = None,
    order_by: Optional[str] = None,
    statement: Optional[str] = None,
    **attributes: Any
) -> Iterator[Optional[Span]]
```

**Parameters**:
- `operation`: Operation type (e.g., "find", "insert", "update")
- `table`: Table name
- `filters_count`: Number of WHERE conditions
- `limit`: LIMIT value
- `offset`: OFFSET value
- `order_by`: ORDER BY clause
- `statement`: SQL statement (use cautiously)
- `**attributes`: Additional custom attributes

**Example**:
```python
from data_bridge.postgres.telemetry import create_query_span, set_span_result

with create_query_span("find", "users", filters_count=2, limit=10) as span:
    result = await execute_query()
    set_span_result(span, count=len(result))
```

##### `create_session_span(...)`

Create a session span with state attributes.

**Signature**:
```python
@contextmanager
def create_session_span(
    operation: str,
    pending_count: Optional[int] = None,
    dirty_count: Optional[int] = None,
    deleted_count: Optional[int] = None,
    **attributes: Any
) -> Iterator[Optional[Span]]
```

**Parameters**:
- `operation`: Session operation (e.g., "flush", "commit", "rollback")
- `pending_count`: Number of pending (new) objects
- `dirty_count`: Number of modified objects
- `deleted_count`: Number of deleted objects
- `**attributes`: Additional custom attributes

**Example**:
```python
from data_bridge.postgres.telemetry import create_session_span

with create_session_span("flush", pending_count=5, dirty_count=3) as span:
    await session.flush()
```

##### `create_relationship_span(...)`

Create a relationship loading span.

**Signature**:
```python
@contextmanager
def create_relationship_span(
    name: str,
    target_model: Optional[str] = None,
    strategy: Optional[str] = None,
    fk_column: Optional[str] = None,
    batch_count: Optional[int] = None,
    depth: int = 0,
    **attributes: Any
) -> Iterator[Optional[Span]]
```

**Parameters**:
- `name`: Relationship name
- `target_model`: Target model class name
- `strategy`: Loading strategy ("select", "selectinload", etc.)
- `fk_column`: Foreign key column
- `batch_count`: Number of instances in batch (eager loading)
- `depth`: Nesting depth (0 = top-level)
- `**attributes`: Additional custom attributes

**Example**:
```python
from data_bridge.postgres.telemetry import create_relationship_span, set_span_result

with create_relationship_span("posts", target_model="Post", strategy="select") as span:
    posts = await load_relationship()
    set_span_result(span, count=len(posts), cache_hit=False)
```

#### Helper Functions

##### `add_exception(span, exception)`

Record an exception in a span.

**Signature**:
```python
def add_exception(span: Optional[Span], exception: Exception) -> None
```

**Parameters**:
- `span`: Span to add exception to
- `exception`: Exception to record

**Example**:
```python
from data_bridge.postgres.telemetry import create_query_span, add_exception

with create_query_span("find", "users") as span:
    try:
        result = await query()
    except Exception as e:
        add_exception(span, e)
        raise
```

##### `set_span_result(...)`

Set result attributes on a span.

**Signature**:
```python
def set_span_result(
    span: Optional[Span],
    count: Optional[int] = None,
    affected_rows: Optional[int] = None,
    cache_hit: Optional[bool] = None,
    **kwargs: Any
) -> None
```

**Parameters**:
- `span`: Span to update
- `count`: Number of results
- `affected_rows`: Number of rows modified
- `cache_hit`: Whether result from cache
- `**kwargs`: Additional custom attributes

**Example**:
```python
from data_bridge.postgres.telemetry import set_span_result

with create_query_span("find", "users") as span:
    result = await query()
    set_span_result(span, count=len(result))
```

#### Decorators

##### `@instrument_span(...)`

Decorate a function to create a span.

**Signature**:
```python
def instrument_span(
    name: Optional[str] = None,
    attributes: Optional[Dict[str, Any]] = None
) -> Callable[[F], F]
```

**Parameters**:
- `name`: Span name (defaults to `module.function`)
- `attributes`: Additional attributes

**Example**:
```python
from data_bridge.postgres.telemetry import instrument_span

@instrument_span("custom.operation", attributes={"component": "business-logic"})
async def process_data(data):
    # Function body
    pass
```

##### `@instrument_query(...)`

Decorate a query function.

**Signature**:
```python
def instrument_query(operation: str) -> Callable[[F], F]
```

**Parameters**:
- `operation`: Operation type (e.g., "find", "insert")

**Example**:
```python
from data_bridge.postgres.telemetry import instrument_query

@instrument_query("find")
async def find_users(filters):
    return await User.find(filters).to_list()
```

##### `@instrument_session(...)`

Decorate a session function.

**Signature**:
```python
def instrument_session(operation: str) -> Callable[[F], F]
```

**Parameters**:
- `operation`: Session operation (e.g., "flush", "commit")

**Example**:
```python
from data_bridge.postgres.telemetry import instrument_session

@instrument_session("flush")
async def flush_changes(session):
    await session.flush()
```

#### Metrics

##### `ConnectionPoolMetrics`

Connection pool metrics collector.

**Methods**:

```python
class ConnectionPoolMetrics:
    def record_pool_stats(
        self,
        in_use: int,
        idle: int,
        max_size: int
    ) -> None:
        """Record connection pool statistics."""
```

**Example**:
```python
from data_bridge.postgres.telemetry import get_connection_pool_metrics

metrics = get_connection_pool_metrics()
metrics.record_pool_stats(in_use=5, idle=3, max_size=10)
```

#### Constants

##### `SpanAttributes`

Semantic convention constants:

```python
class SpanAttributes:
    DB_SYSTEM = "db.system"
    DB_OPERATION_NAME = "db.operation.name"
    DB_COLLECTION_NAME = "db.collection.name"
    DB_STATEMENT = "db.statement"
    DB_QUERY_FILTERS_COUNT = "db.query.filters_count"
    DB_QUERY_LIMIT = "db.query.limit"
    DB_QUERY_OFFSET = "db.query.offset"
    DB_QUERY_ORDER_BY = "db.query.order_by"
    DB_RESULT_COUNT = "db.result.count"
    DB_RESULT_AFFECTED_ROWS = "db.result.affected_rows"
    DB_SESSION_OPERATION = "db.session.operation"
    DB_SESSION_PENDING_COUNT = "db.session.pending_count"
    DB_SESSION_DIRTY_COUNT = "db.session.dirty_count"
    DB_SESSION_DELETED_COUNT = "db.session.deleted_count"
    DB_RELATIONSHIP_NAME = "db.relationship.name"
    DB_RELATIONSHIP_TARGET_MODEL = "db.relationship.target_model"
    DB_RELATIONSHIP_STRATEGY = "db.relationship.strategy"
    DB_RELATIONSHIP_FK_COLUMN = "db.relationship.fk_column"
    DB_RELATIONSHIP_CACHE_HIT = "db.relationship.cache_hit"
    DB_RELATIONSHIP_BATCH_COUNT = "db.relationship.batch_count"
    DB_RELATIONSHIP_DEPTH = "db.relationship.depth"
```

##### `MetricNames`

Metric name constants:

```python
class MetricNames:
    CONNECTION_POOL_IN_USE = "db.connection.pool.in_use"
    CONNECTION_POOL_IDLE = "db.connection.pool.idle"
    CONNECTION_POOL_MAX = "db.connection.pool.max"
    QUERY_DURATION = "db.query.duration"
    QUERY_COUNT = "db.query.count"
```

---

## Future Enhancements

### Planned Features

#### 1. Automatic N+1 Detection Warnings

Emit warning spans when N+1 threshold exceeded:

```python
# Automatic detection (planned)
posts = await session.find(Post).to_list()

for post in posts:
    author = await post.author  # After 10 accesses, warning emitted

# Warning span (future):
# Name: db.n1_warning
# Attributes:
#   - db.relationship.name: author
#   - db.n1.query_count: 10
#   - db.n1.threshold: 5
#   - db.n1.recommendation: "Use selectinload(Post.author)"
```

**Configuration**:
```python
# Future API
from data_bridge.postgres.telemetry import configure_n1_detection

configure_n1_detection(
    enabled=True,
    threshold=5,  # Warn after 5 lazy loads
    log_level="WARNING",
)
```

#### 2. Metrics Export (Beyond Connection Pool)

Export query performance metrics:

```python
# Future metrics (planned)
meter = get_meter()

# Query duration histogram
query_duration = meter.create_histogram(
    "db.query.duration",
    description="Query execution time in milliseconds",
    unit="ms"
)

# Query count counter
query_count = meter.create_counter(
    "db.query.count",
    description="Total number of queries executed"
)

# Automatically recorded for all queries
```

**Exported metrics**:
- `db.query.duration` (histogram) - Query latency distribution
- `db.query.count` (counter) - Total queries by operation
- `db.connection.pool.in_use` (gauge) - Active connections
- `db.connection.pool.idle` (gauge) - Idle connections
- `db.session.transaction.duration` (histogram) - Transaction time

#### 3. Custom Span Processors

Support custom processors for advanced use cases:

```python
# Future API (planned)
from data_bridge.postgres.telemetry import add_span_processor

class CustomProcessor:
    def on_start(self, span, parent_context):
        # Custom logic on span start
        pass

    def on_end(self, span):
        # Custom logic on span end (e.g., log slow queries)
        if span.name == "db.query.find":
            duration_ms = (span.end_time - span.start_time) / 1e6
            if duration_ms > 100:
                print(f"Slow query detected: {duration_ms}ms")

add_span_processor(CustomProcessor())
```

#### 4. Span Links for Async Operations

Link related async operations:

```python
# Future: Span links for background tasks
async def process_user(user_id):
    # Main operation span
    with create_query_span("find", "users") as main_span:
        user = await get_user(user_id)

        # Background task span (linked to main)
        asyncio.create_task(
            send_welcome_email(user, parent_span=main_span)
        )

# Creates span link:
# send_welcome_email span → linked to → process_user span
```

#### 5. Sampling Based on Attributes

Advanced sampling strategies:

```python
# Future: Attribute-based sampling (planned)
from opentelemetry.sdk.trace.sampling import AttributeBasedSampler

sampler = AttributeBasedSampler(
    # Always sample slow queries
    rules=[
        {"duration_ms": {">": 100}, "sample": 1.0},
        {"db.operation.name": "update", "sample": 0.5},
        {"default": 0.1},
    ]
)
```

#### 6. Query Plan Export

Export EXPLAIN output as span events:

```python
# Future: Query plan export (planned)
with create_query_span("find", "users") as span:
    result = await query()

    # Automatically adds query plan as event
    # span.add_event("query_plan", {
    #     "plan": "Seq Scan on users (cost=0.00..10.00 rows=100 width=32)"
    # })
```

### Community Contributions Welcome

We welcome contributions! Areas for improvement:

1. **Additional backends**: Azure Monitor, Google Cloud Trace examples
2. **Metrics exporters**: Prometheus, StatsD
3. **Logging integration**: Correlate logs with traces
4. **Sampling strategies**: Custom samplers for specific use cases
5. **Documentation**: More examples, tutorials, best practices

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

---

## References

### Official Documentation

- [OpenTelemetry Official Site](https://opentelemetry.io/)
- [OpenTelemetry Python Documentation](https://opentelemetry.io/docs/instrumentation/python/)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
- [Database Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/database/)

### data-bridge Documentation

- [PostgreSQL ORM Design](./postgres_orm_design.md)
- [PostgreSQL Transactions](./postgres_transactions.md)
- [PostgreSQL Extensions](./POSTGRESQL_EXTENSIONS.md)
- [FastAPI Example](../examples/README_FASTAPI_OTEL.md)
- [Trace Examples](../examples/TRACE_EXAMPLES.md)
- [Relationship Telemetry](../TELEMETRY_RELATIONSHIPS.md)

### Backend-Specific Docs

- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Grafana Tempo](https://grafana.com/docs/tempo/)
- [DataDog APM](https://docs.datadoghq.com/tracing/)
- [New Relic Distributed Tracing](https://docs.newrelic.com/docs/distributed-tracing/)
- [Honeycomb](https://docs.honeycomb.io/)

### Code Examples

- [Standalone Example](../examples/telemetry_example.py)
- [FastAPI Example](../examples/fastapi_otel_example.py)
- [Backend Configuration](../examples/otel_backends.env.example)
- [Tests](../tests/postgres/test_telemetry.py)

---

## Support

### Getting Help

- **GitHub Issues**: https://github.com/your-org/data-bridge/issues
- **Discussions**: https://github.com/your-org/data-bridge/discussions
- **Email**: support@your-org.com

### Reporting Bugs

When reporting telemetry-related issues, include:

1. OpenTelemetry SDK version
2. data-bridge version
3. Backend (Jaeger, Grafana, etc.)
4. Configuration (environment variables)
5. Minimal reproduction example
6. Error messages/logs

**Example bug report**:
```markdown
## Description
Spans not appearing in Jaeger

## Environment
- data-bridge: 0.1.0
- opentelemetry-sdk: 1.28.0
- Backend: Jaeger 1.50.0

## Configuration
export DATA_BRIDGE_TRACING_ENABLED=true
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

## Steps to Reproduce
1. Configure telemetry as in quickstart
2. Run query: session.find(User).to_list()
3. Check Jaeger UI - no traces

## Expected Behavior
Traces appear in Jaeger

## Actual Behavior
No traces exported
```

---

**Last Updated**: 2026-01-06

**Version**: 1.0.0

**License**: MIT
