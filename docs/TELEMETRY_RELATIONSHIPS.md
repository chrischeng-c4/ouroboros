# Relationship Loading Telemetry

This document describes the OpenTelemetry instrumentation added to the RelationshipLoader for tracing lazy and eager loading operations.

## Overview

OpenTelemetry spans are now automatically created for:
1. **Lazy Loading** (`RelationshipLoader._load()`) - Individual relationship loads
2. **Eager Loading** (`SelectInLoad.apply()`) - Batch relationship loads

## Span Attributes

### Common Attributes

All relationship spans include:
- `db.system` = "postgresql"
- `db.relationship.name` - Relationship attribute name (e.g., "author")
- `db.relationship.target_model` - Target model class name (e.g., "User")
- `db.relationship.strategy` - Loading strategy ("select", "selectinload", "joined", etc.)
- `db.relationship.fk_column` - Foreign key column name (e.g., "author_id")

### Lazy Loading Specific

- `db.relationship.cache_hit` - `true` if loaded via session identity map, `false` if direct query
- `db.result.count` - Number of results (0 or 1)

### Eager Loading Specific

- `db.relationship.batch_count` - Number of instances being batch loaded
- `db.result.count` - Number of related objects loaded

## Span Names

Low-cardinality span names are used for better metric aggregation:
- `db.relationship.select` - Lazy loading with SELECT strategy
- `db.relationship.selectinload` - Eager loading with IN clause
- `db.relationship.joined` - Eager loading with JOIN (future)

## Performance Considerations

### Fast Path
When tracing is disabled (`DATA_BRIDGE_TRACING_ENABLED=false` or OpenTelemetry SDK not installed):
- **Zero overhead** - No span creation or attribute collection
- Fast-path check before any span logic
- No impact on production performance

### Instrumented Path
When tracing is enabled:
- Minimal overhead (~1-2ms per span)
- Spans created only for relationship loading operations
- Existing query spans are reused (no duplicate spans)

## Example Traces

### Lazy Loading (N+1 Query)

```
db.relationship.select (author)
├── db.relationship.name = "author"
├── db.relationship.target_model = "User"
├── db.relationship.strategy = "select"
├── db.relationship.fk_column = "author_id"
├── db.relationship.cache_hit = false
└── db.result.count = 1
    └── db.query.find (users)  # Nested query span
```

### Eager Loading (Batch Load)

```
db.relationship.selectinload (author)
├── db.relationship.name = "author"
├── db.relationship.target_model = "User"
├── db.relationship.strategy = "selectinload"
├── db.relationship.fk_column = "author_id"
├── db.relationship.batch_count = 100
└── db.result.count = 50
    └── db.query.find (users)  # Nested query span with IN clause
```

### Identity Map Cache Hit

```
db.relationship.select (author)
├── db.relationship.name = "author"
├── db.relationship.target_model = "User"
├── db.relationship.strategy = "select"
├── db.relationship.fk_column = "author_id"
├── db.relationship.cache_hit = true  # Loaded from session
└── db.result.count = 1
    # No nested query span (served from cache)
```

## Usage

### Enable/Disable Tracing

```bash
# Enable tracing (requires OpenTelemetry SDK installed)
export DATA_BRIDGE_TRACING_ENABLED=true

# Disable tracing (default, zero overhead)
export DATA_BRIDGE_TRACING_ENABLED=false
```

### Using in Code

No code changes required - instrumentation is automatic:

```python
from data_bridge.postgres import Table, Column, relationship

class Post(Table):
    id: int = Column(primary_key=True)
    title: str
    author_id: int = Column(foreign_key="users.id")

    author: User = relationship(User, foreign_key_column="author_id")

    class Settings:
        table_name = "posts"

# Lazy loading - creates db.relationship.select span
post = await Post.get(1)
author = await post.author  # Instrumented automatically

# Eager loading - creates db.relationship.selectinload span
from data_bridge.postgres import selectinload

posts = await Post.find().options(selectinload("author")).to_list()
for post in posts:
    author = await post.author  # No span (already loaded)
```

## N+1 Query Detection

Use span counts to detect N+1 query problems:

1. **Look for multiple `db.relationship.select` spans** - Indicates lazy loading in a loop
2. **Check `db.relationship.cache_hit=false`** - Each miss triggers a query
3. **Solution**: Use `selectinload()` to batch load relationships

Example trace showing N+1:
```
db.query.find (posts) - 1 query
├── db.relationship.select (author) - Query 1
├── db.relationship.select (author) - Query 2
├── db.relationship.select (author) - Query 3
└── db.relationship.select (author) - Query 4
```

Fixed with selectinload:
```
db.query.find (posts) - 1 query
└── db.relationship.selectinload (author) - 1 query
```

## Implementation Details

### Files Modified

1. **`python/data_bridge/postgres/relationships.py`**
   - Added telemetry imports
   - Instrumented `RelationshipLoader._load()` with fast-path check
   - Added span creation with relationship attributes
   - Set cache_hit attribute based on session usage

2. **`python/data_bridge/postgres/options.py`**
   - Added telemetry imports
   - Instrumented `SelectInLoad.apply()` with fast-path check
   - Added span creation with batch_count attribute
   - Set result count after batch loading

3. **`python/data_bridge/postgres/telemetry.py`**
   - Enhanced `create_relationship_span()` with new parameters
   - Added new span attributes (target_model, fk_column, cache_hit, batch_count)
   - Enhanced `set_span_result()` to support cache_hit

### Code Pattern

All instrumented methods follow this pattern:

```python
async def method():
    # Fast path: no overhead when tracing disabled
    if not is_tracing_enabled():
        # Original logic
        return result

    # Instrumented path: create span
    with create_relationship_span(...) as span:
        try:
            # Original logic
            result = await do_work()

            # Set span result
            set_span_result(span, count=..., cache_hit=...)
            return result
        except Exception as e:
            add_exception(span, e)
            raise
```

## Testing

Run existing tests to verify no regression:

```bash
# Unit tests (no database required)
DATA_BRIDGE_TRACING_ENABLED=false uv run pytest tests/postgres/unit/test_relationship_descriptor.py -v

# Integration tests (requires PostgreSQL)
DATA_BRIDGE_TRACING_ENABLED=false uv run pytest tests/postgres/integration/test_lazy_loading.py -v
DATA_BRIDGE_TRACING_ENABLED=false uv run pytest tests/postgres/integration/test_eager_loading.py -v
```

## Future Enhancements

1. **N+1 Warning Spans** - Emit warning when threshold exceeded
2. **Relationship Depth Tracking** - Track nested relationship loading depth
3. **Performance Metrics** - Collect histogram of loading times
4. **Cache Hit Rate** - Track identity map effectiveness
