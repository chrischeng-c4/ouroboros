import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockValues = vi.hoisted(() => ({
  undo: [[0, 0]],
  redo: [[0, 0]],
}));

// Mock WasmBridge before importing RusheetAPI
vi.mock('../WasmBridge', () => ({
  initWasm: vi.fn().mockResolvedValue(undefined),
  getCellData: vi.fn().mockReturnValue({ value: 'test', displayValue: 'test', format: {} }),
  setCellValue: vi.fn().mockReturnValue([]),
  setCellFormat: vi.fn().mockReturnValue(true),
  setRangeFormat: vi.fn().mockReturnValue(true),
  clearRange: vi.fn().mockReturnValue([]),
  undo: vi.fn().mockReturnValue(mockValues.undo),
  redo: vi.fn().mockReturnValue(mockValues.redo),
  canUndo: vi.fn().mockReturnValue(true),
  canRedo: vi.fn().mockReturnValue(true),
  addSheet: vi.fn().mockReturnValue(1),
  deleteSheet: vi.fn().mockReturnValue(true),
  renameSheet: vi.fn().mockReturnValue(true),
  setActiveSheet: vi.fn().mockReturnValue(true),
  getSheetNames: vi.fn().mockReturnValue(['Sheet1', 'Sheet2']),
  getActiveSheetIndex: vi.fn().mockReturnValue(0),
  setRowHeight: vi.fn(),
  setColWidth: vi.fn(),
  getRowHeight: vi.fn().mockReturnValue(21),
  getColWidth: vi.fn().mockReturnValue(100),
  serialize: vi.fn().mockReturnValue('{}'),
  deserialize: vi.fn().mockReturnValue(true),
  getViewportData: vi.fn().mockReturnValue([]),
  getViewportArrays: vi.fn().mockReturnValue({ rows: new Uint32Array(0), cols: new Uint32Array(0), values: new Float64Array(0), formats: new Uint32Array(0), displayValues: [], length: 0 }),
  // Mock filter functions
  getUniqueValuesInColumn: vi.fn().mockReturnValue([]),
  applyColumnFilter: vi.fn().mockReturnValue([]),
  clearColumnFilter: vi.fn().mockReturnValue([]),
  clearAllFilters: vi.fn().mockReturnValue([]),
  getActiveFilters: vi.fn().mockReturnValue([]),
  isRowHidden: vi.fn().mockReturnValue(false),
  getHiddenRows: vi.fn().mockReturnValue([]),
  // Mock merging functions
  mergeCells: vi.fn().mockReturnValue([]),
  unmergeCells: vi.fn().mockReturnValue([]),
  getMergedRanges: vi.fn().mockReturnValue([]),
  isMergedSlave: vi.fn().mockReturnValue(false),
  getMergeInfo: vi.fn().mockReturnValue(null),
  // Mock row/col operations
  insertRows: vi.fn().mockReturnValue([]),
  deleteRows: vi.fn().mockReturnValue([]),
  insertCols: vi.fn().mockReturnValue([]),
  deleteCols: vi.fn().mockReturnValue([]),
  sortRange: vi.fn().mockReturnValue([]),
}));

import { RusheetAPI } from '../RusheetAPI';
import { emitter } from '../EventEmitter';
import type {
  CellChangeEvent,
  FormatChangeEvent,
  SheetAddEvent,
  SheetDeleteEvent,
  SheetRenameEvent,
  ActiveSheetChangeEvent,
  UndoEvent,
  RedoEvent
} from '../../types/events';

