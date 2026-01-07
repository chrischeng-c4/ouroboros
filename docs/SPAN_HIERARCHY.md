# Span Hierarchy for Relationship Loading

## Lazy Loading (N+1 Pattern)

```
Request: Load 3 posts with authors

db.query.find (posts)
├─ db.system = "postgresql"
├─ db.collection.name = "posts"
├─ db.operation.name = "find"
└─ db.result.count = 3
   |
   └─ Post 1 access
      └─ db.relationship.select (author) ← NEW SPAN
         ├─ db.relationship.name = "author"
         ├─ db.relationship.target_model = "User"
         ├─ db.relationship.strategy = "select"
         ├─ db.relationship.fk_column = "author_id"
         ├─ db.relationship.cache_hit = false
         └─ db.result.count = 1
            |
            └─ db.query.find (users)
               └─ db.statement = "SELECT * FROM users WHERE id = $1"
   |
   └─ Post 2 access
      └─ db.relationship.select (author) ← NEW SPAN
         └─ db.query.find (users)
   |
   └─ Post 3 access
      └─ db.relationship.select (author) ← NEW SPAN
         └─ db.query.find (users)

Total Queries: 1 (posts) + 3 (authors) = 4 queries
```

## Eager Loading (Optimized)

```
Request: Load 3 posts with authors using selectinload()

db.query.find (posts)
├─ db.system = "postgresql"
├─ db.collection.name = "posts"
└─ db.result.count = 3
   |
   └─ db.relationship.selectinload (author) ← NEW SPAN
      ├─ db.relationship.name = "author"
      ├─ db.relationship.target_model = "User"
      ├─ db.relationship.strategy = "selectinload"
      ├─ db.relationship.fk_column = "author_id"
      ├─ db.relationship.batch_count = 3
      └─ db.result.count = 3
         |
         └─ db.query.find (users)
            └─ db.statement = "SELECT * FROM users WHERE id IN ($1, $2, $3)"

Total Queries: 1 (posts) + 1 (authors batch) = 2 queries
```

## Identity Map Cache Hit

```
Request: Load post within session (author already loaded)

Session.get(User, 123)
└─ db.session.get (users)
   └─ Loads User with id=123 into identity map

Post access
└─ db.relationship.select (author) ← NEW SPAN
   ├─ db.relationship.name = "author"
   ├─ db.relationship.target_model = "User"
   ├─ db.relationship.strategy = "select"
   ├─ db.relationship.fk_column = "author_id"
   ├─ db.relationship.cache_hit = true ← Cache hit!
   └─ db.result.count = 1
      # No nested db.query span - served from cache

Total Queries: 0 (served from session identity map)
```

## NULL Foreign Key

```
Request: Load post with NULL author_id

db.query.find (posts)
└─ db.result.count = 1

Post access
└─ db.relationship.select (author) ← NEW SPAN
   ├─ db.relationship.name = "author"
   ├─ db.relationship.target_model = "User"
   ├─ db.relationship.strategy = "select"
   ├─ db.relationship.fk_column = "author_id"
   ├─ db.relationship.cache_hit = false
   └─ db.result.count = 0 ← No result (NULL FK)
      # No nested db.query span - FK is NULL

Total Queries: 0 (FK is NULL, no query needed)
```

## Span Cardinality Analysis

### Low-Cardinality (Good for Metrics)

Span names are strategy-based (not relationship-based):
- `db.relationship.select` - All lazy loads
- `db.relationship.selectinload` - All batch loads
- `db.relationship.joined` - All joined loads (future)

This allows aggregation across different relationships:
```sql
SELECT strategy, COUNT(*), AVG(duration)
FROM spans
WHERE name LIKE 'db.relationship.%'
GROUP BY strategy
```

### High-Cardinality (Good for Debugging)

Span attributes include specific details:
- `db.relationship.name` - Specific relationship (e.g., "author", "posts", "comments")
- `db.relationship.target_model` - Target model class
- `db.relationship.fk_column` - Foreign key column

This allows debugging specific relationships:
```sql
SELECT name, target_model, AVG(duration)
FROM spans
WHERE relationship_name = 'author'
  AND cache_hit = false
```

## Performance Impact

### When Tracing Disabled (default)

```python
async def _load(self):
    if self._is_loaded:
        return self._loaded_value

    # ✓ Fast path: immediate return, no span logic
    if not is_tracing_enabled():
        # Original code path (no overhead)
        ...
        return value

    # This code never executes when tracing disabled
    with create_relationship_span(...):
        ...
```

Overhead: **0ms** (fast-path check is ~0.001ms)

### When Tracing Enabled

```python
async def _load(self):
    if self._is_loaded:
        return self._loaded_value

    # Tracing enabled: create span
    with create_relationship_span(...) as span:
        # Original loading logic
        value = await load()

        # Set span attributes
        set_span_result(span, count=1, cache_hit=True)

        return value
```

Overhead: **~1-2ms per span** (span creation + attribute setting)

For 100 lazy loads:
- Without tracing: 0ms overhead
- With tracing: ~100-200ms overhead (acceptable for debugging)

## N+1 Detection Strategy

Compare span patterns to detect N+1 queries:

### N+1 Pattern (Bad)
```
Multiple db.relationship.select spans with cache_hit=false
→ Each triggers a separate db.query.find span
→ Total queries = N + 1
```

### Optimized Pattern (Good)
```
Single db.relationship.selectinload span
→ Single db.query.find span with IN clause
→ Total queries = 2
```

### Alerting Rule

```yaml
alert: N+1QueryDetected
expr: |
  rate(relationship_select_spans{cache_hit="false"}[5m]) > 10
message: "High number of lazy loads detected. Consider using selectinload()."
```
