import { describe, it, expect, beforeEach, vi } from 'vitest';
import { RusheetAPI } from '../RusheetAPI';

// Use vi.hoisted to share state between mock and tests
const mockState = vi.hoisted(() => ({
  cells: new Map<string, { value: string; displayValue: string }>(),
  merges: [] as Array<{ startRow: number; startCol: number; endRow: number; endCol: number }>,
}));

// Mock WasmBridge for unit tests
vi.mock('../WasmBridge', () => {
  const isCellInMerge = (row: number, col: number) => {
    return mockState.merges.find(m =>
      row >= m.startRow && row <= m.endRow &&
      col >= m.startCol && col <= m.endCol
    );
  };

  return {
    initWasm: vi.fn().mockResolvedValue(undefined),
    setCellValue: vi.fn((row: number, col: number, value: string) => {
      mockState.cells.set(`${row},${col}`, { value, displayValue: value });
      return [];
    }),
    getCellData: vi.fn((row: number, col: number) => {
      const key = `${row},${col}`;
      const cell = mockState.cells.get(key);
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
          mockState.cells.delete(`${r},${c}`);
        }
      }
      return [];
    }),
    mergeCells: vi.fn((startRow: number, startCol: number, endRow: number, endCol: number) => {
      // Check for single cell
      if (startRow === endRow && startCol === endCol) {
        return [];
      }

      // Check for overlap with existing merges
      for (const m of mockState.merges) {
        const overlaps = !(endRow < m.startRow || startRow > m.endRow ||
                          endCol < m.startCol || startCol > m.endCol);
        if (overlaps) {
          return [];
        }
      }

      mockState.merges.push({ startRow, startCol, endRow, endCol });

      // Return affected cells
      const affected: [number, number][] = [];
      for (let r = startRow; r <= endRow; r++) {
        for (let c = startCol; c <= endCol; c++) {
          affected.push([r, c]);
        }
      }
      return affected;
    }),
    unmergeCells: vi.fn((row: number, col: number) => {
      const idx = mockState.merges.findIndex(m =>
        row >= m.startRow && row <= m.endRow &&
        col >= m.startCol && col <= m.endCol
      );
      if (idx === -1) return [];

      const merge = mockState.merges[idx];
      mockState.merges.splice(idx, 1);

      const affected: [number, number][] = [];
      for (let r = merge.startRow; r <= merge.endRow; r++) {
        for (let c = merge.startCol; c <= merge.endCol; c++) {
          affected.push([r, c]);
        }
      }
      return affected;
    }),
    getMergedRanges: vi.fn(() => [...mockState.merges]),
    isMergedSlave: vi.fn((row: number, col: number) => {
      const merge = isCellInMerge(row, col);
      if (!merge) return false;
      return !(row === merge.startRow && col === merge.startCol);
    }),
    getMergeInfo: vi.fn((row: number, col: number) => {
      const merge = isCellInMerge(row, col);
      if (!merge) return null;
      return {
        masterRow: merge.startRow,
        masterCol: merge.startCol,
        rowSpan: merge.endRow - merge.startRow + 1,
        colSpan: merge.endCol - merge.startCol + 1,
      };
    }),
    serialize: vi.fn(() => '{}'),
    deserialize: vi.fn(() => true),
  };
});

