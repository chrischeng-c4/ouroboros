import type { CellData, CellFormat } from '../types';

// Dynamic import for WASM module
let wasmModule: typeof import('../../pkg/data_bridge_sheet_wasm') | null = null;
let engine: InstanceType<typeof import('../../pkg/data_bridge_sheet_wasm').SpreadsheetEngine> | null = null;
let wasmMemory: WebAssembly.Memory | null = null;

export async function initWasm(): Promise<void> {
  if (wasmModule) return;

  try {
    wasmModule = await import('../../pkg/data_bridge_sheet_wasm');
    await wasmModule.default();
    engine = new wasmModule.SpreadsheetEngine();
    // Get WASM memory for zero-copy access
    // @ts-expect-error - memory is exported but not in types
    wasmMemory = wasmModule.memory;
  } catch (error) {
    console.error('Failed to load WASM module:', error);
    throw error;
  }
}

function getEngine() {
  if (!engine) {
    throw new Error('WASM module not initialized. Call initWasm() first.');
  }
  return engine;
}

export function setCellValue(row: number, col: number, value: string): [number, number][] {
  const json = getEngine().setCellValue(row, col, value);
  return JSON.parse(json);
}

export function getCellData(row: number, col: number): CellData | null {
  const data = getEngine().getCellData(row, col);
  return data as CellData | null;
}

export function getViewportData(
  startRow: number,
  endRow: number,
  startCol: number,
  endCol: number
): CellData[] {
  const json = getEngine().getViewportData(startRow, endRow, startCol, endCol);
  return JSON.parse(json);
}

export function setCellFormat(row: number, col: number, format: CellFormat): boolean {
  return getEngine().setCellFormat(row, col, JSON.stringify(format));
}

export function setRangeFormat(
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number,
  format: CellFormat
): boolean {
  return getEngine().setRangeFormat(
    startRow,
    startCol,
    endRow,
    endCol,
    JSON.stringify(format)
  );
}

export function clearRange(
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number
): [number, number][] {
  const json = getEngine().clearRange(startRow, startCol, endRow, endCol);
  return JSON.parse(json);
}

export function undo(): [number, number][] {
  const json = getEngine().undo();
  return JSON.parse(json);
}

export function redo(): [number, number][] {
  const json = getEngine().redo();
  return JSON.parse(json);
}

export function canUndo(): boolean {
  return getEngine().canUndo();
}

export function canRedo(): boolean {
  return getEngine().canRedo();
}

export function addSheet(name: string): number {
  return getEngine().addSheet(name);
}

export function setActiveSheet(index: number): boolean {
  return getEngine().setActiveSheet(index);
}

export function getSheetNames(): string[] {
  const json = getEngine().getSheetNames();
  return JSON.parse(json);
}

export function getActiveSheetIndex(): number {
  return getEngine().getActiveSheetIndex();
}

export function renameSheet(index: number, name: string): boolean {
  return getEngine().renameSheet(index, name);
}

export function deleteSheet(index: number): boolean {
  return getEngine().deleteSheet(index);
}

export function setRowHeight(row: number, height: number): void {
  getEngine().setRowHeight(row, height);
}

export function setColWidth(col: number, width: number): void {
  getEngine().setColWidth(col, width);
}

export function getRowHeight(row: number): number {
  return getEngine().getRowHeight(row);
}

export function getColWidth(col: number): number {
  return getEngine().getColWidth(col);
}

// Row/Column Insert/Delete
export function insertRows(atRow: number, count: number): [number, number][] {
  const json = getEngine().insertRows(atRow, count);
  return JSON.parse(json);
}

export function deleteRows(atRow: number, count: number): [number, number][] {
  const json = getEngine().deleteRows(atRow, count);
  return JSON.parse(json);
}

export function insertCols(atCol: number, count: number): [number, number][] {
  const json = getEngine().insertCols(atCol, count);
  return JSON.parse(json);
}

export function deleteCols(atCol: number, count: number): [number, number][] {
  const json = getEngine().deleteCols(atCol, count);
  return JSON.parse(json);
}

// Sorting
export function sortRange(
  startRow: number,
  endRow: number,
  startCol: number,
  endCol: number,
  sortCol: number,
  ascending: boolean
): [number, number][] {
  const json = getEngine().sortRange(startRow, endRow, startCol, endCol, sortCol, ascending);
  return JSON.parse(json);
}

export function serialize(): string {
  return getEngine().serialize();
}

export function deserialize(json: string): boolean {
  return getEngine().deserialize(json);
}

// =============================================================================
// Cell Merging
// =============================================================================

export interface MergeInfo {
  masterRow: number;
  masterCol: number;
  rowSpan: number;
  colSpan: number;
}

export interface MergeRange {
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
}

export function mergeCells(
  startRow: number,
  startCol: number,
  endRow: number,
  endCol: number
): [number, number][] {
  const json = getEngine().mergeCells(startRow, startCol, endRow, endCol);
  return JSON.parse(json);
}

export function unmergeCells(row: number, col: number): [number, number][] {
  const json = getEngine().unmergeCells(row, col);
  return JSON.parse(json);
}

