# User Experience Specification

## Overview

RuSheet aims to provide a native-like spreadsheet experience that rivals Google Sheets and Excel. This requires precise control over input handling, selection models, and visual feedback, adhering to the "Hybrid Rendering" architecture.

## Interaction Models

### 1. Selection Model

The selection model is the core of user interaction, handling single clicks, range drags, and multi-selections.

#### State Definition

```rust
pub struct SelectionState {
    // The cell where the mouse button went down
    pub anchor: CellCoord,
    // The current cell under the mouse cursor
    pub focus: CellCoord,
    // Additional disjoint ranges (for Ctrl+Click)
    pub ranges: Vec<CellRange>,
}
```

#### Behaviors

| Action | Result | Visual Feedback |
| :--- | :--- | :--- |
| **Click** | `anchor` = `focus` = clicked cell. Clear `ranges`. | Blue border on single cell. |
| **Drag** | Update `focus` to current cell. `anchor` remains fixed. | Semi-transparent blue overlay from `anchor` to `focus`. |
| **Ctrl+Click** | Add current range to `ranges`. Start new selection. | Multiple disjoint blue overlays. |
| **Shift+Click** | Extend selection from `anchor` to clicked cell. | Range expands. |
| **Row/Col Header Click** | Select entire row/column. | Full row/col highlighted. |

### 2. Editing System (DOM Overlay)

To support IME (Input Method Editors), accessibility, and native text features, editing is **not** done directly on the Canvas.

#### Activation Flow
1.  **User Action**: Double-click cell OR press `Enter` / `F2`.
2.  **Engine Calculation**: Determine pixel bounds `(x, y, w, h)` of the active cell.
3.  **DOM Overlay**:
    *   Create/Show a `<textarea>` element absolutely positioned at `(x, y)`.
    *   Sync CSS properties: `font`, `fontSize`, `lineHeight`, `padding`, `textAlign`.
    *   Set `<textarea>` value to cell's raw formula/content.
    *   **Hide Canvas Text**: Temporarily stop rendering text for that cell on Canvas to prevent anti-aliasing artifacts/double vision.

#### Deactivation (Commit) Flow
1.  **User Action**: Click outside, press `Enter` (commit), or `Esc` (cancel).
2.  **Commit**: Send content string to Rust engine via WASM.
3.  **Cleanup**: Hide/Remove `<textarea>`.
4.  **Re-render**: Canvas draws the new value.

### 3. Fill Handle

The small square at the bottom-right of the selection allows "Drag-to-Fill".

#### Logic
*   **Arithmetic Progression**: If `1, 2` is selected -> generates `3, 4, 5`.
*   **Copy**: If `A` is selected -> generates `A, A, A`.
*   **Formulas**: Adjust relative references (e.g., `=A1` becomes `=A2` when dragged down).

#### Interaction
1.  **Hover**: Mouse over bottom-right corner -> Cursor changes to `crosshair`.
2.  **Drag**: Draw a dashed "preview border" showing the target range.
3.  **Release**: Trigger `FillSeriesCommand` in Rust.

## Navigation & Shortcuts

Full keyboard support is mandatory for power users.

| Key | Action | Context |
| :--- | :--- | :--- |
| **Arrows** | Move active cell (`focus` & `anchor`). | Grid |
| **Shift + Arrows** | Expand selection (move `focus` only). | Grid |
| **Ctrl + Arrows** | Jump to data edge (last non-empty cell). | Grid |
| **Tab** | Move right. | Grid |
| **Shift + Tab** | Move left. | Grid |
| **Enter** | Move down (or commit edit). | Grid / Edit |
| **Shift + Enter** | Move up. | Grid |
| **F2** | Enter Edit Mode (jump cursor to end). | Grid |
| **Home** | Jump to start of row (Col A). | Grid |
| **Ctrl + Home** | Jump to A1. | Grid |
| **Page Up/Down** | Scroll viewport by one screen height. | Grid |
| **Delete / Backspace** | Clear content of selected cells. | Grid |
| **Ctrl + Z** | Undo. | Global |
| **Ctrl + Y / Ctrl + Shift + Z** | Redo. | Global |
| **Ctrl + C** | Copy to system clipboard. | Grid |
| **Ctrl + V** | Paste from system clipboard (parse TSV). | Grid |

## Virtual Scrolling

The scrollbar acts as a "remote control" for the viewport.

*   **Fake Scroll Container**: A `<div>` with `height = total_rows * default_height`.
*   **Event Listener**: Listen to `onScroll` of this container.
*   **Sync**:
    1.  Get `scrollTop`.
    2.  Pass to Rust: `engine.update_viewport(scrollTop)`.
    3.  Rust calculates `start_row` using **Fenwick Tree**.
    4.  Canvas renders `start_row` to `end_row`.

## Accessibility (a11y)

*   **Screen Reader Support**: The DOM Overlay `<textarea>` is focusable but only exists during edit. For navigation, we maintain a hidden, focused DOM element that receives keyboard events and has `aria-live` attributes to announce the current cell coordinates and value.
*   **High Contrast**: Ensure grid lines and selection borders meet WCAG contrast ratios.

## References

*   [Google Sheets Keyboard Shortcuts](https://support.google.com/docs/answer/181110)
*   [WAI-ARIA Grid Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/)