describe('Cell Merging', () => {
  let api: RusheetAPI;

  beforeEach(async () => {
    // Reset mock state
    mockState.cells.clear();
    mockState.merges.length = 0;

    api = RusheetAPI.getInstance();
    await api.init();
  });

  describe('mergeCells', () => {
    it('should merge a range of cells', async () => {
      const affected = api.mergeCells(0, 0, 2, 2);

      expect(affected.length).toBe(9); // 3x3 = 9 cells
      expect(api.getMergedRanges()).toHaveLength(1);
      expect(api.getMergedRanges()[0]).toEqual({
        startRow: 0, startCol: 0, endRow: 2, endCol: 2
      });
    });

    it('should not merge single cell', async () => {
      const affected = api.mergeCells(0, 0, 0, 0);

      expect(affected.length).toBe(0);
      expect(api.getMergedRanges()).toHaveLength(0);
    });

    it('should not merge overlapping ranges', async () => {
      // First merge
      api.mergeCells(0, 0, 2, 2);
      expect(api.getMergedRanges()).toHaveLength(1);

      // Try to merge overlapping range
      const affected = api.mergeCells(1, 1, 3, 3);

      expect(affected.length).toBe(0);
      expect(api.getMergedRanges()).toHaveLength(1); // Still only 1 merge
    });

    it('should allow non-overlapping merges', async () => {
      api.mergeCells(0, 0, 1, 1);
      api.mergeCells(0, 3, 1, 4);

      expect(api.getMergedRanges()).toHaveLength(2);
    });

    it('should emit mergeCells event', async () => {
      const callback = vi.fn();
      api.onMergeCells(callback);

      api.mergeCells(0, 0, 1, 1);

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          startRow: 0,
          startCol: 0,
          endRow: 1,
          endCol: 1,
          source: 'api',
        })
      );
    });
  });

  describe('unmergeCells', () => {
    it('should unmerge cells at a position', async () => {
      api.mergeCells(0, 0, 2, 2);
      expect(api.getMergedRanges()).toHaveLength(1);

      const affected = api.unmergeCells(0, 0);

      expect(affected.length).toBe(9);
      expect(api.getMergedRanges()).toHaveLength(0);
    });

    it('should unmerge from any cell in the merged range', async () => {
      api.mergeCells(0, 0, 2, 2);

      // Unmerge from middle of the range
      const affected = api.unmergeCells(1, 1);

      expect(affected.length).toBe(9);
      expect(api.getMergedRanges()).toHaveLength(0);
    });

    it('should return empty array for non-merged cell', async () => {
      const affected = api.unmergeCells(5, 5);

      expect(affected.length).toBe(0);
    });

    it('should emit unmergeCells event', async () => {
      api.mergeCells(0, 0, 1, 1);

      const callback = vi.fn();
      api.onUnmergeCells(callback);

      api.unmergeCells(0, 0);

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          row: 0,
          col: 0,
          source: 'api',
        })
      );
    });
  });

  describe('getMergeInfo', () => {
    it('should return merge info for master cell', async () => {
      api.mergeCells(0, 0, 2, 3);

      const info = api.getMergeInfo(0, 0);

      expect(info).toEqual({
        masterRow: 0,
        masterCol: 0,
        rowSpan: 3,
        colSpan: 4,
      });
    });

    it('should return merge info for slave cell', async () => {
      api.mergeCells(0, 0, 2, 3);

      const info = api.getMergeInfo(1, 2);

      expect(info).toEqual({
        masterRow: 0,
        masterCol: 0,
        rowSpan: 3,
        colSpan: 4,
      });
    });

    it('should return null for non-merged cell', async () => {
      const info = api.getMergeInfo(5, 5);

      expect(info).toBeNull();
    });
  });

  describe('isMergedSlave', () => {
    it('should return false for master cell', async () => {
      api.mergeCells(0, 0, 2, 2);

      expect(api.isMergedSlave(0, 0)).toBe(false);
    });

    it('should return true for slave cell', async () => {
      api.mergeCells(0, 0, 2, 2);

      expect(api.isMergedSlave(1, 1)).toBe(true);
      expect(api.isMergedSlave(0, 1)).toBe(true);
      expect(api.isMergedSlave(2, 2)).toBe(true);
    });

    it('should return false for non-merged cell', async () => {
      expect(api.isMergedSlave(5, 5)).toBe(false);
    });
  });

  describe('getMergedRanges', () => {
    it('should return empty array when no merges', async () => {
      expect(api.getMergedRanges()).toEqual([]);
    });

    it('should return all merged ranges', async () => {
      api.mergeCells(0, 0, 1, 1);
      api.mergeCells(5, 5, 6, 6);

      const ranges = api.getMergedRanges();

      expect(ranges).toHaveLength(2);
      expect(ranges).toContainEqual({ startRow: 0, startCol: 0, endRow: 1, endCol: 1 });
      expect(ranges).toContainEqual({ startRow: 5, startCol: 5, endRow: 6, endCol: 6 });
    });
  });
});
