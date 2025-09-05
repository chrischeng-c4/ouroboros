# MongoDB Field Update Operations

This document outlines the comprehensive MongoDB field update operations implemented in Data Bridge.

## Overview

Each field type now supports MongoDB-specific update operations that enable atomic database updates without fetching and replacing entire documents. This provides significant performance benefits and prevents race conditions in concurrent environments.

## Architecture

### UpdateExpression Class
```python
@dataclass(slots=True, frozen=True)
class UpdateExpression:
    """Represents a single database update operation."""
    field: str
    operator: str  
    value: Any
    modifiers: dict[str, Any] | None = None
```

### MongoUpdateTranslator
Translates field update operations to MongoDB update document format:

```python
# Input: List[UpdateExpression]
# Output: MongoDB update document
{"$set": {...}, "$inc": {...}, "$push": {...}}
```

## Field-Specific Operations

### Base Field (All Types)
- `set(value)` → `$set`
- `unset()` → `$unset` 
- `rename(new_name)` → `$rename`

### Numeric Fields (IntField, FloatField)
- `inc(value=1)` → `$inc` - Increment by value
- `mul(value)` → `$mul` - Multiply by value
- `min(value)` → `$min` - Set to minimum of current and value
- `max(value)` → `$max` - Set to maximum of current and value

### Boolean Fields (BoolField)
- `toggle()` → `$set` with `$not` operator - Toggle boolean value

### Array Fields (ListField)

#### Basic Operations
- `push(value)` → `$push` - Add element to array
- `pull(value)` → `$pull` - Remove matching elements
- `pull_all(values)` → `$pullAll` - Remove multiple values
- `add_to_set(value)` → `$addToSet` - Add unique element
- `pop(position=1)` → `$pop` - Remove first (-1) or last (1) element

#### Batch Operations
- `push_all(values)` → `$push` with `$each`
- `add_to_set_each(values)` → `$addToSet` with `$each`

#### Advanced Modifiers
- `push(value, position=0)` - Insert at specific position
- `push(value, slice=10)` - Keep only last N elements after push
- `push(value, sort=1)` - Sort array after push

### Dict Fields (DictField)
- `set_field(path, value)` - Set nested field using dot notation
- `unset_field(path)` - Remove nested field
- `inc_field(path, value)` - Increment nested numeric field

## Usage Examples

### Basic Usage
```python
# Field definitions
name = StringField()
name.name = "name"

age = IntField() 
age.name = "age"

tags = ListField(str)
tags.name = "tags"

# Create update expressions
updates = [
    name.set("John Doe"),
    age.inc(1),
    tags.push("new_tag")
]

# Translate to MongoDB
update_doc = MongoUpdateTranslator.translate(updates)
# Result: {
#   "$set": {"name": "John Doe"},
#   "$inc": {"age": 1}, 
#   "$push": {"tags": "new_tag"}
# }
```

### Advanced Array Operations
```python
scores = ListField(int)
scores.name = "scores"

# Push with modifiers
updates = [
    scores.push(95, position=0, slice=10, sort=-1)
]

update_doc = MongoUpdateTranslator.translate(updates)
# Result: {
#   "$push": {
#     "scores": {
#       "$each": [95],
#       "$position": 0,
#       "$slice": 10,
#       "$sort": -1
#     }
#   }
# }
```

### Nested Field Operations
```python
metadata = DictField()
metadata.name = "metadata"

updates = [
    metadata.set_field("config.theme", "dark"),
    metadata.inc_field("stats.views", 1),
    metadata.unset_field("old_setting")
]

update_doc = MongoUpdateTranslator.translate(updates)
# Result: {
#   "$set": {"metadata.config.theme": "dark"},
#   "$inc": {"metadata.stats.views": 1},
#   "$unset": {"metadata.old_setting": ""}
# }
```

## Backend Integration

### Sync Backend
```python
def update_query(self, query: MongoQuery[T], updates: list[UpdateExpression]) -> int:
    collection = self.get_collection(query.model_class)
    filter_query = MongoQueryTranslator.translate(query.expressions)
    update_doc = MongoUpdateTranslator.translate(updates)
    result = collection.update_many(filter_query, update_doc)
    return result.modified_count
```

### Async Backend
```python
async def update_query(self, query: AsyncMongoQuery[T], updates: list[UpdateExpression]) -> int:
    collection = await self.get_collection(query.model_class)
    filter_query = MongoQueryTranslator.translate(query.expressions)
    update_doc = MongoUpdateTranslator.translate(updates)
    result = await collection.update_many(filter_query, update_doc)
    return result.modified_count
```

## Type Safety

All update operations are fully typed and provide IDE autocompletion:

- `IntField.inc(value: int)` - Only accepts integers
- `FloatField.inc(value: float)` - Only accepts floats  
- `ListField[T].push(value: T)` - Only accepts items of list type
- `DictField.set_field(path: str, value: Any)` - Typed path strings

## Testing

Comprehensive test coverage includes:

- **Unit Tests**: `tests/unit/base/test_update_operations.py` (36 tests)
- **MongoDB Translator Tests**: `tests/unit/mongo/test_mongo_update_translator.py` (38 tests)
- **Integration Tests**: Field operations with real MongoDB update documents
- **Edge Cases**: Empty values, special characters, nested operations

## Performance Benefits

1. **Atomic Updates**: No fetch-modify-replace cycles
2. **Reduced Network Traffic**: Single update command vs full document replacement
3. **Concurrency Safe**: MongoDB handles atomic operations internally
4. **Efficient Array Operations**: Native array modifiers like `$addToSet`, `$pull`
5. **Partial Updates**: Only update specific fields, not entire documents

## MongoDB Operator Support

| Internal Operator | MongoDB Operator | Description |
|------------------|-----------------|-------------|
| `set` | `$set` | Set field value |
| `unset` | `$unset` | Remove field |
| `rename` | `$rename` | Rename field |
| `inc` | `$inc` | Increment number |
| `mul` | `$mul` | Multiply number |
| `min` | `$min` | Set to minimum |
| `max` | `$max` | Set to maximum |
| `push` | `$push` | Add to array |
| `pull` | `$pull` | Remove from array |
| `pullAll` | `$pullAll` | Remove multiple |
| `addToSet` | `$addToSet` | Add unique to array |
| `pop` | `$pop` | Remove first/last |
| `toggle` | `$set` + `$not` | Toggle boolean |

## Future Enhancements

1. **Conditional Updates**: `$setOnInsert`, `$currentDate`
2. **Array Filters**: Update specific array elements
3. **Aggregation Pipeline Updates**: Complex transformations
4. **Bulk Operations**: Batch multiple update operations
5. **Validation**: Runtime validation of operation combinations
6. **Other Backends**: Extend pattern to Redis, Firestore

## Migration Guide

Existing code using simple dictionary updates:

```python
# Old way
User.objects().update({"name": "John", "age": 30})

# New way  
User.objects().update([
    User.name.set("John"),
    User.age.set(30)
])
```

The new approach provides:
- Type safety
- IDE autocompletion
- Field-specific operations
- Better error messages
- Atomic operations support