# Spreadsheet Engine Architecture

Technical architecture documentation for the data-bridge-sheet spreadsheet system.

**Last Updated:** 2026-01-08

---

## Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Component Details](#component-details)
4. [Custom Database Engine](#custom-database-engine)
5. [Formula Engine](#formula-engine)
6. [Collaboration System](#collaboration-system)
7. [Performance Optimizations](#performance-optimizations)
8. [Data Flow](#data-flow)

---

## Overview

The data-bridge spreadsheet engine is a high-performance spreadsheet system built with Rust and WebAssembly. It features:

- **Zero-copy rendering** - Direct memory access from JavaScript
- **Custom database** - Morton-encoded spatial indexing for O(1) cell lookup
- **CRDT-based collaboration** - Multi-user editing with conflict-free replication
- **Formula evaluation** - 24+ built-in functions with dependency tracking
- **Event-driven architecture** - Subscribe to changes, selections, and operations

### Design Goals

1. **Performance** - Match or exceed native spreadsheet applications
2. **Scalability** - Handle millions of cells efficiently
3. **Collaboration** - Real-time multi-user editing without conflicts
4. **Extensibility** - Easy to add new functions and features
5. **Web-first** - Optimized for browser environments via WebAssembly

---

## System Architecture

### Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Browser Frontend                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Canvas      │  │  Yjs Client │  │   RuSheet API       │  │
│  │ Renderer    │  │  (collab)   │  │   (TypeScript)      │  │
│  │             │  │             │  │                     │  │
│  │ - Virtual   │  │ - Awareness │  │ - Event handlers    │  │
│  │   viewport  │  │ - Cursors   │  │ - State management  │  │
│  │ - Smooth    │  │ - Sync      │  │ - WASM bridge       │  │
│  │   scrolling │  │             │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└──────────────────────────┬──────────────────────────────────┘
                           │
                      WASM Bridge
                    (wasm-bindgen)
                           │
┌──────────────────────────▼──────────────────────────────────┐
│              Spreadsheet Engine (WASM/Rust)                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                 sheet-core                           │   │
│  │  - Cell (value, format, formula)                     │   │
│  │  - Sheet (grid operations)                           │   │
│  │  - Workbook (multi-sheet)                            │   │
│  │  - Formatting (styles, alignment, colors)            │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │               sheet-formula                          │   │
│  │  - Parser (nom-based, recursive descent)             │   │
│  │  - Evaluator (lazy evaluation)                       │   │
│  │  - Functions (SUM, IF, VLOOKUP, etc.)                │   │
│  │  - Dependency tracker (directed acyclic graph)       │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │               sheet-history                          │   │
│  │  - Command pattern (undo/redo)                       │   │
│  │  - History stack (unlimited undo)                    │   │
│  │  - Batch operations (group commands)                 │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                sheet-db                              │   │
│  │  - Morton encoding (Z-order curve)                   │   │
│  │  - Sparse storage (only non-empty cells)             │   │
│  │  - Write-ahead log (crash recovery)                  │   │
│  │  - Range queries (O(log n) spatial queries)          │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────┬──────────────────────────────────┘
                           │
                    WebSocket / HTTP
                           │
┌──────────────────────────▼──────────────────────────────────┐
│             Collaboration Server (Axum + Yjs)                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Axum HTTP   │  │  yrs (CRDT)  │  │   PostgreSQL     │   │
│  │  - REST API  │  │  - Y.Doc     │  │   - Workbooks    │   │
│  │  - WebSocket │  │  - Awareness │  │   - Users        │   │
│  │  - Auth      │  │  - Sync      │  │   - Snapshots    │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Details

### 1. data-bridge-sheet-core

**Responsibility:** Core data structures for cells, sheets, and workbooks.

#### Key Types

```rust
// Cell data structure
pub struct Cell {
    pub value: CellValue,           // Actual value (number, text, bool, error)
    pub formula: Option<String>,     // Formula string (e.g., "=SUM(A1:A10)")
    pub format: CellFormat,          // Formatting (font, color, alignment)
    pub dependencies: Vec<CellRef>,  // Cells this formula depends on
}

// Cell value types
pub enum CellValue {
    Empty,
    Number(f64),
    Text(String),
    Boolean(bool),
    Error(FormulaError),
    DateTime(chrono::DateTime<Utc>),
}

// Cell formatting
pub struct CellFormat {
    pub bold: bool,
    pub italic: bool,
    pub font_size: u8,
    pub text_color: Color,
    pub background_color: Option<Color>,
    pub horizontal_align: HorizontalAlign,
    pub vertical_align: VerticalAlign,
    pub number_format: NumberFormat,
}

// Sheet with 64x64 chunks
pub struct Sheet {
    pub name: String,
    pub chunks: HashMap<ChunkCoord, Chunk>,  // Sparse storage
    pub row_count: u32,
    pub col_count: u32,
}

// Workbook (multi-sheet)
pub struct Workbook {
    pub sheets: Vec<Sheet>,
    pub active_sheet: usize,
    pub metadata: WorkbookMetadata,
}
```

#### Features

- **Sparse storage** - Only non-empty cells consume memory
- **64x64 chunks** - Cells organized in chunks for locality
- **Rich formatting** - Fonts, colors, alignment, number formats
- **Multi-sheet support** - Unlimited sheets per workbook

---

### 2. data-bridge-sheet-db

**Responsibility:** Custom database engine with spatial indexing.

#### Morton Encoding (Z-order Curve)

Morton encoding interleaves the bits of row and column coordinates to create a single integer key that preserves spatial locality.

**Why Morton encoding?**
- **Spatial locality** - Nearby cells have nearby keys
- **Fast range queries** - Query rectangular ranges with O(log n) complexity
- **Cache-friendly** - Sequential access to nearby cells
- **Compact keys** - Single u64 instead of (row, col) tuple

**Example:**

```
Row = 5 (binary: 0101)
Col = 3 (binary: 0011)

Morton key:
  Row bits:  0 1 0 1
  Col bits:  0 0 1 1
  Interleaved: 00 01 10 11 = 0x0027 (39 in decimal)
```

#### Architecture

```rust
// Morton key (Z-order curve encoding)
pub struct MortonKey(u64);

impl MortonKey {
    // Encode (row, col) into Morton key
    pub fn encode(row: u32, col: u32) -> Self {
        let mut key = 0u64;
        for i in 0..32 {
            key |= ((row >> i) & 1) << (2 * i);
            key |= ((col >> i) & 1) << (2 * i + 1);
        }
        MortonKey(key)
    }

    // Decode Morton key back to (row, col)
    pub fn decode(self) -> (u32, u32) {
        let mut row = 0u32;
        let mut col = 0u32;
        for i in 0..32 {
            row |= ((self.0 >> (2 * i)) & 1) << i;
            col |= ((self.0 >> (2 * i + 1)) & 1) << i;
        }
        (row, col)
    }

    // Calculate Morton range for rectangle
    pub fn range_for_rect(
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Vec<(MortonKey, MortonKey)> {
        // Returns list of (min, max) ranges that cover the rectangle
        // Uses quadtree decomposition
        todo!()
    }
}

// Storage layer
pub struct CellStore {
    kv_engine: KvEngine,           // Underlying KV store
    wal: WriteAheadLog,            // Write-ahead log for durability
    cache: LruCache<MortonKey, Cell>, // In-memory cache
}

// Write-ahead log
pub struct WriteAheadLog {
    file: File,
    entries: Vec<WalEntry>,
}

pub enum WalEntry {
    SetCell { key: MortonKey, value: Cell },
    DeleteCell { key: MortonKey },
    Checkpoint { timestamp: u64 },
}
```

#### Query Layer

```rust
// Range query (rectangular area)
pub struct RangeQuery {
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
    pub filter: Option<CellFilter>,
}

// Spatial query
pub enum SpatialQuery {
    NearestNeighbors { row: u32, col: u32, k: usize },
    WithinRadius { row: u32, col: u32, radius: u32 },
    DetectClusters { min_points: usize, epsilon: u32 },
}

// Execute range query
impl CellStore {
    pub fn query_range(&self, query: RangeQuery) -> Vec<(u32, u32, Cell)> {
        let ranges = MortonKey::range_for_rect(
            query.start_row,
            query.start_col,
            query.end_row,
            query.end_col,
        );

        let mut results = Vec::new();
        for (min, max) in ranges {
            // Query KV store for keys in [min, max]
            for (key, cell) in self.kv_engine.range(min..=max) {
                if let Some(filter) = &query.filter {
                    if !filter.matches(&cell) {
                        continue;
                    }
                }
                let (row, col) = key.decode();
                results.push((row, col, cell));
            }
        }
        results
    }
}
```

#### CRDT Operations

```rust
// CRDT operation (conflict-free replicated data type)
pub struct CrdtOperation {
    pub timestamp: u64,              // Lamport timestamp
    pub actor_id: String,            // User ID
    pub operation: Operation,
    pub vector_clock: VectorClock,   // Causality tracking
}

pub enum Operation {
    SetCell { row: u32, col: u32, value: CellValue },
    DeleteCell { row: u32, col: u32 },
    SetFormat { row: u32, col: u32, format: CellFormat },
}

// Last-Write-Wins resolution
pub fn merge_operations(a: &CrdtOperation, b: &CrdtOperation) -> CrdtOperation {
    if a.timestamp > b.timestamp {
        a.clone()
    } else if b.timestamp > a.timestamp {
        b.clone()
    } else {
        // Tie-break by actor_id
        if a.actor_id > b.actor_id {
            a.clone()
        } else {
            b.clone()
        }
    }
}
```

---

### 3. data-bridge-sheet-formula

**Responsibility:** Formula parsing and evaluation.

#### Parser

Uses **nom** parser combinators for robust formula parsing.

```rust
// Formula AST (Abstract Syntax Tree)
pub enum Expr {
    Number(f64),
    Text(String),
    Boolean(bool),
    CellRef(CellRef),                     // A1, $B$2
    Range(CellRef, CellRef),              // A1:B10
    Function(String, Vec<Expr>),          // SUM(A1:A10)
    BinaryOp(BinaryOp, Box<Expr>, Box<Expr>), // A1 + B1
    UnaryOp(UnaryOp, Box<Expr>),          // -A1
}

// Cell reference
pub struct CellRef {
    pub sheet: Option<String>,  // Sheet2!A1
    pub row: u32,
    pub col: u32,
    pub row_absolute: bool,     // $A1
    pub col_absolute: bool,     // A$1
}

// Parser entry point
pub fn parse_formula(input: &str) -> Result<Expr, FormulaError> {
    // Remove leading '='
    let input = input.strip_prefix('=').unwrap_or(input);

    // Parse with nom
    formula_parser(input)
        .map(|(_, expr)| expr)
        .map_err(|e| FormulaError::ParseError(e.to_string()))
}
```

#### Evaluator

```rust
// Evaluator with dependency tracking
pub struct FormulaEvaluator {
    workbook: Arc<Workbook>,
    dependency_graph: DependencyGraph,
    cache: HashMap<CellRef, CellValue>,
}

impl FormulaEvaluator {
    // Evaluate formula
    pub fn eval(&mut self, expr: &Expr) -> Result<CellValue, FormulaError> {
        match expr {
            Expr::Number(n) => Ok(CellValue::Number(*n)),
            Expr::Text(s) => Ok(CellValue::Text(s.clone())),
            Expr::Boolean(b) => Ok(CellValue::Boolean(*b)),

            Expr::CellRef(cell_ref) => {
                // Check cache first
                if let Some(cached) = self.cache.get(cell_ref) {
                    return Ok(cached.clone());
                }

                // Get cell value
                let cell = self.workbook.get_cell(cell_ref)?;

                // If cell has formula, evaluate recursively
                if let Some(formula) = &cell.formula {
                    let expr = parse_formula(formula)?;
                    self.eval(&expr)
                } else {
                    Ok(cell.value.clone())
                }
            }

            Expr::Function(name, args) => {
                self.eval_function(name, args)
            }

            Expr::BinaryOp(op, left, right) => {
                let left_val = self.eval(left)?;
                let right_val = self.eval(right)?;
                self.eval_binary_op(*op, left_val, right_val)
            }

            // ... other cases
        }
    }
}
```

#### Built-in Functions

**Math Functions:**
- `SUM(range)` - Sum of values
- `AVERAGE(range)` - Average of values
- `MIN(range)`, `MAX(range)` - Min/max values
- `COUNT(range)` - Count of numeric values
- `ABS(value)`, `ROUND(value, decimals)` - Absolute value, rounding
- `SQRT(value)`, `POWER(base, exp)` - Square root, exponentiation

**Text Functions:**
- `CONCATENATE(text1, text2, ...)` - Concatenate strings
- `LEFT(text, n)`, `RIGHT(text, n)`, `MID(text, start, n)` - Substrings
- `LEN(text)` - String length
- `UPPER(text)`, `LOWER(text)`, `TRIM(text)` - Case/whitespace

**Logical Functions:**
- `IF(condition, true_value, false_value)` - Conditional
- `AND(condition1, condition2, ...)` - Logical AND
- `OR(condition1, condition2, ...)` - Logical OR
- `NOT(condition)` - Logical NOT

**Date Functions:**
- `TODAY()`, `NOW()` - Current date/time
- `DATE(year, month, day)` - Create date
- `DATEDIF(start, end, unit)` - Date difference

**Lookup Functions:**
- `VLOOKUP(lookup_value, table_range, col_index, [range_lookup])` - Vertical lookup
- `COUNTIF(range, criteria)`, `SUMIF(range, criteria, [sum_range])` - Conditional aggregation
- `AVERAGEIF(range, criteria, [average_range])` - Conditional average

---

### 4. data-bridge-sheet-history

**Responsibility:** Undo/Redo system using Command pattern.

```rust
// Command trait
pub trait Command: Send + Sync {
    fn execute(&self, workbook: &mut Workbook) -> Result<(), Error>;
    fn undo(&self, workbook: &mut Workbook) -> Result<(), Error>;
    fn redo(&self, workbook: &mut Workbook) -> Result<(), Error> {
        self.execute(workbook)
    }
}

// Example: Set cell value command
pub struct SetCellCommand {
    row: u32,
    col: u32,
    new_value: CellValue,
    old_value: CellValue,
}

impl Command for SetCellCommand {
    fn execute(&self, workbook: &mut Workbook) -> Result<(), Error> {
        workbook.set_cell_value(self.row, self.col, self.new_value.clone())
    }

    fn undo(&self, workbook: &mut Workbook) -> Result<(), Error> {
        workbook.set_cell_value(self.row, self.col, self.old_value.clone())
    }
}

// History manager
pub struct HistoryManager {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_history: usize,
}

impl HistoryManager {
    pub fn execute(&mut self, command: Box<dyn Command>, workbook: &mut Workbook) -> Result<(), Error> {
        command.execute(workbook)?;
        self.undo_stack.push(command);
        self.redo_stack.clear();

        // Limit history size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }

        Ok(())
    }

    pub fn undo(&mut self, workbook: &mut Workbook) -> Result<(), Error> {
        if let Some(command) = self.undo_stack.pop() {
            command.undo(workbook)?;
            self.redo_stack.push(command);
            Ok(())
        } else {
            Err(Error::NoHistoryAvailable)
        }
    }

    pub fn redo(&mut self, workbook: &mut Workbook) -> Result<(), Error> {
        if let Some(command) = self.redo_stack.pop() {
            command.redo(workbook)?;
            self.undo_stack.push(command);
            Ok(())
        } else {
            Err(Error::NoRedoAvailable)
        }
    }
}
```

---

### 5. data-bridge-sheet-wasm

**Responsibility:** WebAssembly bindings for browser integration.

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RuSheet {
    workbook: Workbook,
    evaluator: FormulaEvaluator,
    history: HistoryManager,
}

#[wasm_bindgen]
impl RuSheet {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();

        Self {
            workbook: Workbook::new(),
            evaluator: FormulaEvaluator::new(),
            history: HistoryManager::new(1000),
        }
    }

    // Set cell value
    #[wasm_bindgen(js_name = setCellValue)]
    pub fn set_cell_value(&mut self, row: u32, col: u32, value: JsValue) -> Result<(), JsValue> {
        let cell_value = js_to_cell_value(value)?;

        let command = SetCellCommand {
            row,
            col,
            new_value: cell_value,
            old_value: self.workbook.get_cell_value(row, col).clone(),
        };

        self.history.execute(Box::new(command), &mut self.workbook)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // Get cell data
    #[wasm_bindgen(js_name = getCellData)]
    pub fn get_cell_data(&self, row: u32, col: u32) -> Result<JsValue, JsValue> {
        let cell = self.workbook.get_cell(row, col)?;

        // Convert to JavaScript object
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"value".into(), &cell_value_to_js(&cell.value))?;
        js_sys::Reflect::set(&obj, &"formula".into(), &JsValue::from_str(cell.formula.as_deref().unwrap_or("")))?;

        Ok(obj.into())
    }

    // Undo/Redo
    #[wasm_bindgen]
    pub fn undo(&mut self) -> Result<(), JsValue> {
        self.history.undo(&mut self.workbook)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn redo(&mut self) -> Result<(), JsValue> {
        self.history.redo(&mut self.workbook)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

---

### 6. data-bridge-sheet-server

**Responsibility:** Collaboration server with CRDT sync.

#### Architecture

```rust
// Axum server with WebSocket support
#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/workbooks", get(list_workbooks).post(create_workbook))
        .route("/api/workbooks/:id", get(get_workbook).put(update_workbook).delete(delete_workbook))
        .route("/ws/:workbook_id", get(websocket_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(workbook_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, workbook_id))
}

async fn handle_socket(socket: WebSocket, workbook_id: String) {
    let (mut sender, mut receiver) = socket.split();

    // Load Yjs document
    let doc = YDoc::new();

    // Subscribe to updates
    let subscription = doc.observe_update_v1(move |_txn, update| {
        // Broadcast update to all connected clients
        broadcast_update(&workbook_id, update);
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                // Apply update to Yjs document
                let update = Update::decode_v1(&data).unwrap();
                doc.apply_update(update);
            }
            _ => break,
        }
    }
}
```

---

## Performance Optimizations

### 1. Morton Encoding Benefits

**Spatial Locality:**
```
Traditional (row, col):
  (0,0), (0,1), (0,2), (1,0), (1,1), (1,2)
  Keys: random distribution

Morton encoding:
  (0,0)=0, (0,1)=1, (1,0)=2, (1,1)=3, (0,2)=4, (1,2)=6
  Keys: nearby cells have nearby keys
  Cache-friendly: sequential access
```

**Range Query Performance:**
```
Without Morton: O(n) scan of all cells
With Morton: O(log n) range query on sorted keys
```

### 2. Sparse Storage

Only non-empty cells consume memory:

```rust
// 1 million row × 1000 column sheet with 10,000 filled cells
Traditional: 1,000,000 × 1,000 × sizeof(Cell) = ~30 GB
Sparse: 10,000 × sizeof(Cell) = ~300 KB

Savings: 99.999%
```

### 3. Zero-copy WASM

Direct memory access from JavaScript:

```rust
#[wasm_bindgen]
pub fn get_viewport(&self, start_row: u32, start_col: u32, rows: u32, cols: u32) -> js_sys::Uint8Array {
    // Return raw bytes - no copying!
    unsafe {
        js_sys::Uint8Array::view(self.viewport_buffer.as_slice())
    }
}
```

### 4. Formula Caching

Lazy evaluation with dependency tracking:

```rust
// Only re-evaluate when dependencies change
if !self.cache.is_dirty(&cell_ref) {
    return self.cache.get(&cell_ref);
}
```

---

## Data Flow

### Cell Edit Flow

```
1. User types in cell → JavaScript event
2. JavaScript calls WASM: rusheet.setCellValue(row, col, value)
3. WASM creates SetCellCommand
4. Command executes → Workbook updated
5. Command pushed to undo stack
6. WASM broadcasts change event
7. JavaScript updates canvas
8. If collaboration enabled:
   - Change sent to Yjs
   - Yjs broadcasts to WebSocket
   - Server receives and stores
   - Server broadcasts to other clients
   - Other clients apply change via CRDT
```

### Formula Evaluation Flow

```
1. User enters formula: =SUM(A1:A10)
2. Parser creates AST: Function("SUM", [Range(A1, A10)])
3. Evaluator walks AST:
   - Range(A1:A10) → [value1, value2, ..., value10]
   - SUM function reduces: value1 + value2 + ... + value10
4. Result cached with dependencies: [A1, A2, ..., A10]
5. If A5 changes:
   - Dependency tracker detects
   - Formula re-evaluated
   - Display value updated
```

### Collaboration Sync Flow

```
1. User A edits cell (row=5, col=3, value="Hello")
2. Yjs creates operation: { type: "set", path: [5, 3], value: "Hello" }
3. Operation sent to server via WebSocket
4. Server applies operation to Y.Doc
5. Server persists to PostgreSQL
6. Server broadcasts to all clients
7. User B receives operation
8. Yjs applies operation (CRDT merge)
9. User B's workbook updated
10. User B's canvas re-rendered
```

---

## Future Enhancements

### Planned Features

1. **Array Formulas** - ARRAYFORMULA support
2. **Named Ranges** - Define named ranges for formulas
3. **Conditional Formatting** - Highlight cells based on rules
4. **Data Validation** - Dropdown lists, constraints
5. **Pivot Tables** - Interactive data summarization
6. **Charts** - Visualizations integrated with Chart.js

### Performance Improvements

1. **Incremental Formula Evaluation** - Only re-evaluate changed cells
2. **Parallel Formula Evaluation** - Evaluate independent formulas in parallel (Rayon)
3. **Streaming CRDT Sync** - Reduce memory usage for large documents
4. **Compression** - Compress cell data in database

---

## References

- [Morton Encoding (Z-order curve)](https://en.wikipedia.org/wiki/Z-order_curve)
- [CRDT (Conflict-free Replicated Data Types)](https://crdt.tech/)
- [Yjs CRDT Framework](https://github.com/yjs/yjs)
- [yrs (Rust implementation of Yjs)](https://github.com/y-crdt/y-crdt)
- [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/)
- [Axum Web Framework](https://github.com/tokio-rs/axum)

---

**Last Updated:** 2026-01-08
**Maintainer:** data-bridge team
