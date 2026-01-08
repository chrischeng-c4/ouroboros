# data-bridge-sheet-core - Implementation Todos

**Last Updated:** 2026-01-08

**Status:** Migrated from rusheet to data-bridge project.

## Migration Summary

The rusheet project has been successfully integrated into the data-bridge platform as the `data-bridge-sheet-*` family of crates:

- **data-bridge-sheet-core** - Core data structures (cells, sheets, formatting)
- **data-bridge-sheet-db** - Custom database with Morton encoding
- **data-bridge-sheet-formula** - Formula parser and evaluator (24+ functions)
- **data-bridge-sheet-history** - Undo/redo command system
- **data-bridge-sheet-server** - Collaboration server (Axum + Yjs)
- **data-bridge-sheet-wasm** - WebAssembly bindings

This document tracks the ongoing development roadmap for the spreadsheet engine within data-bridge.

**Legend**: `[ ]` pending | `[x]` done | `[!]` blocked | `[~]` in progress

**Target Users:** Developers (embedded SDK) + Enterprise users

---

## Google Sheets 差距分析

### 公式功能 (24 個 vs Google 400+)
| 缺少功能 | 優先級 |
|----------|--------|
| 跨工作表引用 `Sheet2!A1` | P1 | ✅ 已完成 |
| 陣列公式 `ARRAYFORMULA` | P2 |
| 命名範圍 | P2 |
| COUNTIF, SUMIF, AVERAGEIF | P1 | ✅ 已完成 |
| DATE, TODAY, NOW, DATEDIF | P1 | ✅ 已完成 |
| INDEX, MATCH, OFFSET | P2 |
| FIND, SEARCH, SUBSTITUTE | P2 |

### 資料功能
| 缺少功能 | 優先級 | 狀態 |
|----------|--------|------|
| 排序（單欄/多欄）| P1 | ✅ 已完成 |
| 合併儲存格 | P1 | ✅ 已完成 |
| 篩選/自動篩選 | P1 | ✅ 已完成 |
| 條件格式 | P1 | ❌ |
| 資料驗證（下拉選單）| P1 | ❌ |
| 樞紐分析表 | P3 | ❌ |
| 圖表 | P3 | ❌ |

### 編輯功能
| 缺少功能 | 優先級 |
|----------|--------|
| 合併儲存格 | P1 |
| 尋找和取代 | P2 |
| 特殊貼上（僅值、轉置）| P2 |
| 自動填入 | P2 |

### 匯入/匯出
| 缺少功能 | 優先級 | 狀態 |
|----------|--------|------|
| CSV 匯入/匯出 | P0 | ✅ 已完成 |
| Excel 匯入 (.xlsx) | P1 | ✅ 已完成 |
| Excel 匯出 (.xlsx) | P2 | ✅ 已完成 |
| PDF 匯出 | P3 | ❌ |

### 協作功能
| 缺少功能 | 優先級 | 狀態 |
|----------|--------|------|
| 游標追蹤（顯示其他人位置）| P1 | ✅ 已完成 |
| 評論系統 | P2 | ❌ |
| 版本歷史 | P2 | ❌ |
| 權限控制（查看/編輯）| P1 | ❌ |

---

## Open-Source Readiness Checklist

### Documentation & Packaging (Critical)

- [x] **README.md** - Project overview, quick start, badges ✅ (2025-12-30)
- [x] **LICENSE file** - MIT license file in repository root ✅ (2025-12-30)
- [x] **package.json metadata** - Complete npm package info ✅ (2025-12-30)
- [x] **CONTRIBUTING.md** - Contribution guidelines ✅ (2025-12-30)

- [ ] **CHANGELOG.md** - Release notes history
  - Follow Keep a Changelog format
  - Semantic versioning

- [ ] **GitHub Templates**
  - [ ] ISSUE_TEMPLATE (bug report, feature request)
  - [ ] PULL_REQUEST_TEMPLATE
  - [ ] CODE_OF_CONDUCT.md

### SDK & Integration

- [ ] **Headless API** - Server-side usage without DOM
  - Node.js compatible WASM loading
  - No canvas dependency for data operations
  - Use case: Server-side formula calculation

- [x] **React Component Wrapper** - `<RuSheet />` component ✅ (2025-12-30)
  - `examples/react-basic.tsx` updated with latest API

