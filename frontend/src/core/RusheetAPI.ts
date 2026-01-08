import Papa from 'papaparse';
import * as XLSX from 'xlsx';
import { emitter } from './EventEmitter';
import * as WasmBridge from './WasmBridge';
import type { CellData, CellFormat } from '../types';
import type {
  FormatChangeEvent,
  SheetAddEvent,
  SheetDeleteEvent,
  SheetRenameEvent,
  ActiveSheetChangeEvent,
  UndoEvent,
  RedoEvent,
  EventSource,
  RowsInsertEvent,
  RowsDeleteEvent,
  ColsInsertEvent,
  ColsDeleteEvent,
  SortRangeEvent,
  MergeCellsEvent,
  UnmergeCellsEvent,
  FilterChangeEvent
} from '../types/events';

export interface CellChangeEvent {
  row: number;
  col: number;
  oldValue: string | null;
  newValue: string | null;
  source: 'user' | 'api' | 'undo' | 'redo';
}

export interface SelectionChangeEvent {
  row: number;
  col: number;
  previousRow: number;
  previousCol: number;
}

export interface CellEditEvent {
  row: number;
  col: number;
  value: string;
  phase: 'start' | 'change' | 'end' | 'cancel';
}

export class RusheetAPI {
  private static instance: RusheetAPI;
  private initialized = false;
  private currentSelection = { row: 0, col: 0 };
  private currentSheetIndex = 0;
  private sheetNames: string[] = ['Sheet1'];

  private constructor() {}

  static getInstance(): RusheetAPI {
    if (!RusheetAPI.instance) {
      RusheetAPI.instance = new RusheetAPI();
    }
    return RusheetAPI.instance;
  }

  // Initialization
  async init(): Promise<void> {
    if (this.initialized) return;
    await WasmBridge.initWasm();
    this.initialized = true;
  }

  // Event subscription
  onChange(callback: (event: CellChangeEvent) => void): () => void {
    return emitter.on('change', callback);
  }

  onSelectionChange(callback: (event: SelectionChangeEvent) => void): () => void {
    return emitter.on('selectionChange', callback);
  }

  onCellEdit(callback: (event: CellEditEvent) => void): () => void {
    return emitter.on('cellEdit', callback);
  }

  onFormatChange(callback: (event: FormatChangeEvent) => void): () => void {
    return emitter.on('formatChange', callback);
  }

  onSheetAdd(callback: (event: SheetAddEvent) => void): () => void {
    return emitter.on('sheetAdd', callback);
  }

  onSheetDelete(callback: (event: SheetDeleteEvent) => void): () => void {
    return emitter.on('sheetDelete', callback);
  }

  onSheetRename(callback: (event: SheetRenameEvent) => void): () => void {
    return emitter.on('sheetRename', callback);
  }

  onActiveSheetChange(callback: (event: ActiveSheetChangeEvent) => void): () => void {
    return emitter.on('activeSheetChange', callback);
  }

  onUndo(callback: (event: UndoEvent) => void): () => void {
    return emitter.on('undo', callback);
  }

  onRedo(callback: (event: RedoEvent) => void): () => void {
    return emitter.on('redo', callback);
  }

  onRowsInsert(callback: (event: RowsInsertEvent) => void): () => void {
    return emitter.on('rowsInsert', callback);
  }

  onRowsDelete(callback: (event: RowsDeleteEvent) => void): () => void {
    return emitter.on('rowsDelete', callback);
  }

  onColsInsert(callback: (event: ColsInsertEvent) => void): () => void {
    return emitter.on('colsInsert', callback);
  }

  onColsDelete(callback: (event: ColsDeleteEvent) => void): () => void {
    return emitter.on('colsDelete', callback);
  }

  onSortRange(callback: (event: SortRangeEvent) => void): () => void {
    return emitter.on('sortRange', callback);
  }

  onMergeCells(callback: (event: MergeCellsEvent) => void): () => void {
    return emitter.on('mergeCells', callback);
  }

  onUnmergeCells(callback: (event: UnmergeCellsEvent) => void): () => void {
    return emitter.on('unmergeCells', callback);
  }

  onFilterChange(callback: (event: FilterChangeEvent) => void): () => void {
    return emitter.on('filterChange', callback);
  }

