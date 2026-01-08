import { describe, it, expect, beforeEach, vi } from 'vitest';
import { RusheetAPI } from '../RusheetAPI';

const mockState = vi.hoisted(() => ({
  cells: new Map<string, { value: string; displayValue: string }>(),
  hiddenRows: new Set<number>(),
  activeFilters: new Map<number, string[]>(),
}));

// Mock WasmBridge for unit tests
vi.mock('../WasmBridge', () => {
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
    getUniqueValuesInColumn: vi.fn((col: number, maxRows: number = 10000) => {
      const values = new Set<string>();
      for (let row = 0; row < maxRows; row++) {
        const cell = mockState.cells.get(`${row},${col}`);
        if (cell) {
          values.add(cell.displayValue);
        }
      }
      return Array.from(values).sort();
    }),
    applyColumnFilter: vi.fn((col: number, visibleValues: string[], maxRows: number = 10000) => {
      mockState.activeFilters.set(col, visibleValues);
      const affected: number[] = [];

      // Recalculate hidden rows based on all active filters
      mockState.hiddenRows.clear();
      for (let row = 0; row < maxRows; row++) {
        let shouldHide = false;

        // Check each active filter
        for (const [filterCol, filterValues] of mockState.activeFilters.entries()) {
          const cell = mockState.cells.get(`${row},${filterCol}`);
          const cellValue = cell ? cell.displayValue : '';

          if (!filterValues.includes(cellValue)) {
            shouldHide = true;
            break;
          }
        }

        if (shouldHide) {
          mockState.hiddenRows.add(row);
          affected.push(row);
        }
      }

      return affected;
    }),
    clearColumnFilter: vi.fn((col: number) => {
      mockState.activeFilters.delete(col);
      const affected: number[] = [];

      // Recalculate hidden rows without this filter
      const previouslyHidden = new Set(mockState.hiddenRows);
      mockState.hiddenRows.clear();

      if (mockState.activeFilters.size > 0) {
        for (let row = 0; row < 10000; row++) {
          let shouldHide = false;

          for (const [filterCol, filterValues] of mockState.activeFilters.entries()) {
            const cell = mockState.cells.get(`${row},${filterCol}`);
            const cellValue = cell ? cell.displayValue : '';

            if (!filterValues.includes(cellValue)) {
              shouldHide = true;
              break;
            }
          }

          if (shouldHide) {
            mockState.hiddenRows.add(row);
          }
        }
      }

      // Return rows that changed visibility
      for (const row of previouslyHidden) {
        if (!mockState.hiddenRows.has(row)) {
          affected.push(row);
        }
      }

      return affected;
    }),
    clearAllFilters: vi.fn(() => {
      mockState.activeFilters.clear();
      const affected = Array.from(mockState.hiddenRows);
      mockState.hiddenRows.clear();
      return affected;
    }),
    getActiveFilters: vi.fn(() => {
      return Array.from(mockState.activeFilters.entries()).map(([col, visibleValues]) => ({
        col,
        visibleValues,
      }));
    }),
    isRowHidden: vi.fn((row: number) => {
      return mockState.hiddenRows.has(row);
    }),
    serialize: vi.fn(() => '{}'),
    deserialize: vi.fn(() => true),
    getHiddenRows: vi.fn(() => Array.from(mockState.hiddenRows)),
  };
});

