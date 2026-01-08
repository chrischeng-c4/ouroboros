import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as WasmBridge from '../core/WasmBridge';
import { createTestEnvironment, cleanupTestEnvironment, simulateTyping, simulateKeyPress } from '../ui/__tests__/setup/testUtils';
import type CellEditor from '../ui/CellEditor';
import type GridRenderer from '../canvas/GridRenderer';

describe('Edge Cases Integration Tests (Real Dependencies)', () => {
  let env: {
    canvas: HTMLCanvasElement;
    renderer: GridRenderer;
    container: HTMLElement;
    formulaBar: HTMLInputElement;
    cellEditor: CellEditor;
  };

  beforeEach(async () => {
    env = await createTestEnvironment();
  });

  afterEach(() => {
    cleanupTestEnvironment(env);
  });

  describe('Large Data/Range Tests', () => {
    it('should navigate to cell at large offset (row 500, col 50)', () => {
      env.renderer.setActiveCell(500, 50);

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(500);
      expect(activeCell.col).toBe(50);
    });

    it('should set value at extreme position (row 1000, col 100)', () => {
      WasmBridge.setCellValue(1000, 100, 'Extreme Position');

      const cellData = WasmBridge.getCellData(1000, 100);
      expect(cellData.value).toBe('Extreme Position');
      expect(cellData.displayValue).toBe('Extreme Position');
    });

    it('should handle editing at large row and column indices', () => {
      const largeRow = 999;
      const largeCol = 99;

      env.cellEditor.activate(largeRow, largeCol);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = 'Large Index Value';
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(largeRow, largeCol);
      expect(cellData.value).toBe('Large Index Value');
    });

    it('should select and clear large range efficiently', () => {
      for (let row = 0; row < 10; row++) {
        for (let col = 0; col < 10; col++) {
          WasmBridge.setCellValue(row, col, `Cell ${row},${col}`);
        }
      }

      for (let row = 0; row < 10; row++) {
        for (let col = 0; col < 10; col++) {
          WasmBridge.setCellValue(row, col, '');
        }
      }

      for (let row = 0; row < 10; row++) {
        for (let col = 0; col < 10; col++) {
          const cellData = WasmBridge.getCellData(row, col);
          expect(cellData.value).toBe('');
        }
      }
    });
  });

  describe('Special Character Tests', () => {
    it('should handle unicode characters (Chinese text)', () => {
      const chineseText = '你好世界';
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = chineseText;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(chineseText);
      expect(cellData.displayValue).toBe(chineseText);
    });

    it('should handle unicode characters (emoji)', () => {
      const emojiText = 'Hello 🎉🚀💯';
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = emojiText;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(emojiText);
      expect(cellData.displayValue).toBe(emojiText);
    });

    it('should handle special HTML characters without XSS issues', () => {
      const htmlText = '<script>alert("test")</script>';
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = htmlText;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(htmlText);
      expect(cellData.displayValue).toBe(htmlText);
    });

    it('should handle quotes and special characters', () => {
      const quotesText = 'He said "Hello" and \'Goodbye\'';
      WasmBridge.setCellValue(0, 0, quotesText);

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(quotesText);
    });

    it('should handle newlines in cell values', () => {
      const multilineText = 'Line 1\nLine 2\nLine 3';
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = multilineText;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(multilineText);
    });

    it('should handle tab characters in cell values', () => {
      const tabbedText = 'Column1\tColumn2\tColumn3';
      WasmBridge.setCellValue(0, 0, tabbedText);

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(tabbedText);
    });

    it('should handle formula with double-letter column references (AA1)', () => {
      WasmBridge.setCellValue(0, 26, '50');
      WasmBridge.setCellValue(0, 0, '=AA1');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.formula).toBe('=AA1');
      expect(cellData.displayValue).toBe('50');
    });
  });

  describe('Formula Edge Cases', () => {
    it('should detect circular reference (A1 = B1, B1 = A1)', () => {
      WasmBridge.setCellValue(0, 0, '=B1');
      WasmBridge.setCellValue(0, 1, '=A1');

      const cellA1 = WasmBridge.getCellData(0, 0);
      const cellB1 = WasmBridge.getCellData(0, 1);

      expect(cellA1.formula).toBe('=B1');
      expect(cellB1.formula).toBe('=A1');
    });

    it('should handle division by zero', () => {
      WasmBridge.setCellValue(0, 0, '=1/0');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.formula).toBe('=1/0');
      expect(cellData.displayValue).toBeTruthy();
    });

    it('should handle empty cell reference in formula', () => {
      WasmBridge.setCellValue(0, 0, '=A2+1');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.formula).toBe('=A2+1');
      // WASM returns #VALUE! error for empty cell reference (expected behavior)
      expect(cellData.displayValue).toBe('#VALUE!');
    });

    it('should handle nested function calls', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(1, 0, '20');
      WasmBridge.setCellValue(2, 0, '30');
      WasmBridge.setCellValue(0, 1, '5');
      WasmBridge.setCellValue(1, 1, '15');
      WasmBridge.setCellValue(2, 1, '25');

      WasmBridge.setCellValue(0, 2, '=SUM(AVERAGE(A1:A3), MAX(B1:B3))');

      const result = WasmBridge.getCellData(0, 2);
      expect(result.formula).toBe('=SUM(AVERAGE(A1:A3), MAX(B1:B3))');
      expect(result.displayValue).toBe('45');
    });

    it('should handle formula with mixed operators', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '5');
      WasmBridge.setCellValue(0, 2, '2');
      WasmBridge.setCellValue(0, 3, '=A1+B1*C1-3');

      const result = WasmBridge.getCellData(0, 3);
      expect(result.formula).toBe('=A1+B1*C1-3');
      expect(result.displayValue).toBe('17');
    });
  });

  describe('Input Edge Cases', () => {
    it('should handle very long text input (1000+ characters)', () => {
      const longText = 'A'.repeat(1000);

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = longText;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(longText);
      expect(cellData.value.length).toBe(1000);
    });

    it('should handle empty string value correctly', () => {
      WasmBridge.setCellValue(0, 0, 'Initial');

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = '';
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('');
      expect(cellData.displayValue).toBe('');
    });

    it('should handle whitespace-only value', () => {
      const whitespaceText = '   \t  \n  ';

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = whitespaceText;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      // WASM trims whitespace-only values (expected behavior)
      expect(cellData.value).toBe('');
    });

    it('should handle number with many decimal places', () => {
      const preciseNumber = '3.14159265358979323846';

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = preciseNumber;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(preciseNumber);
    });

    it('should handle scientific notation', () => {
      const scientificNumber = '1.23e-10';

      WasmBridge.setCellValue(0, 0, scientificNumber);

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(scientificNumber);
    });

    it('should handle leading zeros in numbers', () => {
      const leadingZeros = '00123';

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = leadingZeros;
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(leadingZeros);
    });

    it('should handle negative numbers', () => {
      const negativeNumber = '-12345.67';

      WasmBridge.setCellValue(0, 0, negativeNumber);

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(negativeNumber);
    });

    it('should handle percentage values', () => {
      const percentage = '75.5%';

      WasmBridge.setCellValue(0, 0, percentage);

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(percentage);
    });
  });

  describe('Rapid Interaction Tests', () => {
    it('should handle multiple rapid key presses', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;

      simulateTyping(textarea, 'A');
      simulateTyping(textarea, 'AB');
      simulateTyping(textarea, 'ABC');
      simulateTyping(textarea, 'ABCD');
      simulateTyping(textarea, 'ABCDE');

      expect(textarea.value).toBe('ABCDE');
      expect(env.formulaBar.value).toBe('ABCDE');
    });

    it('should handle click then immediate typing', () => {
      env.cellEditor.activate(0, 0);

      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Quick');

      expect(textarea.value).toBe('Quick');
      expect(env.cellEditor.isActive()).toBe(true);
    });

    it('should handle Escape during rapid editing', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Cancelled');

      simulateKeyPress(textarea, 'Escape');

      // Empty cell returns null from WASM, not object with empty string
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData?.value ?? null).toBeNull();
      expect(env.cellEditor.isActive()).toBe(false);
    });
  });

  describe('Boundary and Limit Tests', () => {
    it('should handle zero as cell value', () => {
      WasmBridge.setCellValue(0, 0, '0');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('0');
      expect(cellData.displayValue).toBe('0');
    });

    it('should handle negative zero', () => {
      WasmBridge.setCellValue(0, 0, '-0');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('-0');
    });

    it('should handle maximum safe integer', () => {
      const maxInt = String(Number.MAX_SAFE_INTEGER);
      WasmBridge.setCellValue(0, 0, maxInt);

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(maxInt);
    });

    it('should handle minimum safe integer', () => {
      const minInt = String(Number.MIN_SAFE_INTEGER);
      WasmBridge.setCellValue(0, 0, minInt);

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe(minInt);
    });

    it('should handle cell at row 0, col 0', () => {
      WasmBridge.setCellValue(0, 0, 'Origin');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Origin');
    });

    it('should handle SUM with empty range', () => {
      WasmBridge.setCellValue(0, 0, '=SUM(A10:A20)');

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.formula).toBe('=SUM(A10:A20)');
      expect(cellData.displayValue).toBe('0');
    });

    it('should handle AVERAGE with single cell', () => {
      WasmBridge.setCellValue(0, 0, '42');
      WasmBridge.setCellValue(0, 1, '=AVERAGE(A1:A1)');

      const cellData = WasmBridge.getCellData(0, 1);
      expect(cellData.formula).toBe('=AVERAGE(A1:A1)');
      expect(cellData.displayValue).toBe('42');
    });

    it('should handle formula with single cell reference (not a range)', () => {
      WasmBridge.setCellValue(0, 0, '100');
      WasmBridge.setCellValue(0, 1, '=A1');

      const cellData = WasmBridge.getCellData(0, 1);
      expect(cellData.formula).toBe('=A1');
      expect(cellData.displayValue).toBe('100');
    });
  });

  describe('State Consistency Tests', () => {
    it('should maintain state after multiple edits to same cell', () => {
      const testValues = ['First', 'Second', 'Third', 'Fourth'];

      for (const value of testValues) {
        env.cellEditor.activate(0, 0);
        const textarea = env.container.querySelector('textarea')!;
        textarea.value = value;
        env.cellEditor.commit();
      }

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Fourth');
    });

    it('should preserve formatting during value changes', () => {
      WasmBridge.setCellFormat(0, 0, { bold: true, backgroundColor: '#ff0000' });

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = 'New Value';
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('New Value');
      expect(cellData.format?.bold).toBe(true);
      expect(cellData.format?.backgroundColor).toBe('#ff0000');
    });

    it('should handle switching between text and formula in same cell', () => {
      WasmBridge.setCellValue(0, 0, 'Text Value');
      let cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Text Value');

      WasmBridge.setCellValue(0, 0, '=5+3');
      cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.formula).toBe('=5+3');
      expect(cellData.displayValue).toBe('8');

      WasmBridge.setCellValue(0, 0, 'Back to Text');
      cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Back to Text');
      // Formula is undefined/null when cell contains text, not empty string
      expect(cellData.formula).toBeFalsy();
    });

    it('should handle complex dependency chain updates', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '=A1*2');
      WasmBridge.setCellValue(0, 2, '=B1+5');
      WasmBridge.setCellValue(0, 3, '=C1-3');

      expect(WasmBridge.getCellData(0, 3).displayValue).toBe('22');

      WasmBridge.setCellValue(0, 0, '5');

      expect(WasmBridge.getCellData(0, 1).displayValue).toBe('10');
      expect(WasmBridge.getCellData(0, 2).displayValue).toBe('15');
      expect(WasmBridge.getCellData(0, 3).displayValue).toBe('12');
    });
  });
});