  // Cell operations (wrap WasmBridge and emit events)
  setCellValue(row: number, col: number, value: string, source: 'user' | 'api' = 'api'): [number, number][] {
    const oldData = WasmBridge.getCellData(row, col);
    const oldValue = oldData?.value ?? null;
    const affected = WasmBridge.setCellValue(row, col, value);

    emitter.emit<CellChangeEvent>('change', {
      row, col, oldValue, newValue: value, source
    });

    return affected;
  }

  getCellData(row: number, col: number): CellData | null {
    return WasmBridge.getCellData(row, col);
  }

  setCellFormat(row: number, col: number, format: CellFormat, source: EventSource = 'api'): boolean {
    const success = WasmBridge.setCellFormat(row, col, format);
    if (success) {
      emitter.emit<FormatChangeEvent>('formatChange', {
        type: 'cell',
        startRow: row, startCol: col,
        endRow: row, endCol: col,
        format, source
      });
    }
    return success;
  }

  setRangeFormat(startRow: number, startCol: number, endRow: number, endCol: number, format: CellFormat, source: EventSource = 'api'): boolean {
    const success = WasmBridge.setRangeFormat(startRow, startCol, endRow, endCol, format);
    if (success) {
      emitter.emit<FormatChangeEvent>('formatChange', {
        type: 'range',
        startRow, startCol, endRow, endCol,
        format, source
      });
    }
    return success;
  }

  clearRange(startRow: number, startCol: number, endRow: number, endCol: number): [number, number][] {
    return WasmBridge.clearRange(startRow, startCol, endRow, endCol);
  }

  // Selection (emit events)
  setSelection(row: number, col: number): void {
    const previous = { ...this.currentSelection };
    this.currentSelection = { row, col };

    emitter.emit<SelectionChangeEvent>('selectionChange', {
      row, col,
      previousRow: previous.row,
      previousCol: previous.col
    });
  }

  getSelection(): { row: number; col: number } {
    return { ...this.currentSelection };
  }

  // Emit cell edit events (called by CellEditor)
  emitCellEdit(row: number, col: number, value: string, phase: CellEditEvent['phase']): void {
    emitter.emit<CellEditEvent>('cellEdit', { row, col, value, phase });
  }

  // Batch data loading
  setData(data: (string | number | null)[][]): void {
    for (let row = 0; row < data.length; row++) {
      for (let col = 0; col < data[row].length; col++) {
        const value = data[row][col];
        if (value !== null && value !== undefined && value !== '') {
          WasmBridge.setCellValue(row, col, String(value));
        }
      }
    }
    emitter.emit('dataLoaded', { rows: data.length });
  }

  // Get all data as 2D array
  getData(startRow = 0, endRow = 999, startCol = 0, endCol = 25): (string | null)[][] {
    const result: (string | null)[][] = [];
    for (let row = startRow; row <= endRow; row++) {
      const rowData: (string | null)[] = [];
      for (let col = startCol; col <= endCol; col++) {
        const cell = WasmBridge.getCellData(row, col);
        rowData.push(cell?.value ?? null);
      }
      result.push(rowData);
    }
    return result;
  }

  // Undo/Redo (emit events)
  undo(): [number, number][] {
    const affected = WasmBridge.undo();

    // Emit undo event
    emitter.emit<UndoEvent>('undo', { affectedCells: affected });

    // Also emit change events for each cell
    affected.forEach(([row, col]) => {
      const newData = WasmBridge.getCellData(row, col);
      emitter.emit<CellChangeEvent>('change', {
        row, col,
        oldValue: null,
        newValue: newData?.value ?? null,
        source: 'undo'
      });
    });
    return affected;
  }

  redo(): [number, number][] {
    const affected = WasmBridge.redo();

    // Emit redo event
    emitter.emit<RedoEvent>('redo', { affectedCells: affected });

    // Also emit change events for each cell
    affected.forEach(([row, col]) => {
      const newData = WasmBridge.getCellData(row, col);
      emitter.emit<CellChangeEvent>('change', {
        row, col,
        oldValue: null,
        newValue: newData?.value ?? null,
        source: 'redo'
      });
    });
    return affected;
  }

  canUndo(): boolean { return WasmBridge.canUndo(); }
  canRedo(): boolean { return WasmBridge.canRedo(); }

