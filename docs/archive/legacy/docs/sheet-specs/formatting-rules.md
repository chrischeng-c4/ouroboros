# Formatting Rules Specification

## Overview

High-fidelity formatting is essential for a "Google Sheets-like" experience. This specification covers Number Formatting, Text Layout (wrapping/overflow), and Cell Borders.

## 1. Number Formatting

Raw numbers in Rust (`f64`) need to be formatted into display strings based on Excel-compatible format strings.

### Format Types
*   **General**: Default. Tries to be smart.
*   **Number**: `0.00`, `#,##0`.
*   **Currency**: `$#,##0.00`.
*   **Percentage**: `0%` (multiplies by 100).
*   **Date/Time**: `dd/mm/yyyy`, `hh:mm:ss`. *Note: Dates are stored as floats (days since epoch).*
*   **Text**: `@` (Treat input as literal text).

### Implementation
Use a crate like `excel-number-format` or implement a subset manually in `rusheet-core`.

```rust
pub struct CellFormat {
    pub number_format: String, // e.g. "#,##0.00"
    // ...
}

// In Render Loop:
let display_value = formatter.format(cell.value, &cell.format.number_format);
```

## 2. Text Layout & Wrapping

Canvas `fillText` does not support wrapping. We must calculate it manually in Rust or the Renderer.

### Modes
1.  **Overflow (Default)**: Text bleeds into empty adjacent cells (Right). If neighbor not empty, clip.
2.  **Wrap**: Text breaks into multiple lines. Row height *may* need to auto-grow.
3.  **Clip**: Text is cut off at the cell boundary.

### Wrapping Algorithm (Canvas Helper)
Since `measureText` is a Canvas API, exact wrapping is best calculated in the **Frontend (Renderer)** or via a WASM helper that calls out to Canvas.

*   **Logic**:
    1.  Split text into words.
    2.  Measure words.
    3.  Fit into `col_width`.
    4.  Return `lines: Vec<String>`.
*   **Rendering**: Draw line by line, incrementing `y` by `lineHeight`.

## 3. Borders

Borders are complex because adjacent cells share boundaries.

### Data Structure
Borders are stored per cell, but rendering must resolve conflicts (e.g., Left border of B1 vs Right border of A1).

```rust
pub struct Borders {
    pub top: Option<BorderStyle>,
    pub bottom: Option<BorderStyle>,
    pub left: Option<BorderStyle>,
    pub right: Option<BorderStyle>,
}

pub struct BorderStyle {
    pub style: LineStyle, // Solid, Dashed, Dotted, Double
    pub width: u8,        // Thin, Medium, Thick
    pub color: String,    // Hex
}
```

### Rendering Logic
1.  **Conflict Resolution**: Usually, "Thick" wins over "Thin". "Right" wins over "Left"? (Excel has specific precedence rules).
2.  **Drawing Order**: Draw borders *after* cell backgrounds and grid lines.
3.  **Optimization**: Draw borders as continuous lines where possible, rather than 4 segments per cell.

## 4. Vertical Alignment

*   **Top**: Draw text at `y + padding`.
*   **Middle**: Draw text at `y + (height - text_height) / 2`.
*   **Bottom**: Draw text at `y + height - text_height - padding`.

## 5. Merged Cells

### Logic
*   **Storage**: `HashMap<CellCoord, (row_span, col_span)>`.
*   **Constraint**: Only the Top-Left cell holds the value/format.
*   **Rendering**:
    1.  When iterating cells, check if `(row, col)` is a merge start.
    2.  If yes, calculate total width/height of the spanned area.
    3.  Draw background/text over that total area.
    4.  Skip rendering for other cells within the merge range.
