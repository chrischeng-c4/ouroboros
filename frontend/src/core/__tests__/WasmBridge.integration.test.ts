import { describe, it, expect, beforeEach } from 'vitest';
import * as WasmBridge from '../WasmBridge';

describe('WASM Bridge Integration Tests (Real WASM Module)', () => {
  beforeEach(async () => {
    // Initialize REAL WASM module before each test
    await WasmBridge.initWasm();
  });

  describe('Cell Value Persistence', () => {
    it('setting cell value persists to WASM memory', () => {
      WasmBridge.setCellValue(0, 0, 'Hello');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Hello');
      expect(cellData.displayValue).toBe('Hello');
    });

    it('setting numeric value persists correctly', () => {
      WasmBridge.setCellValue(0, 0, '42');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('42');
      expect(cellData.displayValue).toBe('42');
    });

    it('setting boolean value persists correctly', () => {
      WasmBridge.setCellValue(0, 0, 'TRUE');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('TRUE');
      expect(cellData.displayValue).toBe('TRUE');
    });

    it('overwriting cell value updates correctly', () => {
      WasmBridge.setCellValue(0, 0, 'First');
      WasmBridge.setCellValue(0, 0, 'Second');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Second');
    });

    it('empty cells return null or empty display value', () => {
      const cellData = WasmBridge.getCellData(99, 99);
      // Empty cells may return null or an object with empty displayValue
      if (cellData === null) {
        expect(cellData).toBeNull();
      } else {
        expect(cellData.displayValue).toBe('');
      }
    });
  });

  describe('Formula Evaluation', () => {
    it('formula evaluation updates cached value immediately', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '20');
      WasmBridge.setCellValue(0, 2, '=A1+B1');

      const result = WasmBridge.getCellData(0, 2);
      expect(result.formula).toBe('=A1+B1');
      expect(result.displayValue).toBe('30');
    });

    it('editing formula cell preserves formula expression', () => {
      WasmBridge.setCellValue(0, 0, '=SUM(A1:A10)');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.formula).toBe('=SUM(A1:A10)');
      // Formula should be stored, not just the result
    });

    it('formula with SUM function works correctly', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(1, 0, '20');
      WasmBridge.setCellValue(2, 0, '30');
      WasmBridge.setCellValue(3, 0, '=SUM(A1:A3)');

      const result = WasmBridge.getCellData(3, 0);
      expect(result.formula).toBe('=SUM(A1:A3)');
      expect(result.displayValue).toBe('60');
    });

    it('formula with AVERAGE function works correctly', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(1, 0, '20');
      WasmBridge.setCellValue(2, 0, '30');
      WasmBridge.setCellValue(3, 0, '=AVERAGE(A1:A3)');

      const result = WasmBridge.getCellData(3, 0);
      expect(result.formula).toBe('=AVERAGE(A1:A3)');
      expect(result.displayValue).toBe('20');
    });

    it('formula updates when dependencies change', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '=A1*2');

      let result = WasmBridge.getCellData(0, 1);
      expect(result.displayValue).toBe('20');

      // Update dependency
      WasmBridge.setCellValue(0, 0, '5');

      // Formula should recalculate
      result = WasmBridge.getCellData(0, 1);
      expect(result.displayValue).toBe('10');
    });

    it('nested formulas work correctly', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '=A1*2');
      WasmBridge.setCellValue(0, 2, '=B1+5');

      const result = WasmBridge.getCellData(0, 2);
      expect(result.displayValue).toBe('25'); // (10*2)+5
    });
  });

  describe('Original Input Preservation (Bug #1-3 Fix)', () => {
    it('entering "10" preserves "10" as original input', () => {
      WasmBridge.setCellValue(0, 0, '10');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('10');
      // Not converted to number or anything else
    });

    it('entering percentage preserves original format', () => {
      WasmBridge.setCellValue(0, 0, '50%');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('50%');
    });

    it('entering boolean preserves original case', () => {
      WasmBridge.setCellValue(0, 0, 'TRUE');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('TRUE');
    });

    it('re-editing cell shows original input, not computed value', () => {
      // This was the core bug - editing "10" showed "B2C2" corruption
      WasmBridge.setCellValue(0, 0, '10');

      // Get for editing
      const cellData = WasmBridge.getCellData(0, 0);

      // Should return original input for editing
      expect(cellData.value).toBe('10');
    });
  });

  describe('Cell Reference Display (Bug #8)', () => {
    it('cell (0,0) displays as A1', () => {
      WasmBridge.setCellValue(0, 0, 'Test');
      // This test verifies the cell addressing works
      // The actual column-to-letter conversion is tested separately
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData).toBeTruthy();
    });

    it('supports multiple columns and rows', () => {
      WasmBridge.setCellValue(5, 10, 'Test');
      const cellData = WasmBridge.getCellData(5, 10);
      expect(cellData.value).toBe('Test');
    });
  });

  describe('Cell Dimensions', () => {
    it('getColWidth returns valid width', () => {
      const width = WasmBridge.getColWidth(0);
      expect(width).toBeGreaterThan(0);
      expect(typeof width).toBe('number');
    });

    it('getRowHeight returns valid height', () => {
      const height = WasmBridge.getRowHeight(0);
      expect(height).toBeGreaterThan(0);
      expect(typeof height).toBe('number');
    });
  });

  describe('Multi-Sheet Support', () => {
    it('can add new sheets', () => {
      const initialSheetCount = 1; // Default sheet
      const newSheetIndex = WasmBridge.addSheet('TestSheet');

      expect(newSheetIndex).toBeGreaterThanOrEqual(initialSheetCount);
    });

    it('can switch between sheets', () => {
      // Set value on default sheet
      WasmBridge.setCellValue(0, 0, 'Sheet1Value');

      // Add and switch to new sheet
      const newSheetIndex = WasmBridge.addSheet('Sheet2');
      WasmBridge.setActiveSheet(newSheetIndex);

      // New sheet should be empty (null or empty displayValue)
      const cellData = WasmBridge.getCellData(0, 0);
      if (cellData !== null) {
        expect(cellData.displayValue).toBe('');
      }

      // Switch back to first sheet
      WasmBridge.setActiveSheet(0);

      // Original value should still be there
      const originalData = WasmBridge.getCellData(0, 0);
      expect(originalData?.value).toBe('Sheet1Value');
    });
  });

  describe('Format Application', () => {
    it('can apply formatting to single cell', () => {
      WasmBridge.setCellFormat(0, 0, { bold: true });

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.format?.bold).toBe(true);
    });

    it('can apply formatting to range', () => {
      WasmBridge.setRangeFormat(0, 0, 2, 2, {
        bold: true,
        backgroundColor: '#ff0000',
      });

      // Check first cell in range
      const cell1 = WasmBridge.getCellData(0, 0);
      expect(cell1.format?.bold).toBe(true);
      expect(cell1.format?.backgroundColor).toBe('#ff0000');

      // Check last cell in range
      const cell2 = WasmBridge.getCellData(2, 2);
      expect(cell2.format?.bold).toBe(true);
      expect(cell2.format?.backgroundColor).toBe('#ff0000');
    });
  });

  describe('Serialization and State', () => {
    it('can serialize workbook state', () => {
      WasmBridge.setCellValue(0, 0, 'Test');
      WasmBridge.setCellValue(1, 1, '=A1');

      const serialized = WasmBridge.serialize();

      expect(serialized).toBeTruthy();
      expect(typeof serialized).toBe('string');
      expect(serialized.length).toBeGreaterThan(0);
    });

    it('can deserialize workbook state', () => {
      // Set up state
      WasmBridge.setCellValue(0, 0, 'Original');
      const serialized = WasmBridge.serialize();

      // Clear by setting empty value
      WasmBridge.setCellValue(0, 0, '');

      // Restore
      WasmBridge.deserialize(serialized);

      // Verify restored
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData?.value).toBe('Original');
    });
  });
});