  // Serialization
  serialize(): string { return WasmBridge.serialize(); }
  deserialize(json: string): boolean { return WasmBridge.deserialize(json); }

  // CSV Import/Export
  /**
   * Export spreadsheet data as CSV string
   * @param options - CSV export options
   * @returns CSV string
   */
  exportCSV(options: {
    delimiter?: string;
    startRow?: number;
    endRow?: number;
    startCol?: number;
    endCol?: number;
    includeEmptyRows?: boolean;
  } = {}): string {
    const {
      delimiter = ',',
      startRow = 0,
      endRow = 999,
      startCol = 0,
      endCol = 25,
      includeEmptyRows = false
    } = options;

    const data = this.getData(startRow, endRow, startCol, endCol);

    // Trim trailing empty rows
    let lastNonEmptyRow = data.length - 1;
    if (!includeEmptyRows) {
      while (lastNonEmptyRow >= 0 && data[lastNonEmptyRow].every(cell => cell === null || cell === '')) {
        lastNonEmptyRow--;
      }
    }

    // Trim trailing empty columns
    let maxCol = 0;
    for (let row = 0; row <= lastNonEmptyRow; row++) {
      for (let col = data[row].length - 1; col >= 0; col--) {
        if (data[row][col] !== null && data[row][col] !== '') {
          maxCol = Math.max(maxCol, col);
          break;
        }
      }
    }

    const trimmedData = data
      .slice(0, lastNonEmptyRow + 1)
      .map(row => row.slice(0, maxCol + 1).map(cell => cell ?? ''));

    return Papa.unparse(trimmedData, { delimiter });
  }

  /**
   * Import CSV data into spreadsheet
   * @param csvString - CSV string to import
   * @param options - CSV import options
   * @returns Number of rows imported
   */
  importCSV(csvString: string, options: {
    delimiter?: string;
    startRow?: number;
    startCol?: number;
    clearExisting?: boolean;
  } = {}): { rows: number; cols: number } {
    const {
      delimiter = ',',
      startRow = 0,
      startCol = 0,
      clearExisting = true
    } = options;

    const result = Papa.parse<string[]>(csvString, {
      delimiter,
      skipEmptyLines: false
    });

    if (result.errors.length > 0) {
      console.warn('CSV parse warnings:', result.errors);
    }

    const data = result.data;

    // Clear existing data if requested
    if (clearExisting) {
      const maxRows = Math.max(data.length, 1000);
      const maxCols = Math.max(data[0]?.length ?? 0, 26);
      WasmBridge.clearRange(0, 0, maxRows, maxCols);
    }

    // Import data
    let maxColCount = 0;
    for (let row = 0; row < data.length; row++) {
      const rowData = data[row];
      maxColCount = Math.max(maxColCount, rowData.length);
      for (let col = 0; col < rowData.length; col++) {
        const value = rowData[col];
        if (value !== null && value !== undefined && value !== '') {
          WasmBridge.setCellValue(startRow + row, startCol + col, value);
        }
      }
    }

    emitter.emit('dataLoaded', { rows: data.length, cols: maxColCount, source: 'csv' });

    return { rows: data.length, cols: maxColCount };
  }

  /**
   * Download spreadsheet as CSV file
   * @param filename - Name of the file (default: 'spreadsheet.csv')
   * @param options - CSV export options
   */
  downloadCSV(filename = 'spreadsheet.csv', options: Parameters<typeof this.exportCSV>[0] = {}): void {
    const csv = this.exportCSV(options);
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);

