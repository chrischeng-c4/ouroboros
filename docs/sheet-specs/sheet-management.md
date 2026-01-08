# Sheet Management Specification

## Overview

A Workbook consists of multiple Sheets. Users need to be able to manage these sheets similarly to Google Sheets/Excel.

**Architecture Note**: The Sheet Tab Bar is a **React/DOM Component**. It communicates with the WASM engine to update the model, but rendering and interaction (drag-and-drop, context menus) are handled in HTML/CSS.

## Data Structure (Rust Core)

```rust
// rusheet-core/src/data/workbook.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
    pub active_sheet_index: usize,
    pub next_sheet_id: u32,
}

// rusheet-core/src/data/sheet.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    pub id: u32,             // Stable ID
    pub name: String,
    pub tab_color: Option<String>,
    pub hidden: bool,
    // ... cells, rows, cols ...
}
```

## UI Component (React)

The `SheetBar` component resides at the bottom of the screen.

### Features
1.  **Sheet Tabs**:
    *   Button-like elements.
    *   Active tab has distinct white background/border.
    *   Colored strip at bottom if `tab_color` is set.
2.  **Add Button**: `+` icon to create new sheet.
3.  **List Button**: Hamburger menu to list all sheets (useful when many sheets overflow).
4.  **Scroll Controls**: Left/Right arrows if tabs overflow the viewport.

### Interactions

| Action | Logic |
| :--- | :--- |
| **Click Tab** | `engine.setActiveSheet(index)` -> Engine triggers re-render of Grid. |
| **Double Click** | Switch tab to `<input>` mode for renaming. On Blur/Enter -> `engine.renameSheet(id, newName)`. |
| **Drag & Drop** | Use HTML5 DnD API or library (dnd-kit). On drop -> `engine.reorderSheet(fromIndex, toIndex)`. |
| **Right Click** | Prevent default. Show custom DOM Context Menu. |

## Operations & Logic

### 1. Add Sheet
*   **Logic**: Generate unique name `SheetN`. Create new empty `Sheet` in Rust.
*   **Undo**: `RemoveSheetCommand`.

### 2. Rename Sheet
*   **Validation**: Non-empty, unique, no invalid chars (`: \ / ? * [ ]`).
*   **Impact**:
    *   Update `name` in struct.
    *   **Formula Refactoring**: Search all formulas in *all* sheets. If they reference `'Old Name'!A1`, update to `'New Name'!A1`.

### 3. Delete Sheet
*   **Validation**: Cannot delete last visible sheet.
*   **Impact**: Formulas referencing this sheet become `#REF!`.
*   **Undo**: Restore sheet and data.

### 4. Duplicate Sheet
*   **Logic**: Deep clone sheet. Name: `Copy of [Name]`.

### 5. Hide/Unhide
*   **Logic**: `hidden = true`.
*   **UI**: Tab disappears. Accessible only via "All Sheets" menu.

## WASM API

The frontend needs `SheetInfo` structs to render the tabs without fetching all cell data.

```rust
#[derive(Serialize)]
pub struct SheetInfo {
    pub id: u32,
    pub name: String,
    pub color: Option<String>,
    pub hidden: bool,
}

impl Workbook {
    pub fn get_sheet_info(&self) -> Vec<SheetInfo>;
}
```
