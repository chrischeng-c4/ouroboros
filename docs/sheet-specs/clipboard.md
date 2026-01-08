# Clipboard Specification

## Overview

Clipboard operations (`Cut`, `Copy`, `Paste`) are critical for data interoperability. RuSheet must support both **Internal Clipboard** (preserving full fidelity of formulas/styles) and **System Clipboard** (interop with Excel/Google Sheets/Notepad).

## 1. Copy (`Ctrl+C`)

When the user copies a selection, we must write multiple formats to the System Clipboard to ensure maximum compatibility.

### 1.1 Formats

| MIME Type | Content | Purpose |
| :--- | :--- | :--- |
| `text/plain` | TSV (Tab-Separated Values) | Basic text editors, Excel (simple paste). |
| `text/html` | HTML Table `<table>...</table>` | Excel, Google Sheets, Word (preserves formatting). |
| `application/json` | RuSheet Internal JSON | Full fidelity copy within RuSheet (formulas, exact styles). |

### 1.2 TSV Generation
*   **Logic**: Iterate rows/cols in selection.
*   **Separator**: `\t`.
*   **Newline**: `\r\n` (Windows style for max compat) or `\n`.
*   **Quoting**: If cell contains `\t`, `\n`, or `"`, wrap in `"` and escape internal `"` as `""`.

### 1.3 HTML Generation
*   Structure:
    ```html
    <style>
      <!-- CSS matching cell styles -->
    </style>
    <table>
      <tr>
        <td style=\"...">Value</td>
        ...
      </tr>
    </table>
    ```
*   **Styles**: Convert Rust `CellFormat` to CSS (`font-weight: bold`, `background-color: #Hex`, etc.).

## 2. Paste (`Ctrl+V`)

Paste is complex because we must guess the source and intent.

### 2.1 Parsing Priority
1.  **Internal JSON**: If present, use it. This allows exact cloning of formulas, dependencies, and complex formatting.
2.  **HTML**: If JSON missing, parse HTML `<table>`. Good for pasting from web or Excel.
    *   *Challenge*: Parsing HTML in WASM or JS? -> JS `DOMParser` is easiest. Extract data, send to WASM.
3.  **TSV / Text**: Fallback.
    *   Split by `\n`, then by `\t`.
    *   Handle quoting rules.

### 2.2 Paste Logic
*   **Single Cell Target**: If `activeCell` is single, expand the paste range to match the source size.
*   **Range Target**: If target selection matches source size (or is a multiple), paste into it. If size mismatch, warn or paste top-left.
*   **Formula Adjustment**:
    *   If pasting formulas, relative references (`=A1`) must be shifted by the delta `(target_row - source_row, target_col - source_col)`.

### 2.3 Cut (`Ctrl+X`)
*   **Action**: Copy to clipboard + Mark selection as "Cut Mode" (dashed moving border).
*   **On Paste**:
    1.  Perform Paste.
    2.  Clear content/formats from original source.
    3.  *Important*: Update references! (Moving a cell should update formulas pointing TO it, unlike Copy).

## 3. Security
*   Web Clipboard API requires user gesture (click/key press) and sometimes permission prompts.
*   We must handle `navigator.clipboard.read()` and `write()` in `GridController`.

## 4. Implementation Plan

### Frontend (`GridController`)
*   Listen for `keydown` (`Ctrl+C`, `Ctrl+V`, `Ctrl+X`).
*   **Copy**:
    *   Call `engine.serializeRange(selection, "TSV")`.
    *   Call `engine.serializeRange(selection, "HTML")`.
    *   Call `navigator.clipboard.write(...)`.
*   **Paste**:
    *   Call `navigator.clipboard.read()`.
    *   If text, call `engine.pasteTSV(activeCell, text)`.
    *   If html, parse in JS -> Convert to simplified JSON -> `engine.pasteData(activeCell, data)`.

### Backend (`rusheet-core`)
*   `RangeSerializer`: Trait for exporting ranges.
*   `TSVParser`: Robust state machine for parsing quoted CSV/TSV.
*   `PasteCommand`: History-aware command to insert data.
