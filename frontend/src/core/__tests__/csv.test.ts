import { describe, it, expect, beforeEach, vi } from 'vitest';
import Papa from 'papaparse';

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
    serialize: vi.fn(() => '{}'),
    deserialize: vi.fn(() => true),
    // Clear mock cells for test isolation
    __clearMockCells: () => mockCells.clear(),
    __getMockCells: () => mockCells,
  };
});

import { RusheetAPI } from '../RusheetAPI';

describe('CSV Import/Export', () => {
  let api: RusheetAPI;

  beforeEach(async () => {
    // Get a fresh instance
    api = RusheetAPI.getInstance();
    await api.init();
    // Clear mock cells between tests
    const WasmBridge = await import('../WasmBridge');
    (WasmBridge as unknown as { __clearMockCells: () => void }).__clearMockCells();
  });

  describe('exportCSV', () => {
    it('exports empty spreadsheet as empty CSV', () => {
      const csv = api.exportCSV();
      expect(csv).toBe('');
    });

    it('exports single cell', () => {
      api.setCellValue(0, 0, 'Hello');
      const csv = api.exportCSV();
      expect(csv).toBe('Hello');
    });

    it('exports multiple cells in a row', () => {
      api.setCellValue(0, 0, 'A');
      api.setCellValue(0, 1, 'B');
      api.setCellValue(0, 2, 'C');
      const csv = api.exportCSV();
      expect(csv).toBe('A,B,C');
    });

    it('exports multiple rows', () => {
      api.setCellValue(0, 0, 'Name');
      api.setCellValue(0, 1, 'Age');
      api.setCellValue(1, 0, 'Alice');
      api.setCellValue(1, 1, '30');
      const csv = api.exportCSV();
      // papaparse uses \r\n as line separator
      const lines = csv.split(/\r?\n/);
      expect(lines).toHaveLength(2);
      expect(lines[0]).toBe('Name,Age');
      expect(lines[1]).toBe('Alice,30');
    });

    it('uses custom delimiter', () => {
      api.setCellValue(0, 0, 'A');
      api.setCellValue(0, 1, 'B');
      const csv = api.exportCSV({ delimiter: ';' });
      expect(csv).toBe('A;B');
    });

    it('handles values with commas by quoting', () => {
      api.setCellValue(0, 0, 'Hello, World');
      const csv = api.exportCSV();
      expect(csv).toBe('"Hello, World"');
    });

    it('handles values with quotes by escaping', () => {
      api.setCellValue(0, 0, 'Say "Hello"');
      const csv = api.exportCSV();
      expect(csv).toBe('"Say ""Hello"""');
    });

    it('handles values with newlines', () => {
      api.setCellValue(0, 0, 'Line1\nLine2');
      const csv = api.exportCSV();
      expect(csv).toBe('"Line1\nLine2"');
    });
  });

  describe('importCSV', () => {
    it('imports single cell', () => {
      const stats = api.importCSV('Hello');
      expect(stats.rows).toBe(1);
      expect(stats.cols).toBe(1);
      expect(api.getCellData(0, 0)?.value).toBe('Hello');
    });

    it('imports multiple cells in a row', () => {
      const stats = api.importCSV('A,B,C');
      expect(stats.rows).toBe(1);
      expect(stats.cols).toBe(3);
      expect(api.getCellData(0, 0)?.value).toBe('A');
      expect(api.getCellData(0, 1)?.value).toBe('B');
      expect(api.getCellData(0, 2)?.value).toBe('C');
    });

    it('imports multiple rows', () => {
      const stats = api.importCSV('Name,Age\nAlice,30\nBob,25');
      expect(stats.rows).toBe(3);
      expect(stats.cols).toBe(2);
      expect(api.getCellData(0, 0)?.value).toBe('Name');
      expect(api.getCellData(0, 1)?.value).toBe('Age');
      expect(api.getCellData(1, 0)?.value).toBe('Alice');
      expect(api.getCellData(1, 1)?.value).toBe('30');
      expect(api.getCellData(2, 0)?.value).toBe('Bob');
      expect(api.getCellData(2, 1)?.value).toBe('25');
    });

    it('uses custom delimiter', () => {
      const stats = api.importCSV('A;B;C', { delimiter: ';' });
      expect(stats.cols).toBe(3);
      expect(api.getCellData(0, 0)?.value).toBe('A');
      expect(api.getCellData(0, 1)?.value).toBe('B');
      expect(api.getCellData(0, 2)?.value).toBe('C');
    });

    it('handles quoted values with commas', () => {
      api.importCSV('"Hello, World",Test');
      expect(api.getCellData(0, 0)?.value).toBe('Hello, World');
      expect(api.getCellData(0, 1)?.value).toBe('Test');
    });

    it('handles quoted values with escaped quotes', () => {
      api.importCSV('"Say ""Hello""",Test');
      expect(api.getCellData(0, 0)?.value).toBe('Say "Hello"');
    });

    it('imports at specified offset', () => {
      api.importCSV('A,B', { startRow: 5, startCol: 3 });
      expect(api.getCellData(5, 3)?.value).toBe('A');
      expect(api.getCellData(5, 4)?.value).toBe('B');
    });

    it('clears existing data by default', async () => {
      api.setCellValue(0, 0, 'Old');
      const WasmBridge = await import('../WasmBridge');
      api.importCSV('New');
      expect(WasmBridge.clearRange).toHaveBeenCalled();
    });

    it('preserves existing data when clearExisting is false', async () => {
      api.setCellValue(0, 0, 'Old');
      api.setCellValue(0, 1, 'Keep');
      const WasmBridge = await import('../WasmBridge');
      vi.clearAllMocks();
      api.importCSV('New', { clearExisting: false });
      expect(WasmBridge.clearRange).not.toHaveBeenCalled();
    });
  });

  describe('roundtrip', () => {
    it('exports and re-imports data correctly', () => {
      // Set up initial data
      api.setCellValue(0, 0, 'Name');
      api.setCellValue(0, 1, 'Value');
      api.setCellValue(1, 0, 'Item1');
      api.setCellValue(1, 1, '100');

      // Export
      const csv = api.exportCSV();

      // Clear and re-import
      api.importCSV(csv);

      // Verify
      expect(api.getCellData(0, 0)?.value).toBe('Name');
      expect(api.getCellData(0, 1)?.value).toBe('Value');
      expect(api.getCellData(1, 0)?.value).toBe('Item1');
      expect(api.getCellData(1, 1)?.value).toBe('100');
    });

    it('handles special characters in roundtrip', () => {
      api.setCellValue(0, 0, 'Hello, World');
      api.setCellValue(0, 1, 'Say "Hi"');
      api.setCellValue(1, 0, 'Line1\nLine2');

      const csv = api.exportCSV();
      api.importCSV(csv);

      expect(api.getCellData(0, 0)?.value).toBe('Hello, World');
      expect(api.getCellData(0, 1)?.value).toBe('Say "Hi"');
      expect(api.getCellData(1, 0)?.value).toBe('Line1\nLine2');
    });
  });
});

describe('papaparse integration', () => {
  it('parses standard CSV correctly', () => {
    const result = Papa.parse<string[]>('a,b,c\n1,2,3', { skipEmptyLines: false });
    expect(result.data).toEqual([['a', 'b', 'c'], ['1', '2', '3']]);
  });

  it('generates standard CSV correctly', () => {
    const csv = Papa.unparse([['a', 'b', 'c'], ['1', '2', '3']]);
    expect(csv).toBe('a,b,c\r\n1,2,3');
  });

  it('handles empty cells', () => {
    const result = Papa.parse<string[]>('a,,c\n,2,', { skipEmptyLines: false });
    expect(result.data).toEqual([['a', '', 'c'], ['', '2', '']]);
  });
});
