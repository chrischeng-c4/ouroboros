# PostgreSQL Extensions Quick Reference

This guide covers advanced PostgreSQL-specific features available in data-bridge.

## Table of Contents

1. [Full-Text Search](#full-text-search)
2. [PostGIS Spatial Queries](#postgis-spatial-queries)
3. [Array Operators](#array-operators)
4. [JSONB Operators](#jsonb-operators)
5. [RaiseLoad (N+1 Detection)](#raiseload-n1-detection)

---

## Full-Text Search

PostgreSQL's full-text search provides linguistic search capabilities.

### Import

```python
from data_bridge.postgres import FullTextSearch, fts
```

### Available Methods

#### `to_tsvector(column, config="english")`
Create a tsvector for full-text indexing.

```python
expr = FullTextSearch.to_tsvector("content")
# Result: "to_tsvector('english', content)"

# With custom language
expr = FullTextSearch.to_tsvector("content", config="spanish")
# Result: "to_tsvector('spanish', content)"
```

#### `to_tsquery(query, config="english")`
Create a tsquery for full-text search.

```python
expr = FullTextSearch.to_tsquery("python & database")
# Result: "to_tsquery('english', 'python & database')"
```

#### `plainto_tsquery(query, config="english")`
Convert plain text to tsquery (automatically handles operators).

```python
expr = FullTextSearch.plainto_tsquery("python database")
# Result: "plainto_tsquery('english', 'python database')"
```

#### `match(column, query, config="english")`
Generate a complete match expression.

```python
expr = fts.match("content", "python programming")
# Result: "to_tsvector('english', content) @@ plainto_tsquery('english', 'python programming')"
```

#### `rank(column, query, config="english")`
Generate a ranking expression for sorting results.

```python
expr = fts.rank("content", "python programming")
# Result: "ts_rank(to_tsvector('english', content), plainto_tsquery('english', 'python programming'))"
```

### Creating FTS Indexes

```sql
-- Create a GIN index for fast full-text search
CREATE INDEX articles_content_fts_idx ON articles
USING GIN (to_tsvector('english', content));

-- Query using the index
SELECT * FROM articles
WHERE to_tsvector('english', content) @@ plainto_tsquery('english', 'python database');
```

---

## PostGIS Spatial Queries

PostGIS adds support for geographic objects in PostgreSQL.

### Import

```python
from data_bridge.postgres import Point, GeoQuery
```

### Point Class

#### Creating Points

```python
# Create a point (longitude, latitude)
point = Point(121.5, 25.0)  # Taipei coordinates

# Custom SRID
point = Point(121.5, 25.0, srid=3857)  # Web Mercator
```

#### Converting to SQL

```python
sql = point.to_sql()
# Result: "ST_SetSRID(ST_MakePoint(121.5, 25.0), 4326)"
```

#### From Well-Known Text (WKT)

```python
sql = Point.from_wkt("POINT(121.5 25.0)")
# Result: "ST_GeomFromText('POINT(121.5 25.0)', 4326)"
```

### GeoQuery Class

#### `distance(geom1, geom2)`
Calculate distance between geometries.

```python
expr = GeoQuery.distance("coordinates", "ST_MakePoint(121.5, 25.0)")
# Result: "ST_Distance(coordinates, ST_MakePoint(121.5, 25.0))"
```

#### `dwithin(geom1, geom2, distance)`
Check if geometries are within a specified distance.

```python
point = Point(121.5, 25.0)
expr = GeoQuery.dwithin("coordinates", point.to_sql(), 1000)
# Result: "ST_DWithin(coordinates, ST_SetSRID(ST_MakePoint(121.5, 25.0), 4326), 1000)"
```

#### `contains(geom1, geom2)`
Check if geom1 contains geom2.

```python
expr = GeoQuery.contains("polygon", "point")
# Result: "ST_Contains(polygon, point)"
```

#### `within(geom1, geom2)`
Check if geom1 is within geom2.

```python
expr = GeoQuery.within("point", "polygon")
# Result: "ST_Within(point, polygon)"
```

#### `intersects(geom1, geom2)`
Check if geometries intersect.

```python
expr = GeoQuery.intersects("geom1", "geom2")
# Result: "ST_Intersects(geom1, geom2)"
```

### Installing PostGIS

```sql
-- Enable PostGIS extension
CREATE EXTENSION IF NOT EXISTS postgis;

-- Create a table with geometry column
CREATE TABLE locations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255),
    coordinates GEOMETRY(Point, 4326)
);

-- Create spatial index
CREATE INDEX locations_coordinates_idx ON locations
USING GIST (coordinates);
```

---

## Array Operators

PostgreSQL array operators for working with array columns.

### Import

```python
from data_bridge.postgres import ArrayOps
```

### Available Methods

#### `contains(column, value)`
Check if array contains all elements (@> operator).

```python
expr = ArrayOps.contains("tags", ["python", "database"])
# Result: "tags @> ARRAY['python', 'database']"
```

#### `contained_by(column, value)`
Check if array is contained by (<@ operator).

```python
expr = ArrayOps.contained_by("tags", ["python", "rust", "go"])
# Result: "tags <@ ARRAY['python', 'rust', 'go']"
```

#### `overlap(column, value)`
Check if arrays have any common elements (&& operator).

```python
expr = ArrayOps.overlap("tags", ["python", "rust"])
# Result: "tags && ARRAY['python', 'rust']"
```

#### `any(column, value)`
Check if value is in array (ANY operator).

```python
# String value
expr = ArrayOps.any("tags", "python")
# Result: "'python' = ANY(tags)"

# Numeric value
expr = ArrayOps.any("scores", 100)
# Result: "100 = ANY(scores)"
```

#### `length(column)`
Get array length.

```python
expr = ArrayOps.length("tags")
# Result: "array_length(tags, 1)"
```

### Creating Array Columns

```sql
-- Create table with array column
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255),
    tags TEXT[]
);

-- Create GIN index for array containment queries
CREATE INDEX posts_tags_idx ON posts USING GIN (tags);

-- Insert data
INSERT INTO posts (title, tags)
VALUES ('My Post', ARRAY['python', 'database', 'orm']);
```

---

## JSONB Operators

Enhanced JSONB operators integrated into QueryBuilder.

### Import

```python
from data_bridge.postgres import Table, Column
```

### QueryBuilder Methods

#### `jsonb_contains(column, value)`
Check if JSONB column contains the given JSON (@> operator).

```python
class User(Table):
    id: int = Column(primary_key=True)
    metadata: dict

    class Settings:
        table_name = "users"

# Find users with admin role
admins = await User.find().jsonb_contains("metadata", {"role": "admin"}).to_list()
```

#### `jsonb_contained_by(column, value)`
Check if JSONB column is contained by the given JSON (<@ operator).

```python
users = await User.find().jsonb_contained_by("metadata", {"role": "admin", "active": True}).to_list()
```

#### `jsonb_has_key(column, key)`
Check if JSONB column has a specific key (? operator).

```python
users = await User.find().jsonb_has_key("settings", "theme").to_list()
```

#### `jsonb_has_any_key(column, keys)`
Check if JSONB column has any of the specified keys (?| operator).

```python
users = await User.find().jsonb_has_any_key("metadata", ["email", "phone"]).to_list()
```

#### `jsonb_has_all_keys(column, keys)`
Check if JSONB column has all specified keys (?& operator).

```python
users = await User.find().jsonb_has_all_keys("settings", ["theme", "language"]).to_list()
```

### Chaining JSONB Methods

```python
# Complex query with multiple JSONB conditions
users = await User.find()
    .jsonb_contains("metadata", {"active": True})
    .jsonb_has_key("settings", "theme")
    .jsonb_has_all_keys("profile", ["name", "email"])
    .order_by("name")
    .to_list()
```

### Creating JSONB Indexes

```sql
-- Create GIN index for JSONB containment
CREATE INDEX users_metadata_idx ON users USING GIN (metadata);

-- Create GIN index for key existence
CREATE INDEX users_settings_idx ON users USING GIN (settings jsonb_path_ops);
```

---

## RaiseLoad (N+1 Detection)

RaiseLoad is a testing utility that raises an error if a relationship is accessed without being eager-loaded, helping detect N+1 query problems.

### Import

```python
from data_bridge.postgres import raiseload, selectinload
```

### Basic Usage

```python
from data_bridge.postgres import Table, Column, relationship

class Author(Table):
    id: int = Column(primary_key=True)
    name: str

    class Settings:
        table_name = "authors"

class Book(Table):
    id: int = Column(primary_key=True)
    title: str
    author_id: int = Column(foreign_key="authors.id")

    author: Author = relationship(Author, foreign_key_column="author_id")

    class Settings:
        table_name = "books"
```

### Detecting N+1 Queries in Tests

```python
import pytest

@pytest.mark.asyncio
async def test_no_n_plus_one():
    """Test that code doesn't have N+1 queries."""
    # Load books with raiseload - will raise if relationship is accessed
    books = await Book.find().options(raiseload("author")).to_list()

    # This would raise RuntimeError
    # author = await books[0].author  # ❌ N+1 detected!

    # Fix: Use selectinload instead
    books = await Book.find().options(selectinload("author")).to_list()
    author = await books[0].author  # ✅ OK! Already loaded
```

### Error Message

When raiseload detects an unloaded relationship access:

```
RuntimeError: Attempted to access unloaded relationship 'author'.
Use selectinload() to eagerly load this relationship.
```

### When to Use RaiseLoad

- **DO** use in test environments to catch N+1 queries
- **DO** use during code review to verify proper eager loading
- **DON'T** use in production (performance overhead and errors)
- **DON'T** use with relationships that are intentionally lazy-loaded

### Fixing N+1 Queries

```python
# ❌ BAD: N+1 queries (1 + N queries)
books = await Book.find().to_list()
for book in books:
    author = await book.author  # N queries

# ✅ GOOD: Batch loading (1 + 1 queries)
books = await Book.find().options(selectinload("author")).to_list()
for book in books:
    author = await book.author  # No query, already loaded
```

---

## PostgreSQL Requirements

| Feature | PostgreSQL Version | Extension Required |
|---------|-------------------|-------------------|
| Full-Text Search | 8.3+ | Built-in |
| JSONB | 9.4+ | Built-in |
| Array Operations | 8.1+ | Built-in |
| PostGIS | 9.1+ | `CREATE EXTENSION postgis` |

## Performance Tips

1. **Full-Text Search**
   - Create GIN indexes on tsvector columns
   - Use materialized views for complex searches
   - Consider language-specific configurations

2. **PostGIS**
   - Create GIST indexes on geometry columns
   - Keep SRID consistent across queries
   - Use ST_DWithin instead of ST_Distance for range queries

3. **Arrays**
   - Create GIN indexes for containment queries
   - Consider PostgreSQL version for best performance
   - Use `ANY` operator for single element checks

4. **JSONB**
   - Create GIN indexes for containment and key queries
   - Use `jsonb_path_ops` for key-only queries
   - Normalize frequently queried JSON fields to columns

5. **RaiseLoad**
   - Only use in test environments
   - Combine with selectinload for proper eager loading
   - Monitor query counts with database logging

---

## Additional Resources

- [PostgreSQL Full-Text Search Documentation](https://www.postgresql.org/docs/current/textsearch.html)
- [PostGIS Documentation](https://postgis.net/documentation/)
- [PostgreSQL Array Functions](https://www.postgresql.org/docs/current/functions-array.html)
- [PostgreSQL JSON Functions](https://www.postgresql.org/docs/current/functions-json.html)
- [Avoiding N+1 Queries](https://stackoverflow.com/questions/97197/what-is-the-n1-selects-problem)
