# Keyboard Shortcuts Specification

## Overview

This document outlines the complete keyboard shortcut mappings for RuSheet. These shortcuts are designed to match standard spreadsheet behavior (Excel/Google Sheets) to ensure zero learning curve for users.

## Navigation

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| `Arrow Keys` | Move active cell selection one step. | Grid |
| `Shift + Arrow Keys` | Extend selection range from anchor. | Grid |
| `Ctrl + Arrow Keys` (Cmd on Mac) | Jump to the edge of the current data region. | Grid |
| `Ctrl + Shift + Arrow Keys` | Extend selection to the edge of the data region. | Grid |
| `Tab` | Move active cell right. | Grid |
| `Shift + Tab` | Move active cell left. | Grid |
| `Enter` | Move active cell down (or commit edit). | Grid / Editor |
| `Shift + Enter` | Move active cell up. | Grid |
| `Home` | Move to the beginning of the row (Column A). | Grid |
| `Ctrl + Home` (Cmd+Home on Mac) | Move to cell A1. | Grid |
| `Ctrl + End` (Cmd+End on Mac) | Move to the last used cell (bottom-right of data). | Grid |
| `Page Up` | Scroll up one viewport height. | Grid |
| `Page Down` | Scroll down one viewport height. | Grid |
| `Alt + Page Up` | Scroll left one viewport width. | Grid |
| `Alt + Page Down` | Scroll right one viewport width. | Grid |

## Editing

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| `F2` | Enter Edit Mode (cursor at end of content). | Grid |
| `Enter` | Enter Edit Mode (if not editing), Commit changes (if editing). | Grid / Editor |
| `Esc` | Cancel editing and discard changes. | Editor |
| `Delete` / `Backspace` | Clear content of selected cell(s). | Grid |
| `Alt + Enter` | Insert new line within a cell. | Editor |

## Clipboard & History

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| `Ctrl + C` (Cmd+C) | Copy selected cells to clipboard. | Grid |
| `Ctrl + X` (Cmd+X) | Cut selected cells to clipboard. | Grid |
| `Ctrl + V` (Cmd+V) | Paste from clipboard. | Grid |
| `Ctrl + Z` (Cmd+Z) | Undo last action. | Global |
| `Ctrl + Y` (Cmd+Y) or `Ctrl + Shift + Z` | Redo last undone action. | Global |

## Formatting

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| `Ctrl + B` (Cmd+B) | Toggle **Bold**. | Grid |
| `Ctrl + I` (Cmd+I) | Toggle *Italic*. | Grid |
| `Ctrl + U` (Cmd+U) | Toggle <u>Underline</u>. | Grid |
| `Ctrl + \` | Clear formatting. | Grid |

## Selection

| Shortcut | Action | Scope |
| :--- | :--- | :--- |
| `Ctrl + A` (Cmd+A) | Select all (or current data region). | Grid |
| `Shift + Space` | Select entire row. | Grid |
| `Ctrl + Space` | Select entire column. | Grid |

## Implementation Notes

1.  **Event Handling**: Keyboard events should be captured globally on the window/document when the grid has focus.
2.  **Platform Detection**: The key handler must detect macOS (`navigator.platform`) to swap `Ctrl` with `Cmd` (Meta key) appropriately.
3.  **Conflict Prevention**: Prevent default browser behaviors for shortcuts like `Ctrl+S` (Save) or `Ctrl+P` (Print) if we intend to override them.
4.  **Editor Context**: When the DOM overlay (`<textarea>`) is active, navigation keys (Arrows) should move the text cursor *inside* the textarea, not change the selected cell, unless modifier keys (like `Enter` or `Tab`) are used to commit.