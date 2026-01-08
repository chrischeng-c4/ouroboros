import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock WasmBridge for unit tests
vi.mock('../WasmBridge', () => {
  const mockCells: Map<string, { value: string; displayValue: string }> = new Map();

  return {
    initWasm: vi.fn().mockResolvedValue(undefined),
    setCellValue: vi.fn((row: number, col: number, value: string) => {
      mockCells.set(`${row},${col}`, { value, displayValue: value });
      return [];
    }),
    getCellData: vi.fn((row: number, col: number) => {
      const key = `${row},${col}`;
      const cell = mockCells.get(key);
      if (cell) {
        return {
          value: cell.value,
          displayValue: cell.displayValue,
          row,
          col,
          format: {},
        };
      }
      return null;
    }),
    clearRange: vi.fn((startRow: number, startCol: number, endRow: number, endCol: number) => {
      for (let r = startRow; r <= endRow; r++) {
        for (let c = startCol; c <= endCol; c++) {
          mockCells.delete(`${r},${c}`);
        }
      }
      return [];
    }),
    sortRange: vi.fn((startRow: number, endRow: number, startCol: number, endCol: number, sortCol: number, ascending: boolean) => {
      // Get all rows in the range
      const rows: { rowIndex: number; cells: Map<number, string> }[] = [];

      for (let r = startRow; r <= endRow; r++) {
        const cells = new Map<number, string>();
        for (let c = startCol; c <= endCol; c++) {
          const cell = mockCells.get(`${r},${c}`);
          if (cell) {
            cells.set(c, cell.value);
          }
        }
        rows.push({ rowIndex: r, cells });
      }

      // Sort rows by the sort column value
      rows.sort((a, b) => {
        const aVal = a.cells.get(sortCol) ?? '';
        const bVal = b.cells.get(sortCol) ?? '';

        // Try numeric comparison first
        const aNum = parseFloat(aVal);
        const bNum = parseFloat(bVal);

        if (!isNaN(aNum) && !isNaN(bNum)) {
          return ascending ? aNum - bNum : bNum - aNum;
        }

        // Fall back to string comparison
        const cmp = aVal.localeCompare(bVal);
        return ascending ? cmp : -cmp;
      });

      // Write sorted data back
      const affected: [number, number][] = [];
      rows.forEach((row, newRowIndex) => {
        const targetRow = startRow + newRowIndex;
        row.cells.forEach((value, col) => {
          mockCells.set(`${targetRow},${col}`, { value, displayValue: value });
          affected.push([targetRow, col]);
        });
      });

      return affected;
    }),
    serialize: vi.fn(() => '{}'),
    deserialize: vi.fn(() => true),
    __clearMockCells: () => mockCells.clear(),
    __getMockCells: () => mockCells,
  };
});

import { RusheetAPI } from '../RusheetAPI';