    const link = document.createElement('a');
    link.href = url;
    link.download = filename.endsWith('.csv') ? filename : `${filename}.csv`;
    link.style.display = 'none';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }

  /**
   * Import CSV from File object
   * @param file - File object from input or drag-drop
   * @param options - CSV import options
   * @returns Promise resolving to import stats
   */
  async importCSVFile(file: File, options: Parameters<typeof this.importCSV>[1] = {}): Promise<{ rows: number; cols: number }> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = (e) => {
        const csvString = e.target?.result as string;
        if (csvString) {
          const stats = this.importCSV(csvString, options);
          resolve(stats);
        } else {
          reject(new Error('Failed to read file'));
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsText(file);
    });
  }

  // XLSX Import/Export
  /**
   * Export spreadsheet data as XLSX ArrayBuffer
   * @param options - XLSX export options
   * @returns ArrayBuffer containing XLSX file data
   */
  exportXLSX(options: {
    sheetName?: string;
    startRow?: number;
    endRow?: number;
    startCol?: number;
    endCol?: number;
  } = {}): ArrayBuffer {
    const {
      sheetName = 'Sheet1',
      startRow = 0,
      endRow = 999,
      startCol = 0,
      endCol = 25,
    } = options;

    const data = this.getData(startRow, endRow, startCol, endCol);

    // Trim trailing empty rows
    let lastNonEmptyRow = data.length - 1;
    while (lastNonEmptyRow >= 0 && data[lastNonEmptyRow].every(cell => cell === null || cell === '')) {
      lastNonEmptyRow--;
    }

    // Trim trailing empty columns
    let maxCol = 0;
    for (let row = 0; row <= lastNonEmptyRow; row++) {
      for (let col = data[row].length - 1; col >= 0; col--) {
        if (data[row][col] !== null && data[row][col] !== '') {
          maxCol = Math.max(maxCol, col);
          break;
        }
      }
    }

    const trimmedData = data
      .slice(0, lastNonEmptyRow + 1)
      .map(row => row.slice(0, maxCol + 1).map(cell => cell ?? ''));

    // Create workbook
    const ws = XLSX.utils.aoa_to_sheet(trimmedData);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws, sheetName);

    // Write to buffer
    const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });
    return buffer;
  }

  /**
   * Import XLSX data from ArrayBuffer
   * @param buffer - ArrayBuffer containing XLSX file data
   * @param options - XLSX import options
   * @returns Import statistics
   */
  importXLSX(buffer: ArrayBuffer, options: {
    sheetIndex?: number;
    sheetName?: string;
    startRow?: number;
    startCol?: number;
    clearExisting?: boolean;
  } = {}): { rows: number; cols: number; sheetName: string } {
    const {
      sheetIndex = 0,
      sheetName,
      startRow = 0,
      startCol = 0,
      clearExisting = true
    } = options;

    // Parse workbook
    const wb = XLSX.read(buffer, { type: 'array' });

    // Get sheet
    const targetSheetName = sheetName || wb.SheetNames[sheetIndex];
    if (!targetSheetName || !wb.Sheets[targetSheetName]) {
      throw new Error(`Sheet not found: ${sheetName || `index ${sheetIndex}`}`);
    }

    const ws = wb.Sheets[targetSheetName];

    // Convert to 2D array
    const data: (string | number | null)[][] = XLSX.utils.sheet_to_json(ws, {
      header: 1,
      defval: null,
      raw: false // Convert all to strings for consistency
    });

    // Clear existing data if requested
    if (clearExisting) {
      const maxRows = Math.max(data.length, 1000);
      const maxCols = Math.max(data[0]?.length ?? 0, 26);
      WasmBridge.clearRange(0, 0, maxRows, maxCols);
    }

    // Import data
    let maxColCount = 0;
    for (let row = 0; row < data.length; row++) {
      const rowData = data[row] || [];
      maxColCount = Math.max(maxColCount, rowData.length);
      for (let col = 0; col < rowData.length; col++) {
        const value = rowData[col];
        if (value !== null && value !== undefined && value !== '') {
          WasmBridge.setCellValue(startRow + row, startCol + col, String(value));
        }
      }
    }

    emitter.emit('dataLoaded', { rows: data.length, cols: maxColCount, source: 'xlsx', sheetName: targetSheetName });

    return { rows: data.length, cols: maxColCount, sheetName: targetSheetName };
  }

  /**
   * Download spreadsheet as XLSX file
   * @param filename - Name of the file (default: 'spreadsheet.xlsx')
   * @param options - XLSX export options
   */
  downloadXLSX(filename = 'spreadsheet.xlsx', options: Parameters<typeof this.exportXLSX>[0] = {}): void {
    const buffer = this.exportXLSX(options);
    const blob = new Blob([buffer], { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' });
    const url = URL.createObjectURL(blob);

    const link = document.createElement('a');
    link.href = url;
    link.download = filename.endsWith('.xlsx') ? filename : `${filename}.xlsx`;
    link.style.display = 'none';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }

  /**
   * Import XLSX from File object
   * @param file - File object from input or drag-drop
   * @param options - XLSX import options
   * @returns Promise resolving to import stats
   */
  async importXLSXFile(file: File, options: Parameters<typeof this.importXLSX>[1] = {}): Promise<{ rows: number; cols: number; sheetName: string }> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = (e) => {
        const buffer = e.target?.result as ArrayBuffer;
        if (buffer) {
          try {
            const stats = this.importXLSX(buffer, options);
            resolve(stats);
          } catch (error) {
            reject(error);
          }
        } else {
          reject(new Error('Failed to read file'));
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsArrayBuffer(file);
    });
  }

  /**
   * Get sheet names from an XLSX file
   * @param buffer - ArrayBuffer containing XLSX file data
   * @returns Array of sheet names
   */
  getXLSXSheetNames(buffer: ArrayBuffer): string[] {
    const wb = XLSX.read(buffer, { type: 'array' });
    return wb.SheetNames;
  }

  // Sheet management (pass through)
  addSheet(name: string, source: EventSource = 'api'): number {
    const index = WasmBridge.addSheet(name);
    this.sheetNames = WasmBridge.getSheetNames();
    emitter.emit<SheetAddEvent>('sheetAdd', { index, name, source });
    return index;
  }

  setActiveSheet(index: number, source: EventSource = 'api'): boolean {
    const previousIndex = this.currentSheetIndex;
    const previousName = this.sheetNames[previousIndex] ?? 'Sheet1';
    const success = WasmBridge.setActiveSheet(index);
    if (success) {
      this.currentSheetIndex = index;
      this.sheetNames = WasmBridge.getSheetNames();
      const newName = this.sheetNames[index] ?? 'Sheet1';
      emitter.emit<ActiveSheetChangeEvent>('activeSheetChange', {
        previousIndex, newIndex: index, previousName, newName, source
      });
    }
    return success;
  }

  getSheetNames(): string[] { return WasmBridge.getSheetNames(); }
  getActiveSheetIndex(): number { return WasmBridge.getActiveSheetIndex(); }

  renameSheet(index: number, newName: string, source: EventSource = 'api'): boolean {
    const oldName = this.sheetNames[index] ?? '';
    try {
      const success = WasmBridge.renameSheet(index, newName);
      if (success) {
        this.sheetNames = WasmBridge.getSheetNames();
        emitter.emit<SheetRenameEvent>('sheetRename', { index, oldName, newName, source });
      }
      return success;
    } catch (e: any) {
      console.warn(`Rename sheet failed: ${e.message || e}`);
      return false;
    }
  }

  deleteSheet(index: number, source: EventSource = 'api'): boolean {
    const name = this.sheetNames[index] ?? '';
    try {
      const success = WasmBridge.deleteSheet(index);
      if (success) {
        this.sheetNames = WasmBridge.getSheetNames();
        emitter.emit<SheetDeleteEvent>('sheetDelete', { index, name, source });
      }
      return success;
    } catch (e: any) {
      console.warn(`Delete sheet failed: ${e.message || e}`);
      return false;
    }
  }

  // Row/Col sizing (pass through)
  setRowHeight(row: number, height: number): void { WasmBridge.setRowHeight(row, height); }
  setColWidth(col: number, width: number): void { WasmBridge.setColWidth(col, width); }
  getRowHeight(row: number): number { return WasmBridge.getRowHeight(row); }
  getColWidth(col: number): number { return WasmBridge.getColWidth(col); }

  // Row/Column Insert/Delete
  insertRows(atRow: number, count: number, source: EventSource = 'api'): [number, number][] {
    const affected = WasmBridge.insertRows(atRow, count);
    emitter.emit<RowsInsertEvent>('rowsInsert', { atRow, count, affectedCells: affected, source });
    return affected;
  }

  deleteRows(atRow: number, count: number, source: EventSource = 'api'): [number, number][] {
    const affected = WasmBridge.deleteRows(atRow, count);
    emitter.emit<RowsDeleteEvent>('rowsDelete', { atRow, count, affectedCells: affected, source });
    return affected;
  }

  insertCols(atCol: number, count: number, source: EventSource = 'api'): [number, number][] {
    const affected = WasmBridge.insertCols(atCol, count);
    emitter.emit<ColsInsertEvent>('colsInsert', { atCol, count, affectedCells: affected, source });
    return affected;
  }

  deleteCols(atCol: number, count: number, source: EventSource = 'api'): [number, number][] {
    const affected = WasmBridge.deleteCols(atCol, count);
    emitter.emit<ColsDeleteEvent>('colsDelete', { atCol, count, affectedCells: affected, source });
    return affected;
  }

  // Sorting
  sortRange(
    startRow: number,
    endRow: number,
    startCol: number,
    endCol: number,
    sortCol: number,
    ascending: boolean,
    source: EventSource = 'api'
  ): [number, number][] {
    const affected = WasmBridge.sortRange(startRow, endRow, startCol, endCol, sortCol, ascending);
    emitter.emit<SortRangeEvent>('sortRange', {
      startRow, endRow, startCol, endCol, sortCol, ascending,
      affectedCells: affected, source
    });
    return affected;
  }

  // Cell Merging
  mergeCells(
    startRow: number,
    startCol: number,
    endRow: number,
    endCol: number,
    source: EventSource = 'api'
  ): [number, number][] {
    const affected = WasmBridge.mergeCells(startRow, startCol, endRow, endCol);
    if (affected.length > 0) {
      emitter.emit<MergeCellsEvent>('mergeCells', {
        startRow, startCol, endRow, endCol,
        affectedCells: affected, source
      });
    }
    return affected;
  }

  unmergeCells(row: number, col: number, source: EventSource = 'api'): [number, number][] {
    const affected = WasmBridge.unmergeCells(row, col);
    if (affected.length > 0) {
      emitter.emit<UnmergeCellsEvent>('unmergeCells', {
        row, col, affectedCells: affected, source
      });
    }
    return affected;
  }

  getMergedRanges(): WasmBridge.MergeRange[] {
    return WasmBridge.getMergedRanges();
  }

  isMergedSlave(row: number, col: number): boolean {
    return WasmBridge.isMergedSlave(row, col);
  }

  getMergeInfo(row: number, col: number): WasmBridge.MergeInfo | null {
    return WasmBridge.getMergeInfo(row, col);
  }

  // Viewport (pass through)
  getViewportData(startRow: number, endRow: number, startCol: number, endCol: number) {
    return WasmBridge.getViewportData(startRow, endRow, startCol, endCol);
  }
  getViewportArrays(startRow: number, endRow: number, startCol: number, endCol: number) {
    return WasmBridge.getViewportArrays(startRow, endRow, startCol, endCol);
  }

  // Filtering
  /**
   * Get unique values in a column (for filter dropdown)
   */
  getUniqueValuesInColumn(col: number, maxRows: number = 10000): string[] {
    return WasmBridge.getUniqueValuesInColumn(col, maxRows);
  }

  /**
   * Apply a column filter - show only rows with values in the visible set
   */
  applyColumnFilter(col: number, visibleValues: string[], maxRows: number = 10000): void {
    const affected = WasmBridge.applyColumnFilter(col, visibleValues, maxRows);
    emitter.emit<FilterChangeEvent>('filterChange', { col, visibleValues, affected });
    emitter.emit('change', { type: 'filter', col });
  }

  /**
   * Clear filter on a specific column
   */
  clearColumnFilter(col: number): void {
    const affected = WasmBridge.clearColumnFilter(col);
    emitter.emit<FilterChangeEvent>('filterChange', { col, cleared: true, affected });
    emitter.emit('change', { type: 'filter', col });
  }

  /**
   * Clear all filters
   */
  clearAllFilters(): void {
    const affected = WasmBridge.clearAllFilters();
    emitter.emit<FilterChangeEvent>('filterChange', { cleared: true, all: true, affected });
    emitter.emit('change', { type: 'filter' });
  }

  /**
   * Get active filters
   */
  getActiveFilters(): WasmBridge.FilterState[] {
    return WasmBridge.getActiveFilters();
  }

  /**
   * Check if a row is hidden by filters
   */
  isRowHidden(row: number): boolean {
    return WasmBridge.isRowHidden(row);
  }

  /**
   * Get all hidden rows
   */
  getHiddenRows(): number[] {
    return WasmBridge.getHiddenRows();
  }

  // Cleanup
  destroy(): void {
    emitter.removeAllListeners();
    this.initialized = false;
  }
}

// Export singleton
export const rusheet = RusheetAPI.getInstance();
