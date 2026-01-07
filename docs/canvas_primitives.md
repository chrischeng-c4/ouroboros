# Canvas Primitives: Map, Starmap, and Chunks

This document describes the Canvas primitives implemented for data-bridge-tasks, providing Celery-compatible parallel task execution patterns.

## Overview

Canvas primitives are high-level workflow patterns that simplify parallel task execution. All three primitives (Map, Starmap, Chunks) internally convert to a `Group` for execution.

## Map

**Purpose**: Apply a task to each item in an iterable.

**Celery equivalent**: `group([task.s(item) for item in items])`

**Rust API**:
```rust
use data_bridge_tasks::{Map, xmap};
use serde_json::json;

// Using constructor
let items = vec![json!(1), json!(2), json!(3)];
let map = Map::new("process_item", items);
let result = map.apply_async(&broker).await?;

// Using helper function
let result = xmap("process_item", vec![json!(1), json!(2)])
    .apply_async(&broker).await?;
```

**Key Methods**:
- `new(task_name, items)` - Create new Map
- `with_options(options)` - Set TaskOptions
- `to_group()` - Convert to Group
- `apply_async(broker)` - Execute (convenience method)

**Behavior**: Each item is passed as the first (and only) argument to the task.

## Starmap

**Purpose**: Apply a task to each tuple of arguments in an iterable.

**Celery equivalent**: `group([task.s(*args) for args in items])`

**Rust API**:
```rust
use data_bridge_tasks::{Starmap, starmap};
use serde_json::json;

// Using constructor
let tuples = vec![
    vec![json!(1), json!(2)],
    vec![json!(3), json!(4)],
];
let starmap = Starmap::new("add", tuples);
let result = starmap.apply_async(&broker).await?;

// Using helper function
let result = starmap("add", vec![
    vec![json!(1), json!(2)],
    vec![json!(3), json!(4)],
]).apply_async(&broker).await?;
```

**Key Methods**:
- `new(task_name, items)` - Create new Starmap
- `with_options(options)` - Set TaskOptions
- `to_group()` - Convert to Group
- `apply_async(broker)` - Execute (convenience method)

**Behavior**: Each inner Vec is unpacked as arguments to the task.

## Chunks

**Purpose**: Split items into batches, processing each batch as one task.

**Celery equivalent**: Celery's chunks primitive

**Rust API**:
```rust
use data_bridge_tasks::{Chunks, chunks};
use serde_json::json;

// Using constructor
let items = vec![json!(1), json!(2), json!(3), json!(4), json!(5)];
let chunks = Chunks::new("batch_process", items, 2);
println!("Will create {} chunks", chunks.num_chunks()); // 3
let result = chunks.apply_async(&broker).await?;

// Using helper function
let result = chunks("batch_process", vec![json!(1), json!(2), json!(3)], 2)
    .apply_async(&broker).await?;
```

**Key Methods**:
- `new(task_name, items, chunk_size)` - Create new Chunks
- `with_options(options)` - Set TaskOptions
- `num_chunks()` - Calculate number of chunks that will be created
- `to_group()` - Convert to Group
- `apply_async(broker)` - Execute (convenience method)

**Behavior**:
- Items are split into batches of `chunk_size`
- Each batch is passed as a single array argument to the task
- Last chunk may be smaller if items don't divide evenly

## Task Options

All primitives support setting TaskOptions that apply to all generated tasks:

```rust
use data_bridge_tasks::TaskOptions;

let options = TaskOptions::new()
    .with_queue("priority")
    .with_countdown(30);

let map = Map::new("task", items).with_options(options);
```

Options are inherited by:
1. The Group as a whole
2. Each individual TaskSignature in the Group

Task-level options override group-level options during execution.

## Examples

### Example 1: Process user records in parallel
```rust
let user_ids = vec![json!(1), json!(2), json!(3)];
let result = xmap("process_user", user_ids)
    .apply_async(&broker)
    .await?;

// In worker:
#[task]
async fn process_user(ctx: TaskContext, user_id: i64) -> Result<Value, TaskError> {
    // Process user...
    Ok(json!({"user_id": user_id, "status": "processed"}))
}
```

### Example 2: Calculate with multiple arguments
```rust
let calculations = vec![
    vec![json!(10), json!(20), json!("add")],
    vec![json!(100), json!(50), json!("subtract")],
];
let result = starmap("calculate", calculations)
    .apply_async(&broker)
    .await?;

// In worker:
#[task]
async fn calculate(ctx: TaskContext, a: i64, b: i64, op: String) -> Result<Value, TaskError> {
    let result = match op.as_str() {
        "add" => a + b,
        "subtract" => a - b,
        _ => return Err(TaskError::InvalidInput("Unknown operation".into())),
    };
    Ok(json!(result))
}
```

### Example 3: Batch processing for efficiency
```rust
// Process 10,000 items in batches of 100
let items: Vec<Value> = (0..10000).map(|i| json!(i)).collect();
let chunks = Chunks::new("batch_insert", items, 100)
    .with_options(TaskOptions::new().with_queue("bulk"));

println!("Creating {} batch tasks", chunks.num_chunks()); // 100
let result = chunks.apply_async(&broker).await?;

// In worker:
#[task]
async fn batch_insert(ctx: TaskContext, items: Vec<Value>) -> Result<Value, TaskError> {
    // Insert items as a batch...
    Ok(json!({"inserted": items.len()}))
}
```

## Implementation Details

### Conversion to Group

All three primitives implement `to_group()` which generates a `Group` of `TaskSignature` objects:

- **Map**: Each item becomes `TaskSignature::new(task_name, [item])`
- **Starmap**: Each args tuple becomes `TaskSignature::new(task_name, args)`
- **Chunks**: Each chunk becomes `TaskSignature::new(task_name, [chunk])`

### Helper Functions

Helper functions (`xmap`, `starmap`, `chunks`) provide ergonomic API similar to Python's built-ins:
- `xmap` follows Python's convention (x for cross-product/map)
- `starmap` matches Python's `itertools.starmap`
- `chunks` is custom but follows the naming pattern

### Performance Considerations

1. **Vector Pre-allocation**: All primitives pre-allocate vectors to avoid reallocations
2. **Zero-copy Conversion**: Items are cloned only when creating TaskSignatures
3. **Chunk Size**: For Chunks, larger batches reduce overhead but increase task granularity
4. **Parallel Execution**: All tasks in the generated Group execute in parallel via the broker

## Testing

All primitives include comprehensive tests:
- **Map**: 6 tests covering basic usage, empty input, group conversion, options
- **Starmap**: 8 tests covering tuples, single args, complex types, options
- **Chunks**: 11 tests covering even/uneven splits, edge cases, calculation

Run tests:
```bash
cargo test -p data-bridge-tasks --features nats workflow
```

Run example:
```bash
cargo run -p data-bridge-tasks --example canvas_primitives --features nats
```

## Files Created

- `/crates/data-bridge-tasks/src/workflow/map.rs` - Map implementation
- `/crates/data-bridge-tasks/src/workflow/starmap.rs` - Starmap implementation
- `/crates/data-bridge-tasks/src/workflow/chunks.rs` - Chunks implementation
- `/crates/data-bridge-tasks/examples/canvas_primitives.rs` - Usage examples
- `/docs/canvas_primitives.md` - This documentation

## Future Enhancements

Potential improvements:
1. Add `Map::collect()` to gather results (like Python's map)
2. Support async iterators for streaming large datasets
3. Add retry policies specific to batch operations
4. Implement `chord` variants (map_chord, starmap_chord)
5. Add progress tracking for long-running batch operations
