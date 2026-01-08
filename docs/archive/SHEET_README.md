# RuSheet

A high-performance spreadsheet engine built with Rust and WebAssembly.

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/typescript-5.0%2B-blue.svg)](https://www.typescriptlang.org/)

## Features

- **High Performance** - Rust-powered formula engine compiled to WebAssembly
- **Real-time Collaboration** - Multi-user editing with CRDT-based sync
- **Full Formula Support** - 24+ built-in functions (SUM, IF, VLOOKUP, etc.)
- **Undo/Redo** - Complete history with unlimited undo
- **Event-driven API** - Subscribe to cell changes, selections, and more
- **Zero-copy Rendering** - Direct memory access for optimal performance
- **Canvas Rendering** - Smooth scrolling with virtual grid

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/chrischeng-c4/rusheet.git
cd rusheet

# Install dependencies
pnpm install

# Build WASM module
just build-wasm

# Start development server
just dev
```

### Basic Usage

```typescript
import { rusheet } from 'rusheet';

// Initialize the engine
await rusheet.init();

// Set cell values
rusheet.setCellValue(0, 0, 'Hello');
rusheet.setCellValue(0, 1, 'World');
rusheet.setCellValue(1, 0, '=A1 & " " & B1');

// Get cell data
const cell = rusheet.getCellData(1, 0);
console.log(cell.displayValue); // "Hello World"

// Subscribe to changes
rusheet.onChange((event) => {
  console.log(`Cell ${event.row},${event.col} changed to ${event.newValue}`);
});
```

### React Component

```tsx
import { RuSheet, useRuSheet } from 'rusheet/react';

function App() {
  const { ref, api } = useRuSheet();

  return (
    <RuSheet
      ref={ref}
      initialData={[
        ['Name', 'Age', 'City'],
        ['Alice', 30, 'NYC'],
        ['Bob', 25, 'LA'],
      ]}
      onChange={(e) => console.log('Changed:', e)}
      width="100%"
      height={500}
    />
  );
}
```

See [examples/react-basic.tsx](examples/react-basic.tsx) for more examples.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (Browser)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Canvas      │  │  Yjs Client │  │   RuSheet API       │  │
│  │ Renderer    │  │  (collab)   │  │   (TypeScript)      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                          │                    │
                    WebSocket              WASM Bridge
                          │                    │
                          ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                    rusheet-wasm (WASM)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ rusheet-core │  │rusheet-formula│ │  rusheet-history │   │
│  │ (cells/grid) │  │   (parser)   │  │   (undo/redo)    │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  rusheet-server (Rust)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Axum HTTP   │  │  yrs (CRDT)  │  │    PostgreSQL    │   │
│  │  WebSocket   │  │              │  │                  │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Description |
|-------|-------------|
| `rusheet-core` | Core data structures (cells, sheets, formatting) |
| `rusheet-formula` | Formula parser and evaluator |
| `rusheet-history` | Undo/redo command system |
| `rusheet-wasm` | WebAssembly bindings |
| `rusheet-server` | Collaboration server with REST API |

## API Reference

### Cell Operations

```typescript
// Set cell value (supports formulas)
rusheet.setCellValue(row, col, value);

// Get cell data
const cell = rusheet.getCellData(row, col);
// Returns: { value, displayValue, formula?, format }

// Clear a range
rusheet.clearRange(startRow, startCol, endRow, endCol);
```

### Formatting

```typescript
// Set cell format
rusheet.setCellFormat(row, col, {
  bold: true,
  fontSize: 14,
  textColor: '#FF0000',
  backgroundColor: '#FFFF00',
  horizontalAlign: 'center'
});

// Format a range
rusheet.setRangeFormat(0, 0, 10, 5, { bold: true });
```

### Row/Column Operations

```typescript
// Insert rows
rusheet.insertRows(atRow, count);

// Delete rows
rusheet.deleteRows(atRow, count);

// Insert columns
rusheet.insertCols(atCol, count);

// Delete columns
rusheet.deleteCols(atCol, count);
```

### Sheet Management

```typescript
// Add a new sheet
rusheet.addSheet('Sheet2');

// Switch active sheet
rusheet.setActiveSheet(1);

// Get sheet names
const sheets = rusheet.getSheetNames(); // ['Sheet1', 'Sheet2']

// Rename sheet
rusheet.renameSheet(0, 'Data');

// Delete sheet
rusheet.deleteSheet(1);
```

### History

```typescript
// Undo/Redo
rusheet.undo();
rusheet.redo();

// Check availability
if (rusheet.canUndo()) { /* ... */ }
if (rusheet.canRedo()) { /* ... */ }
```

### Events

```typescript
// Cell changes
rusheet.onChange((event) => {
  console.log(event.row, event.col, event.newValue);
});

// Selection changes
rusheet.onSelectionChange((event) => {
  console.log(event.row, event.col);
});

// Format changes
rusheet.onFormatChange((event) => {
  console.log(event.format);
});

// Sheet operations
rusheet.onSheetAdd((event) => console.log('Added:', event.name));
rusheet.onSheetDelete((event) => console.log('Deleted:', event.name));
rusheet.onSheetRename((event) => console.log('Renamed:', event.newName));

// Row/Column operations
rusheet.onRowsInsert((event) => console.log('Inserted rows at:', event.atRow));
rusheet.onColsDelete((event) => console.log('Deleted cols at:', event.atCol));
```

### Serialization

```typescript
// Save to JSON
const json = rusheet.serialize();
localStorage.setItem('workbook', json);

// Load from JSON
const saved = localStorage.getItem('workbook');
rusheet.deserialize(saved);
```

## Real-time Collaboration

RuSheet includes a collaboration server for multi-user editing:

```bash
# Start PostgreSQL
just db-up

# Start the collaboration server
just server

# Open the app with collaboration
open http://localhost:5173?workbook=<uuid>
```

### Server API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/workbooks` | GET | List all workbooks |
| `/api/workbooks` | POST | Create a workbook |
| `/api/workbooks/{id}` | GET | Get workbook details |
| `/api/workbooks/{id}` | PUT | Update workbook |
| `/api/workbooks/{id}` | DELETE | Delete workbook |
| `/ws/{workbook_id}` | WS | Real-time collaboration |

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+
- pnpm
- wasm-pack
- just (command runner)

### Commands

```bash
# Build WASM
just build-wasm

# Run dev server
just dev

# Run tests
just test-rust      # Rust tests
just test-unit      # TypeScript unit tests
just test-integration # Browser integration tests

# Type check
just check

# Format code
just fmt

# Build for production
just build
```

### Project Structure

```
rusheet/
├── crates/
│   ├── rusheet-core/       # Core data structures
│   ├── rusheet-formula/    # Formula engine
│   ├── rusheet-history/    # Undo/redo
│   ├── rusheet-wasm/       # WASM bindings
│   └── rusheet-server/     # Collaboration server
├── src/                    # TypeScript frontend
│   ├── core/               # API and state
│   ├── canvas/             # Rendering
│   ├── ui/                 # UI components
│   ├── collab/             # Collaboration client
│   └── worker/             # Web Worker
├── pkg/                    # Built WASM package
├── docs/                   # VitePress documentation
└── migrations/             # Database migrations
```

## Supported Formulas

### Math Functions
`SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT`, `ABS`, `ROUND`, `FLOOR`, `CEILING`, `SQRT`, `POWER`, `MOD`

### Text Functions
`CONCATENATE`, `LEFT`, `RIGHT`, `MID`, `LEN`, `UPPER`, `LOWER`, `TRIM`

### Logical Functions
`IF`, `AND`, `OR`, `NOT`

### Lookup Functions
`VLOOKUP` (coming soon), `HLOOKUP` (coming soon)

## Performance

- **Morton-indexed chunks**: O(1) cell lookup in 64x64 chunks
- **Sparse storage**: Only non-empty cells consume memory
- **Zero-copy viewport**: Direct memory access from JavaScript
- **Formula caching**: Lazy evaluation with dependency tracking
- **Web Worker rendering**: Non-blocking canvas updates

## Roadmap

See [TODOS.md](TODOS.md) for the complete development roadmap.

**Upcoming:**
- [ ] CSV/XLSX import/export
- [ ] Cross-sheet references
- [ ] Advanced lookup functions (VLOOKUP, INDEX, MATCH)
- [x] React component wrapper (`rusheet/react`)
- [ ] Vue component wrapper
- [ ] npm/crates.io publishing

## Contributing

Contributions are welcome! Please read our [contributing guidelines](CONTRIBUTING.md).

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [yrs](https://github.com/y-crdt/y-crdt) - Rust implementation of Yjs CRDT
- [Axum](https://github.com/tokio-rs/axum) - Web framework for Rust
- [wasm-pack](https://github.com/rustwasm/wasm-pack) - Rust to WASM toolchain
- [nom](https://github.com/rust-bakery/nom) - Parser combinators for formula parsing