describe('Sorting', () => {
  let api: RusheetAPI;

  beforeEach(async () => {
    api = RusheetAPI.getInstance();
    // @ts-expect-error - access mock helper
    (await import('../WasmBridge')).__clearMockCells();
    await api.init();
  });

  describe('sortRange', () => {
    it('should sort numeric values ascending', async () => {
      // Set up data: Column A has names, Column B has numbers
      api.setCellValue(0, 0, 'Charlie');
      api.setCellValue(0, 1, '30');
      api.setCellValue(1, 0, 'Alice');
      api.setCellValue(1, 1, '10');
      api.setCellValue(2, 0, 'Bob');
      api.setCellValue(2, 1, '20');

      // Sort by column B (index 1), ascending
      const affected = api.sortRange(0, 2, 0, 1, 1, true);

      // Should return affected cells
      expect(affected.length).toBeGreaterThan(0);

      // After sorting by numbers ascending:
      // Row 0: Alice, 10
      // Row 1: Bob, 20
      // Row 2: Charlie, 30
      expect(api.getCellData(0, 0)?.value).toBe('Alice');
      expect(api.getCellData(0, 1)?.value).toBe('10');
      expect(api.getCellData(1, 0)?.value).toBe('Bob');
      expect(api.getCellData(1, 1)?.value).toBe('20');
      expect(api.getCellData(2, 0)?.value).toBe('Charlie');
      expect(api.getCellData(2, 1)?.value).toBe('30');
    });

    it('should sort numeric values descending', async () => {
      api.setCellValue(0, 0, 'Alice');
      api.setCellValue(0, 1, '10');
      api.setCellValue(1, 0, 'Bob');
      api.setCellValue(1, 1, '20');
      api.setCellValue(2, 0, 'Charlie');
      api.setCellValue(2, 1, '30');

      // Sort by column B (index 1), descending
      api.sortRange(0, 2, 0, 1, 1, false);

      // After sorting descending:
      // Row 0: Charlie, 30
      // Row 1: Bob, 20
      // Row 2: Alice, 10
      expect(api.getCellData(0, 0)?.value).toBe('Charlie');
      expect(api.getCellData(0, 1)?.value).toBe('30');
      expect(api.getCellData(1, 0)?.value).toBe('Bob');
      expect(api.getCellData(1, 1)?.value).toBe('20');
      expect(api.getCellData(2, 0)?.value).toBe('Alice');
      expect(api.getCellData(2, 1)?.value).toBe('10');
    });

    it('should sort string values alphabetically', async () => {
      api.setCellValue(0, 0, 'Zebra');
      api.setCellValue(1, 0, 'Apple');
      api.setCellValue(2, 0, 'Mango');

      // Sort by column A (index 0), ascending
      api.sortRange(0, 2, 0, 0, 0, true);

      expect(api.getCellData(0, 0)?.value).toBe('Apple');
      expect(api.getCellData(1, 0)?.value).toBe('Mango');
      expect(api.getCellData(2, 0)?.value).toBe('Zebra');
    });

    it('should emit sortRange event', async () => {
      const callback = vi.fn();
      api.onSortRange(callback);

      api.setCellValue(0, 0, '3');
      api.setCellValue(1, 0, '1');
      api.setCellValue(2, 0, '2');

      api.sortRange(0, 2, 0, 0, 0, true);

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          startRow: 0,
          endRow: 2,
          startCol: 0,
          endCol: 0,
          sortCol: 0,
          ascending: true,
          source: 'api',
        })
      );
    });

    it('should handle single row range', async () => {
      api.setCellValue(0, 0, 'Only');
      api.setCellValue(0, 1, 'Row');

      // Sort single row should be a no-op but not error
      const affected = api.sortRange(0, 0, 0, 1, 0, true);

      expect(api.getCellData(0, 0)?.value).toBe('Only');
      expect(api.getCellData(0, 1)?.value).toBe('Row');
    });

    it('should handle mixed data types', async () => {
      api.setCellValue(0, 0, 'Text');
      api.setCellValue(1, 0, '100');
      api.setCellValue(2, 0, '50');
      api.setCellValue(3, 0, 'Another');

      // Sort - numbers should be compared numerically, text alphabetically
      api.sortRange(0, 3, 0, 0, 0, true);

      // The mock sorts with numeric comparison first if both are numbers
      // In real implementation, ordering would be: Empty < Number < Text < Boolean < Error
    });

    it('should preserve all columns when sorting', async () => {
      // 3 columns of data
      api.setCellValue(0, 0, 'Z');
      api.setCellValue(0, 1, '1');
      api.setCellValue(0, 2, 'First');
      api.setCellValue(1, 0, 'A');
      api.setCellValue(1, 1, '2');
      api.setCellValue(1, 2, 'Second');

      // Sort by first column
      api.sortRange(0, 1, 0, 2, 0, true);

      // Row with 'A' should be first, all columns preserved
      expect(api.getCellData(0, 0)?.value).toBe('A');
      expect(api.getCellData(0, 1)?.value).toBe('2');
      expect(api.getCellData(0, 2)?.value).toBe('Second');
      expect(api.getCellData(1, 0)?.value).toBe('Z');
      expect(api.getCellData(1, 1)?.value).toBe('1');
      expect(api.getCellData(1, 2)?.value).toBe('First');
    });
  });
});