describe('RusheetAPI Event System', () => {
  let api: RusheetAPI;

  beforeEach(async () => {
    // Clear all event listeners
    emitter.removeAllListeners();
    // Get fresh instance
    api = RusheetAPI.getInstance();
    await api.init();
  });

  afterEach(() => {
    emitter.removeAllListeners();
    vi.clearAllMocks();
  });

  describe('onChange', () => {
    it('should emit change event when setCellValue is called', () => {
      const callback = vi.fn();
      api.onChange(callback);

      api.setCellValue(0, 0, 'Hello', 'user');

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          row: 0,
          col: 0,
          newValue: 'Hello',
          source: 'user',
        })
      );
    });

    it('should return unsubscribe function', () => {
      const callback = vi.fn();
      const unsubscribe = api.onChange(callback);

      api.setCellValue(0, 0, 'First', 'api');
      expect(callback).toHaveBeenCalledTimes(1);

      unsubscribe();

      api.setCellValue(0, 0, 'Second', 'api');
      expect(callback).toHaveBeenCalledTimes(1); // Still 1, not 2
    });
  });

  describe('onFormatChange', () => {
    it('should emit formatChange event for single cell', () => {
      const callback = vi.fn();
      api.onFormatChange(callback);

      api.setCellFormat(0, 0, { bold: true }, 'user');

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'cell',
          startRow: 0,
          startCol: 0,
          endRow: 0,
          endCol: 0,
          format: { bold: true },
          source: 'user',
        })
      );
    });

    it('should emit formatChange event for range', () => {
      const callback = vi.fn();
      api.onFormatChange(callback);

      api.setRangeFormat(0, 0, 2, 3, { italic: true }, 'api');

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'range',
          startRow: 0,
          startCol: 0,
          endRow: 2,
          endCol: 3,
          format: { italic: true },
          source: 'api',
        })
      );
    });
  });

  describe('onSheetAdd', () => {
    it('should emit sheetAdd event when adding sheet', () => {
      const callback = vi.fn();
      api.onSheetAdd(callback);

      api.addSheet('NewSheet', 'user');

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          index: 1,
          name: 'NewSheet',
          source: 'user',
        })
      );
    });
  });

  describe('onSheetRename', () => {
    it('should emit sheetRename event on success', () => {
      const callback = vi.fn();
      api.onSheetRename(callback);

      // WasmBridge mock returns true by default
      const success = api.renameSheet(0, 'RenamedSheet', 'user');

      expect(success).toBe(true);
      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          index: 0,
          oldName: 'Sheet1',
          newName: 'RenamedSheet',
          source: 'user',
        })
      );
    });

    it('should handle errors gracefully', async () => {
      const callback = vi.fn();
      api.onSheetRename(callback);

      // Mock WasmBridge to throw an error
      const wasmBridge = await import('../WasmBridge');
      vi.mocked(wasmBridge.renameSheet).mockImplementationOnce(() => {
        throw { code: 'SHEET_NAME_EXISTS', message: 'Name exists' };
      });

      const success = api.renameSheet(0, 'ExistingName', 'user');

      expect(success).toBe(false);
      expect(callback).not.toHaveBeenCalled();
    });
  });

  describe('onSheetDelete', () => {
    it('should emit sheetDelete event on success', () => {
      const callback = vi.fn();
      api.onSheetDelete(callback);

      const success = api.deleteSheet(0, 'user');

      expect(success).toBe(true);
      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          index: 0,
          name: 'Sheet1',
          source: 'user',
        })
      );
    });

    it('should handle errors gracefully', async () => {
      const callback = vi.fn();
      api.onSheetDelete(callback);

      const wasmBridge = await import('../WasmBridge');
      vi.mocked(wasmBridge.deleteSheet).mockImplementationOnce(() => {
        throw { code: 'CANNOT_DELETE_LAST_SHEET', message: 'Last sheet' };
      });

      const success = api.deleteSheet(0, 'user');

      expect(success).toBe(false);
      expect(callback).not.toHaveBeenCalled();
    });
  });

  describe('onActiveSheetChange', () => {
    it('should emit activeSheetChange event', () => {
      const callback = vi.fn();
      api.onActiveSheetChange(callback);

      api.setActiveSheet(1, 'user');

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          previousIndex: 0,
          newIndex: 1,
          source: 'user',
        })
      );
    });
  });

  describe('onUndo/onRedo', () => {
    it('should emit undo event with affected cells', () => {
      const callback = vi.fn();
      api.onUndo(callback);

      api.undo();

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          affectedCells: [[0, 0]],
        })
      );
    });

    it('should emit redo event with affected cells', () => {
      const callback = vi.fn();
      api.onRedo(callback);

      api.redo();

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          affectedCells: [[0, 0]],
        })
      );
    });

    it('should also emit change events for undo', () => {
      const changeCallback = vi.fn();
      api.onChange(changeCallback);

      api.undo();

      expect(changeCallback).toHaveBeenCalledTimes(1);
      expect(changeCallback).toHaveBeenCalledWith(
        expect.objectContaining({
          source: 'undo',
        })
      );
    });
  });

  describe('multiple subscribers', () => {
    it('should notify all subscribers', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();
      const callback3 = vi.fn();

      api.onChange(callback1);
      api.onChange(callback2);
      api.onChange(callback3);

      api.setCellValue(0, 0, 'Test', 'api');

      expect(callback1).toHaveBeenCalledTimes(1);
      expect(callback2).toHaveBeenCalledTimes(1);
      expect(callback3).toHaveBeenCalledTimes(1);
    });

    it('should allow independent unsubscription', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();

      const unsub1 = api.onChange(callback1);
      api.onChange(callback2);

      unsub1();

      api.setCellValue(0, 0, 'Test', 'api');

      expect(callback1).not.toHaveBeenCalled();
      expect(callback2).toHaveBeenCalledTimes(1);
    });
  });
});
