import { useRef, useCallback } from 'react';
import type { RuSheetRef, CSVExportOptions, CSVImportOptions, XLSXExportOptions, XLSXImportOptions } from './RuSheet';

/**
 * Custom hook for easier RuSheet ref management
 *
 * @example
 * ```tsx
 * function App() {
 *   const { ref, api } = useRuSheet();
 *
 *   const handleAddRow = () => {
 *     api.insertRows(0, 1);
 *   };
 *
 *   return (
 *     <>
 *       <button onClick={handleAddRow}>Add Row</button>
 *       <RuSheet ref={ref} />
 *     </>
 *   );
 * }
 * ```
 */
export function useRuSheet() {
  const ref = useRef<RuSheetRef>(null);

  const api = {
    getCellData: useCallback(
      (row: number, col: number) => ref.current?.getCellData(row, col) ?? null,
      []
    ),
    setCellValue: useCallback(
      (row: number, col: number, value: string) =>
        ref.current?.setCellValue(row, col, value),
      []
    ),
    setCellFormat: useCallback(
      (row: number, col: number, format: Parameters<RuSheetRef['setCellFormat']>[2]) =>
        ref.current?.setCellFormat(row, col, format),
      []
    ),
    setRangeFormat: useCallback(
      (
        startRow: number,
        startCol: number,
        endRow: number,
        endCol: number,
        format: Parameters<RuSheetRef['setRangeFormat']>[4]
      ) => ref.current?.setRangeFormat(startRow, startCol, endRow, endCol, format),
      []
    ),
    clearRange: useCallback(
      (startRow: number, startCol: number, endRow: number, endCol: number) =>
        ref.current?.clearRange(startRow, startCol, endRow, endCol),
      []
    ),
    insertRows: useCallback(
      (atRow: number, count: number) => ref.current?.insertRows(atRow, count),
      []
    ),
    deleteRows: useCallback(
      (atRow: number, count: number) => ref.current?.deleteRows(atRow, count),
      []
    ),
    insertCols: useCallback(
      (atCol: number, count: number) => ref.current?.insertCols(atCol, count),
      []
    ),
    deleteCols: useCallback(
      (atCol: number, count: number) => ref.current?.deleteCols(atCol, count),
      []
    ),
    addSheet: useCallback(
      (name: string) => ref.current?.addSheet(name) ?? -1,
      []
    ),
    deleteSheet: useCallback(
      (index: number) => ref.current?.deleteSheet(index) ?? false,
      []
    ),
    getSheetNames: useCallback(
      () => ref.current?.getSheetNames() ?? [],
      []
    ),
    setActiveSheet: useCallback(
      (index: number) => ref.current?.setActiveSheet(index) ?? false,
      []
    ),
    undo: useCallback(() => ref.current?.undo(), []),
    redo: useCallback(() => ref.current?.redo(), []),
    canUndo: useCallback(() => ref.current?.canUndo() ?? false, []),
    canRedo: useCallback(() => ref.current?.canRedo() ?? false, []),
    serialize: useCallback(() => ref.current?.serialize() ?? '{}', []),
    deserialize: useCallback(
      (json: string) => ref.current?.deserialize(json) ?? false,
      []
    ),
    getData: useCallback(
      (startRow = 0, endRow = 999, startCol = 0, endCol = 25) =>
        ref.current?.getData(startRow, endRow, startCol, endCol) ?? [],
      []
    ),
    setData: useCallback(
      (data: (string | number | null)[][]) => ref.current?.setData(data),
      []
    ),
    render: useCallback(() => ref.current?.render(), []),
    // CSV Import/Export
    exportCSV: useCallback(
      (options?: CSVExportOptions) => ref.current?.exportCSV(options) ?? '',
      []
    ),
    importCSV: useCallback(
      (csvString: string, options?: CSVImportOptions) =>
        ref.current?.importCSV(csvString, options) ?? { rows: 0, cols: 0 },
      []
    ),
    downloadCSV: useCallback(
      (filename?: string, options?: CSVExportOptions) =>
        ref.current?.downloadCSV(filename, options),
      []
    ),
    importCSVFile: useCallback(
      async (file: File, options?: CSVImportOptions) =>
        ref.current?.importCSVFile(file, options) ?? { rows: 0, cols: 0 },
      []
    ),
    // XLSX Import/Export
    exportXLSX: useCallback(
      (options?: XLSXExportOptions) => ref.current?.exportXLSX(options) ?? new ArrayBuffer(0),
      []
    ),
    importXLSX: useCallback(
      (buffer: ArrayBuffer, options?: XLSXImportOptions) =>
        ref.current?.importXLSX(buffer, options) ?? { rows: 0, cols: 0, sheetName: '' },
      []
    ),
    downloadXLSX: useCallback(
      (filename?: string, options?: XLSXExportOptions) =>
        ref.current?.downloadXLSX(filename, options),
      []
    ),
    importXLSXFile: useCallback(
      async (file: File, options?: XLSXImportOptions) =>
        ref.current?.importXLSXFile(file, options) ?? { rows: 0, cols: 0, sheetName: '' },
      []
    ),
    getXLSXSheetNames: useCallback(
      (buffer: ArrayBuffer) => ref.current?.getXLSXSheetNames(buffer) ?? [],
      []
    ),
    sortRange: useCallback(
      (
        startRow: number,
        endRow: number,
        startCol: number,
        endCol: number,
        sortCol: number,
        ascending: boolean
      ) => ref.current?.sortRange(startRow, endRow, startCol, endCol, sortCol, ascending) ?? [],
      []
    ),
    mergeCells: useCallback(
      (startRow: number, startCol: number, endRow: number, endCol: number) =>
        ref.current?.mergeCells(startRow, startCol, endRow, endCol) ?? [],
      []
    ),
    unmergeCells: useCallback(
      (row: number, col: number) => ref.current?.unmergeCells(row, col) ?? [],
      []
    ),
    getMergedRanges: useCallback(
      () => ref.current?.getMergedRanges() ?? [],
      []
    ),
    isMergedSlave: useCallback(
      (row: number, col: number) => ref.current?.isMergedSlave(row, col) ?? false,
      []
    ),
    getMergeInfo: useCallback(
      (row: number, col: number) => ref.current?.getMergeInfo(row, col) ?? null,
      []
    ),
  };

  return { ref, api };
}