describe('Filtering', () => {
  let api: RusheetAPI;

  beforeEach(async () => {
    api = RusheetAPI.getInstance();
    
    // Reset mock state
    mockState.cells.clear();
    mockState.hiddenRows.clear();
    mockState.activeFilters.clear();
    
    await api.init();
  });

  describe('Basic filter operations', () => {
    it('getUniqueValuesInColumn returns correct unique values', async () => {
      // Set up data with duplicates
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(2, 0, 'Apple');
      api.setCellValue(3, 0, 'Cherry');
      api.setCellValue(4, 0, 'Banana');

      const uniqueValues = api.getUniqueValuesInColumn(0);

      expect(uniqueValues).toEqual(['Apple', 'Banana', 'Cherry']);
      expect(uniqueValues.length).toBe(3);
    });

    it('getUniqueValuesInColumn returns sorted values', async () => {
      api.setCellValue(0, 0, 'Zebra');
      api.setCellValue(1, 0, 'Apple');
      api.setCellValue(2, 0, 'Mango');

      const uniqueValues = api.getUniqueValuesInColumn(0);

      expect(uniqueValues).toEqual(['Apple', 'Mango', 'Zebra']);
    });

    it('applyColumnFilter hides rows not matching filter', async () => {
      // Set up data
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(2, 0, 'Cherry');
      api.setCellValue(3, 0, 'Apple');

      // Filter to show only 'Apple' and 'Cherry'
      api.applyColumnFilter(0, ['Apple', 'Cherry']);

      // Row 1 (Banana) should be hidden
      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(true);
      expect(api.isRowHidden(2)).toBe(false);
      expect(api.isRowHidden(3)).toBe(false);
    });

    it('clearColumnFilter unhides rows', async () => {
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(2, 0, 'Cherry');

      // Apply filter
      api.applyColumnFilter(0, ['Apple']);
      expect(api.isRowHidden(1)).toBe(true);
      expect(api.isRowHidden(2)).toBe(true);

      // Clear filter
      api.clearColumnFilter(0);
      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(false);
      expect(api.isRowHidden(2)).toBe(false);
    });

    it('clearAllFilters removes all filters', async () => {
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(0, 1, 'Red');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(1, 1, 'Yellow');

      // Apply filters on two columns
      api.applyColumnFilter(0, ['Apple']);
      api.applyColumnFilter(1, ['Red']);

      expect(api.isRowHidden(1)).toBe(true);

      // Clear all filters
      api.clearAllFilters();
      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(false);
    });
  });

  describe('Filter state', () => {
    it('getActiveFilters returns correct state', async () => {
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(0, 1, 'Red');

      // Apply filters
      api.applyColumnFilter(0, ['Apple', 'Banana']);
      api.applyColumnFilter(1, ['Red', 'Green']);

      const activeFilters = api.getActiveFilters();

      expect(activeFilters).toHaveLength(2);
      expect(activeFilters).toEqual(
        expect.arrayContaining([
          { col: 0, visibleValues: ['Apple', 'Banana'] },
          { col: 1, visibleValues: ['Red', 'Green'] },
        ])
      );
    });

    it('getActiveFilters returns empty array when no filters active', async () => {
      const activeFilters = api.getActiveFilters();
      expect(activeFilters).toEqual([]);
    });

    it('isRowHidden returns correct value for filtered rows', async () => {
      api.setCellValue(0, 0, 'Show');
      api.setCellValue(1, 0, 'Hide');
      api.setCellValue(2, 0, 'Show');

      api.applyColumnFilter(0, ['Show']);

      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(true);
      expect(api.isRowHidden(2)).toBe(false);
    });

    it('isRowHidden returns false for unfiltered rows', async () => {
      api.setCellValue(0, 0, 'Data');
      api.setCellValue(1, 0, 'Data');

      // No filter applied
      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(false);
    });
  });

  describe('Multiple filters', () => {
    it('applies filters on multiple columns', async () => {
      // Set up a table with Fruit and Color columns
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(0, 1, 'Red');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(1, 1, 'Yellow');
      api.setCellValue(2, 0, 'Apple');
      api.setCellValue(2, 1, 'Green');
      api.setCellValue(3, 0, 'Cherry');
      api.setCellValue(3, 1, 'Red');

      // Filter: Show only Apples
      api.applyColumnFilter(0, ['Apple']);
      expect(api.isRowHidden(0)).toBe(false); // Apple, Red
      expect(api.isRowHidden(1)).toBe(true);  // Banana, Yellow
      expect(api.isRowHidden(2)).toBe(false); // Apple, Green
      expect(api.isRowHidden(3)).toBe(true);  // Cherry, Red

      // Add filter: Show only Red color
      api.applyColumnFilter(1, ['Red']);
      expect(api.isRowHidden(0)).toBe(false); // Apple, Red - matches both
      expect(api.isRowHidden(1)).toBe(true);  // Banana, Yellow - matches neither
      expect(api.isRowHidden(2)).toBe(true);  // Apple, Green - matches fruit but not color
      expect(api.isRowHidden(3)).toBe(true);  // Cherry, Red - matches color but not fruit
    });

    it('clearing one filter does not affect others', async () => {
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(0, 1, 'Red');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(1, 1, 'Yellow');
      api.setCellValue(2, 0, 'Apple');
      api.setCellValue(2, 1, 'Green');

      // Apply two filters
      api.applyColumnFilter(0, ['Apple']);
      api.applyColumnFilter(1, ['Red', 'Green']);

      // Only row 0 and 2 should be visible (Apple with Red or Green)
      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(true);
      expect(api.isRowHidden(2)).toBe(false);

      // Clear fruit filter
      api.clearColumnFilter(0);

      // Now only color filter applies - rows with Red or Green
      expect(api.isRowHidden(0)).toBe(false); // Red
      expect(api.isRowHidden(1)).toBe(true);  // Yellow
      expect(api.isRowHidden(2)).toBe(false); // Green

      // Verify color filter is still active
      const activeFilters = api.getActiveFilters();
      expect(activeFilters).toHaveLength(1);
      expect(activeFilters[0].col).toBe(1);
    });

    it('reapplying filter on same column updates it', async () => {
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(2, 0, 'Cherry');

      // First filter: show only Apple
      api.applyColumnFilter(0, ['Apple']);
      expect(api.isRowHidden(1)).toBe(true);
      expect(api.isRowHidden(2)).toBe(true);

      // Update filter: show Apple and Banana
      api.applyColumnFilter(0, ['Apple', 'Banana']);
      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(false);
      expect(api.isRowHidden(2)).toBe(true);
    });
  });

  describe('Edge cases', () => {
    it('filters on empty column', async () => {
      // Column has no data
      const uniqueValues = api.getUniqueValuesInColumn(5);
      expect(uniqueValues).toEqual([]);

      // Applying filter on empty column should not error
      api.applyColumnFilter(5, []);
      expect(api.getActiveFilters()).toHaveLength(1);
    });

    it('filters with no matching values', async () => {
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(1, 0, 'Banana');
      api.setCellValue(2, 0, 'Cherry');

      // Filter to show only values that don't exist
      api.applyColumnFilter(0, ['Mango', 'Grape']);

      // All rows should be hidden
      expect(api.isRowHidden(0)).toBe(true);
      expect(api.isRowHidden(1)).toBe(true);
      expect(api.isRowHidden(2)).toBe(true);
    });

    it('filters on column with formulas', async () => {
      // Set up some cells with formulas
      api.setCellValue(0, 0, '10');
      api.setCellValue(1, 0, '20');
      api.setCellValue(2, 0, '30');

      // In real implementation, formulas would have displayValue different from value
      // For this mock, we just use the value directly
      const uniqueValues = api.getUniqueValuesInColumn(0);
      expect(uniqueValues).toContain('10');
      expect(uniqueValues).toContain('20');
      expect(uniqueValues).toContain('30');

      // Filter should work on display values
      api.applyColumnFilter(0, ['10', '30']);
      expect(api.isRowHidden(0)).toBe(false);
      expect(api.isRowHidden(1)).toBe(true);
      expect(api.isRowHidden(2)).toBe(false);
    });

    it('handles empty string values in filter', async () => {
      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(1, 0, '');
      api.setCellValue(2, 0, 'Banana');
      // Row 3 has no cell (null/undefined)

      const uniqueValues = api.getUniqueValuesInColumn(0);
      // Empty cells should be included as empty string
      expect(uniqueValues).toContain('');
      expect(uniqueValues).toContain('Apple');
      expect(uniqueValues).toContain('Banana');

      // Filter to show only empty values
      api.applyColumnFilter(0, ['']);
      expect(api.isRowHidden(0)).toBe(true);  // Apple
      expect(api.isRowHidden(1)).toBe(false); // Empty string
      expect(api.isRowHidden(2)).toBe(true);  // Banana
      expect(api.isRowHidden(3)).toBe(false); // No cell (treated as empty)
    });

    it('handles large number of unique values', async () => {
      // Set up 100 rows with unique values
      for (let i = 0; i < 100; i++) {
        api.setCellValue(i, 0, `Value${i}`);
      }

      const uniqueValues = api.getUniqueValuesInColumn(0);
      expect(uniqueValues).toHaveLength(100);

      // Filter to show only first 10 values
      const visibleValues = uniqueValues.slice(0, 10);
      api.applyColumnFilter(0, visibleValues);

      // Check some samples
      expect(api.isRowHidden(0)).toBe(false); // Value0 is in visible set
      expect(api.isRowHidden(50)).toBe(true); // Value50 is not
    });
  });

  describe('Events', () => {
    it('emits filterChange event when applying filter', async () => {
      const callback = vi.fn();
      api.onFilterChange(callback);

      api.setCellValue(0, 0, 'Apple');
      api.setCellValue(1, 0, 'Banana');

      api.applyColumnFilter(0, ['Apple']);

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          col: 0,
          visibleValues: ['Apple'],
          affected: expect.any(Array),
        })
      );
    });

    it('emits filterChange event when clearing column filter', async () => {
      const callback = vi.fn();

      api.setCellValue(0, 0, 'Apple');
      api.applyColumnFilter(0, ['Apple']);

      // Register callback after applying filter
      api.onFilterChange(callback);
      api.clearColumnFilter(0);

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          col: 0,
          cleared: true,
          affected: expect.any(Array),
        })
      );
    });

    it('emits filterChange event when clearing all filters', async () => {
      const callback = vi.fn();

      api.setCellValue(0, 0, 'Apple');
      api.applyColumnFilter(0, ['Apple']);

      api.onFilterChange(callback);
      api.clearAllFilters();

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          cleared: true,
          all: true,
          affected: expect.any(Array),
        })
      );
    });
  });
});