- [ ] **Vue Component Wrapper** - `<RuSheet />` component
  - Similar API to React wrapper

- [ ] **REST API Client SDK** - TypeScript client for rusheet-server
  - Workbook CRUD operations
  - WebSocket connection helper
  - Type-safe API calls

### Package Publishing

- [ ] **Publish to npm** - `rusheet` package
- [ ] **Publish to crates.io** - Core crates

---

## Recent Accomplishments (Jan 2026)

- [x] **Default Grid Dimensions** ✅ (2026-01-05)
  - Updated default sheet size to **1000 rows** and **26 columns (A-Z)** to match Google Sheets defaults.
  - Aligned WASM API, Frontend Controller, and CSV import logic.

- [x] **Column Header Rendering Fix** ✅ (2026-01-05)
  - Fixed visual bug where columns beyond 'Z' (e.g., AA, AB) were rendered incorrectly.
  - Implemented proper base-26 column lettering logic in example renderer.

- [x] **Structured Error Handling** ✅ (2026-01-04)
  - Created `RuSheetError` enum in Rust core.
  - Implemented `JsRuSheetError` in WASM bridge.

- [x] **Documentation & Spec Sync** ✅ (2026-01-05)
  - Updated `docs/guide/getting-started.md` to use the modern React component API.
  - Updated `specs/architecture.md` to include the Collaboration Server (Axum/Yjs).

---

## Current Priority Queue

### P0: Critical (Blocks Production Use)

- [x] **Real-time Collaboration Server** ✅ (2025-12-30)
  - Axum + Yjs/yrs + PostgreSQL
  - WebSocket sync & Cursor tracking

- [x] **Event/Callback System** ✅ (2025-12-30)
- [x] **Row/Column Insert/Delete** ✅ (2025-12-30)
- [x] **CSV/XLSX Import/Export** ✅ (2025-12-30)

### P1: High Priority (Essential Features)

- [ ] **Authentication & Permissions**
  - **Goal**: Secure the collaboration server.
  - JWT Auth
  - Workbook ownership and sharing permissions (Public/Private/Shared).

- [ ] **Data Validation**
  - **Goal**: Ensure data integrity.
  - Dropdown lists (from range or list).
  - Number/Date constraints.

- [ ] **Conditional Formatting**
  - **Goal**: Visual data analysis.
  - Highlight cells based on value/formula.
  - Color scales.

### P2: Medium Priority (Feature Completeness)

#### Formula Engine
- [ ] **Array Formulas** (ARRAYFORMULA support)
- [ ] **Named Ranges**
- [ ] **Text Functions** (FIND, SEARCH, SUBSTITUTE)
- [ ] **Advanced Lookups** (INDEX, OFFSET)

#### Editing & UI
- [ ] **Find & Replace** (Cross-sheet, Regex)
- [ ] **Special Paste** (Values only, Transpose)
- [ ] **AutoFill** (Drag handle logic)
- [ ] **Comments System** (Cell-based comments)

### P3: Low Priority (Nice to Have)

- [ ] **Pivot Tables**
- [ ] **Charts** (Chart.js integration)
- [ ] **PDF Export**
- [ ] **Plugin System**

---

## Architecture Notes

### Performance Optimizations
- **Morton encoding**: O(1) cell lookup in 64x64 chunks
- **Sparse storage**: bitvec + Option<T> array
- **Zero-copy viewport**: Direct memory access from JS
- **Formula caching**: Lazy evaluation with dependency tracking

### Module Dependencies
```
rusheet-wasm (WASM bindings)
  ├── rusheet-core (cells, sheets, formatting)
  ├── rusheet-formula (parser, evaluator)
  └── rusheet-history (undo/redo commands)

rusheet-server (Collaboration backend)
  ├── rusheet-core
  ├── axum (HTTP/WebSocket)
  ├── yrs (CRDT)
  └── sqlx (PostgreSQL)

Frontend
  ├── rusheet-wasm (via pkg/)
  ├── yjs + y-websocket (collaboration)
  └── Canvas rendering
```\n## Documentation Debt (New)\n\n- [ ] **Server API Specification**: Detailed Swagger/OpenAPI spec for `rusheet-server` endpoints.\n- [ ] **Collaboration Protocol**: Sequence diagram explaining the Yjs sync flow (Client <-> Server <-> DB).\n- [ ] **Custom Function Guide**: How to add new functions to `rusheet-formula`.
