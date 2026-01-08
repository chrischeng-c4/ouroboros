# Persistence Specification

## Overview

RuSheet needs to save and load workbooks. Since it runs in the browser, "persistence" means:
1.  **Serialization**: Converting the Rust `Workbook` struct to a portable format.
2.  **Storage**: Saving to `IndexedDB`, `localStorage`, or downloading as a file.

## 1. File Format (`.rusheet` or `.json`)

We use a standard JSON structure that mirrors the internal Rust state but optimized for size where possible (using sparse maps).

### Schema

```json
{
  "version": "1.0",
  "meta": {
    "created": 1234567890,
    "author": "User",
    "lastModified": 1234567890
  },
  "sheets": [
    {
      "id": 1,
      "name": "Sheet1",
      "order": 0,
      "config": {
        "frozenRows": 0,
        "frozenCols": 0,
        "zoom": 1.0,
        "gridLines": true
      },
      "columns": {
        "0": { "width": 120 },
        "2": { "width": 200 }
      },
      "rows": {
        "0": { "height": 40 }
      },
      "cells": {
        "0,0": {
          "v": 100,             // value
          "f": "=SUM(A1:A10)",  // formula (optional)
          "s": 1                // style ID (optional)
        },
        "0,1": { "v": "Hello" }
      },
      "merges": [
        "A1:B2"
      ]
    }
  ],
  "styles": {
    "1": {
      "bold": true,
      "color": "#FF0000",
      "align": "center"
    }
  },
  "names": [
    { "name": "TaxRate", "ref": "Sheet1!$Z$1" }
  ]
}
```

### Optimization
*   **Style Dictionary**: Instead of storing `{ bold: true }` on every cell, we store a `style_id` and a lookup table.
*   **Sparse Coordinates**: Cells are stored as `"row,col"` keys in a map, not a 2D array.

## 2. Storage Mechanisms

### 2.1 Browser Storage (IndexedDB)
*   **Use Case**: Auto-save, "Recent Files".
*   **Library**: `idb-keyval` (JS) or pure Rust `web-sys` calls.
*   **Flow**:
    *   `onEdit` -> Debounce (1s) -> Serialize -> Save to IDB.

### 2.2 File Export/Import
*   **Export**:
    *   Serialize to string.
    *   Create `Blob`.
    *   Trigger download (`<a download="sheet.json">`).
*   **Import**:
    *   File Input (`<input type="file">`).
    *   FileReader -> Text.
    *   Pass string to `engine.load_json(text)`.

### 2.3 Excel Compatibility (Future)
*   To support `.xlsx`, we need a heavy parser/writer.
*   **Strategy**: Use `calamine` (Read) and `rust_xlsxwriter` (Write) crates in WASM.
    *   *Note*: These crates might be heavy for WASM. If too big, move import/export to a Serverless Function or Web Worker.

## 3. Implementation

### Rust Trait
```rust
pub trait Persistable {
    fn to_json(&self) -> String;
    fn from_json(json: &str) -> Result<Self, Error>;
}
```

### Versioning
*   Include `"version": "1.0"`.
*   If we change the schema, the loader must handle migrations (e.g., adding default values for new fields).
