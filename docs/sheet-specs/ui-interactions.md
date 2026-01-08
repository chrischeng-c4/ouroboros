# UI Interactions Specification

## Overview

This document details the "Chrome" interactions: Context Menus, Resizing, and other mouse-driven behaviors that happen outside the main cell editing flow.

## 1. Context Menus (Right Click)

We replace the browser default menu with custom DOM menus.

### 1.1 Grid Context Menu
*   **Trigger**: Right-click on any cell.
*   **Items**:
    *   Cut / Copy / Paste
    *   ---
    *   Insert Row Above / Below
    *   Insert Column Left / Right
    *   Delete Row / Column
    *   Delete Cells... (Shift Up/Left)
    *   ---
    *   Clear Content
    *   ---
    *   Format Cells... (Dialog)
    *   Conditional Formatting...

### 1.2 Header Context Menu (Row/Col)
*   **Trigger**: Right-click on `1, 2...` or `A, B...`.
*   **Items**:
    *   Insert 1 Left/Right
    *   Delete Column
    *   Clear Column
    *   ---
    *   Resize Column...
    *   Hide Column
    *   Unhide Column (if selection spans hidden)

### 1.3 Sheet Tab Context Menu
(Defined in `sheet-management.md`)

## 2. Resizing Rows/Columns

### 2.1 Drag Resize
*   **Hit Test**: Hovering over the separator line in the Header (e.g., between A and B).
    *   *Zone*: +/- 3px from the line.
*   **Cursor**: Change to `col-resize` or `row-resize`.
*   **Interaction**:
    1.  **MouseDown**: Capture initial `x` or `y`. Show a "Guide Line" (vertical line across the whole screen).
    2.  **Drag**: Update Guide Line position.
    3.  **MouseUp**: Calculate `delta`. Call `engine.setColWidth(col, newWidth)`.
    4.  **Optim**: Do not re-render grid during drag (expensive). Just render the Guide Line. Re-render on drop.

### 2.2 Auto-Fit (Double Click)
*   **Trigger**: Double-click the resize handle.
*   **Logic**:
    1.  **Frontend**: Call `engine.calculateAutoWidth(col)`.
    2.  **Backend (Rust)**:
        *   Iterate all cells in that column (in the current view? or all data?). *Google Sheets scans first ~1000 rows.*
        *   Calculate max string length.
        *   Return pixel width estimate (approximate char width * length).
    3.  **Apply**: `engine.setColWidth(col, calculatedWidth)`.

## 3. Scrollbars

*   **Native Feel**: We use a "Fake Scroll" div overlays or alongside the canvas.
*   **Syncing**:
    *   Scroll event -> `requestAnimationFrame` -> `engine.updateViewport` -> `render`.
*   **Sticky Headers**:
    *   Headers are drawn *after* the scroll transformation is applied to the grid, effectively making them "fixed" relative to the container frame.

## 4. Frozen Panes (Freeze Rows/Cols)

### UI
*   **Drag Handle**: Thick gray bars at the top-left (between A and 1 headers).
*   **Action**: Drag bar down to freeze Row 1, 2...
*   **Rendering**:
    *   The Grid is split into 4 viewports logically.
    *   **Zone 1 (Top-Left)**: Fixed.
    *   **Zone 2 (Top-Right)**: Scrolls X only.
    *   **Zone 3 (Bottom-Left)**: Scrolls Y only.
    *   **Zone 4 (Bottom-Right)**: Fully scrollable.
