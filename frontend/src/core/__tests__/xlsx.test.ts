import { describe, it, expect, beforeEach, vi } from 'vitest';
import * as XLSX from 'xlsx';

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

describe('XLSX Import/Export', () => {
  let api: RusheetAPI;

  beforeEach(async () => {
    api = RusheetAPI.getInstance();
    await api.init();
    const WasmBridge = await import('../WasmBridge');
    (WasmBridge as unknown as { __clearMockCells: () => void }).__clearMockCells();
  });

  describe('exportXLSX', () => {
    it('exports empty spreadsheet', () => {
      const buffer = api.exportXLSX();
      expect(buffer).toBeInstanceOf(ArrayBuffer);
      expect(buffer.byteLength).toBeGreaterThan(0);

      // Verify it's a valid XLSX
      const wb = XLSX.read(buffer, { type: 'array' });
      expect(wb.SheetNames).toHaveLength(1);
    });

    it('exports single cell', () => {
      api.setCellValue(0, 0, 'Hello');
      const buffer = api.exportXLSX();

      const wb = XLSX.read(buffer, { type: 'array' });
      const ws = wb.Sheets[wb.SheetNames[0]];
      const data = XLSX.utils.sheet_to_json<string[]>(ws, { header: 1 });

      expect(data[0][0]).toBe('Hello');
    });

    it('exports multiple cells', () => {
      api.setCellValue(0, 0, 'Name');
      api.setCellValue(0, 1, 'Age');
      api.setCellValue(1, 0, 'Alice');
      api.setCellValue(1, 1, '30');

      const buffer = api.exportXLSX();
      const wb = XLSX.read(buffer, { type: 'array' });
      const ws = wb.Sheets[wb.SheetNames[0]];
      const data = XLSX.utils.sheet_to_json<string[]>(ws, { header: 1 });

      expect(data[0]).toEqual(['Name', 'Age']);
      expect(data[1]).toEqual(['Alice', '30']);
    });

    it('uses custom sheet name', () => {
      api.setCellValue(0, 0, 'Test');
      const buffer = api.exportXLSX({ sheetName: 'MySheet' });

      const wb = XLSX.read(buffer, { type: 'array' });
      expect(wb.SheetNames[0]).toBe('MySheet');
    });
  });

  describe('importXLSX', () => {
    it('imports single cell', () => {
      // Create test XLSX
      const ws = XLSX.utils.aoa_to_sheet([['Hello']]);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, 'Sheet1');
      const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

      const stats = api.importXLSX(buffer);

      expect(stats.rows).toBe(1);
      expect(stats.cols).toBe(1);
      expect(stats.sheetName).toBe('Sheet1');
      expect(api.getCellData(0, 0)?.value).toBe('Hello');
    });

    it('imports multiple rows', () => {
      const ws = XLSX.utils.aoa_to_sheet([
        ['Name', 'Age'],
        ['Alice', '30'],
        ['Bob', '25'],
      ]);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, 'Data');
      const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

      const stats = api.importXLSX(buffer);

      expect(stats.rows).toBe(3);
      expect(stats.cols).toBe(2);
      expect(api.getCellData(0, 0)?.value).toBe('Name');
      expect(api.getCellData(0, 1)?.value).toBe('Age');
      expect(api.getCellData(1, 0)?.value).toBe('Alice');
      expect(api.getCellData(2, 0)?.value).toBe('Bob');
    });

    it('imports specific sheet by name', () => {
      const ws1 = XLSX.utils.aoa_to_sheet([['Sheet1 Data']]);
      const ws2 = XLSX.utils.aoa_to_sheet([['Sheet2 Data']]);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws1, 'First');
      XLSX.utils.book_append_sheet(wb, ws2, 'Second');
      const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

      const stats = api.importXLSX(buffer, { sheetName: 'Second' });

      expect(stats.sheetName).toBe('Second');
      expect(api.getCellData(0, 0)?.value).toBe('Sheet2 Data');
    });

    it('imports specific sheet by index', () => {
      const ws1 = XLSX.utils.aoa_to_sheet([['Sheet1 Data']]);
      const ws2 = XLSX.utils.aoa_to_sheet([['Sheet2 Data']]);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws1, 'First');
      XLSX.utils.book_append_sheet(wb, ws2, 'Second');
      const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

      const stats = api.importXLSX(buffer, { sheetIndex: 1 });

      expect(stats.sheetName).toBe('Second');
      expect(api.getCellData(0, 0)?.value).toBe('Sheet2 Data');
    });

    it('throws error for non-existent sheet', () => {
      const ws = XLSX.utils.aoa_to_sheet([['Test']]);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, 'Sheet1');
      const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

      expect(() => api.importXLSX(buffer, { sheetName: 'NonExistent' }))
        .toThrow('Sheet not found');
    });
  });

  describe('getXLSXSheetNames', () => {
    it('returns sheet names', () => {
      const ws1 = XLSX.utils.aoa_to_sheet([['1']]);
      const ws2 = XLSX.utils.aoa_to_sheet([['2']]);
      const ws3 = XLSX.utils.aoa_to_sheet([['3']]);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws1, 'Alpha');
      XLSX.utils.book_append_sheet(wb, ws2, 'Beta');
      XLSX.utils.book_append_sheet(wb, ws3, 'Gamma');
      const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

      const names = api.getXLSXSheetNames(buffer);

      expect(names).toEqual(['Alpha', 'Beta', 'Gamma']);
    });
  });

  describe('roundtrip', () => {
    it('exports and re-imports data correctly', () => {
      api.setCellValue(0, 0, 'Name');
      api.setCellValue(0, 1, 'Value');
      api.setCellValue(1, 0, 'Item1');
      api.setCellValue(1, 1, '100');

      const buffer = api.exportXLSX();
      api.importXLSX(buffer);

      expect(api.getCellData(0, 0)?.value).toBe('Name');
      expect(api.getCellData(0, 1)?.value).toBe('Value');
      expect(api.getCellData(1, 0)?.value).toBe('Item1');
      expect(api.getCellData(1, 1)?.value).toBe('100');
    });
  });
});

describe('xlsx library integration', () => {
  it('creates valid XLSX from array', () => {
    const ws = XLSX.utils.aoa_to_sheet([['a', 'b'], ['1', '2']]);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws, 'Test');

    const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });
    expect(buffer.byteLength).toBeGreaterThan(0);
  });

  it('reads XLSX back to array', () => {
    const ws = XLSX.utils.aoa_to_sheet([['a', 'b'], ['1', '2']]);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws, 'Test');
    const buffer = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

    const readWb = XLSX.read(buffer, { type: 'array' });
    const readWs = readWb.Sheets['Test'];
    const data = XLSX.utils.sheet_to_json<string[]>(readWs, { header: 1 });

    expect(data).toEqual([['a', 'b'], ['1', '2']]);
  });
});