export function getMergedRanges(): MergeRange[] {
  const json = getEngine().getMergedRanges();
  return JSON.parse(json);
}

export function isMergedSlave(row: number, col: number): boolean {
  return getEngine().isMergedSlave(row, col);
}

export function getMergeInfo(row: number, col: number): MergeInfo | null {
  return getEngine().getMergeInfo(row, col) as MergeInfo | null;
}

export function recalculateAll(): void {
  getEngine().recalculateAll();
}

// =============================================================================
// Filter Functions
// =============================================================================

export function getUniqueValuesInColumn(col: number, maxRows: number = 10000): string[] {
  const json = getEngine().getUniqueValuesInColumn(col, maxRows);
  return JSON.parse(json);
}

export function applyColumnFilter(col: number, visibleValues: string[], maxRows: number = 10000): [number, number][] {
  const json = getEngine().applyColumnFilter(col, JSON.stringify(visibleValues), maxRows);
  return JSON.parse(json);
}

export function clearColumnFilter(col: number): [number, number][] {
  const json = getEngine().clearColumnFilter(col);
  return JSON.parse(json);
}

export function clearAllFilters(): [number, number][] {
  const json = getEngine().clearAllFilters();
  return JSON.parse(json);
}

export interface FilterState {
  col: number;
  visibleValues: string[];
}

export function getActiveFilters(): FilterState[] {
  const json = getEngine().getActiveFilters();
  return JSON.parse(json);
}

export function isRowHidden(row: number): boolean {
  return getEngine().isRowHidden(row);
}

export function getHiddenRows(): number[] {
  const json = getEngine().getHiddenRows();
  return JSON.parse(json);
}

// =============================================================================
// Zero-Copy Viewport API
// =============================================================================

/**
 * Viewport data from zero-copy buffer.
 * Provides direct access to WASM memory through TypedArrays.
 */
export interface ViewportArrays {
  /** Row indices for each cell */
  rows: Uint32Array;
  /** Column indices for each cell */
  cols: Uint32Array;
  /** Numeric values (NaN for non-numeric) */
  values: Float64Array;
  /** Packed format flags */
  formats: Uint32Array;
  /** Display strings (still JSON) */
  displayValues: string[];
  /** Number of cells */
  length: number;
}

/**
 * Populate the viewport buffer and get zero-copy access to the data.
 * This is much faster than getViewportData() for large viewports.
 *
 * @param startRow - Starting row (inclusive)
 * @param endRow - Ending row (inclusive)
 * @param startCol - Starting column (inclusive)
 * @param endCol - Ending column (inclusive)
 * @returns ViewportArrays with direct memory access
 */
export function getViewportArrays(
  startRow: number,
  endRow: number,
  startCol: number,
  endCol: number
): ViewportArrays {
  const eng = getEngine();

  // Populate the buffer
  eng.populateViewport(startRow, endRow, startCol, endCol);

  const len = eng.getViewportLen();

  if (len === 0 || !wasmMemory) {
    return {
      rows: new Uint32Array(0),
      cols: new Uint32Array(0),
      values: new Float64Array(0),
      formats: new Uint32Array(0),
      displayValues: [],
      length: 0,
    };
  }

  // Get pointers
  const rowsPtr = eng.getViewportRowsPtr();
  const colsPtr = eng.getViewportColsPtr();
  const valuesPtr = eng.getViewportValuesPtr();
  const formatsPtr = eng.getViewportFormatsPtr();

  // Create views into WASM memory (zero-copy!)
  const rows = new Uint32Array(wasmMemory.buffer, rowsPtr, len);
  const cols = new Uint32Array(wasmMemory.buffer, colsPtr, len);
  const values = new Float64Array(wasmMemory.buffer, valuesPtr, len);
  const formats = new Uint32Array(wasmMemory.buffer, formatsPtr, len);

  // Display values still need JSON parsing
  const displayValues: string[] = JSON.parse(eng.getViewportDisplayValues());

  return {
    rows,
    cols,
    values,
    formats,
    displayValues,
    length: len,
  };
}

/**
 * Unpack format flags into individual properties.
 *
 * @param flags - Packed format flags from formats array
 * @returns Object with individual format properties
 */
export function unpackFormatFlags(flags: number): {
  bold: boolean;
  italic: boolean;
  underline: boolean;
  fontSize: number;
  horizontalAlign: 'left' | 'center' | 'right';
  verticalAlign: 'middle' | 'top' | 'bottom';
} {
  const bold = (flags & (1 << 0)) !== 0;
  const italic = (flags & (1 << 1)) !== 0;
  const underline = (flags & (1 << 2)) !== 0;
  const fontSize = (flags >> 8) & 0xff;
  const hAlign = (flags >> 16) & 0x7;
  const vAlign = (flags >> 19) & 0x7;

  const horizontalAlignMap = ['left', 'center', 'right'] as const;
  const verticalAlignMap = ['middle', 'top', 'bottom'] as const;

  return {
    bold,
    italic,
    underline,
    fontSize,
    horizontalAlign: horizontalAlignMap[hAlign] || 'left',
    verticalAlign: verticalAlignMap[vAlign] || 'middle',
  };
}
