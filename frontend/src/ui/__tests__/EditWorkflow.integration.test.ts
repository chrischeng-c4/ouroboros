import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as WasmBridge from '../../core/WasmBridge';
import { createTestEnvironment, cleanupTestEnvironment, simulateDoubleClick, simulateTyping, simulateKeyPress } from './setup/testUtils';
import type CellEditor from '../CellEditor';
import type GridRenderer from '../../canvas/GridRenderer';
import type InputController from '../InputController';

describe('Complete Edit Workflow Integration Tests', () => {
  let env: {
    canvas: HTMLCanvasElement;
    renderer: GridRenderer;
    container: HTMLElement;
    formulaBar: HTMLInputElement;
    cellEditor: CellEditor;
    inputController: InputController;
  };

  beforeEach(async () => {
    env = await createTestEnvironment();
  });

  afterEach(() => {
    cleanupTestEnvironment(env);
  });

  describe('Full User Workflows', () => {
    it('WORKFLOW: activate → type → Enter → value persists and displays', () => {
      // Activate editor at (0, 0)
      env.cellEditor.activate(0, 0);

      // Should activate cell editor
      expect(env.cellEditor.isActive()).toBe(true);

      // Type value
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Test Value');

      // Press Enter
      simulateKeyPress(textarea, 'Enter');

      // Verify workflow completed correctly
      expect(env.cellEditor.isActive()).toBe(false);

      // Verify persisted to WASM
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData?.value).toBe('Test Value');

      // Verify active cell moved down
      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(1);
      expect(activeCell.col).toBe(0);
    });

    it('WORKFLOW: click cell → type in formula bar → Tab → next cell', () => {
      // Click cell to select (simulated by activating directly)
      env.renderer.setActiveCell(1, 2);

      // Focus formula bar
      env.formulaBar.focus();

      // Should activate cell editor when formula bar gets focus
      expect(env.cellEditor.isActive()).toBe(true);

      // Type in formula bar
      env.formulaBar.value = 'Formula Bar Value';
      env.formulaBar.dispatchEvent(new Event('input'));

      // Press Tab
      simulateKeyPress(env.formulaBar, 'Tab');

      // Verify value saved
      const cellData = WasmBridge.getCellData(1, 2);
      expect(cellData.value).toBe('Formula Bar Value');

      // Verify moved to next column
      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(1);
      expect(activeCell.col).toBe(3);
    });

    it('WORKFLOW: edit → Tab 3 times → verify all values saved', () => {
      // Edit cell (0, 0)
      env.cellEditor.activate(0, 0);
      let textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Value 1');
      simulateKeyPress(textarea, 'Tab');

      // Now at (0, 1) - need to activate to continue editing
      expect(env.renderer.getActiveCell().col).toBe(1);
      env.cellEditor.activate(0, 1);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Value 2');
      simulateKeyPress(textarea, 'Tab');

      // Now at (0, 2) - need to activate to continue editing
      expect(env.renderer.getActiveCell().col).toBe(2);
      env.cellEditor.activate(0, 2);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Value 3');
      simulateKeyPress(textarea, 'Tab');

      // Verify all values persisted
      expect(WasmBridge.getCellData(0, 0).value).toBe('Value 1');
      expect(WasmBridge.getCellData(0, 1).value).toBe('Value 2');
      expect(WasmBridge.getCellData(0, 2).value).toBe('Value 3');
    });

    it('WORKFLOW: edit → Escape → value not saved', () => {
      // Set initial value
      WasmBridge.setCellValue(0, 0, 'Original');

      // Edit and cancel
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Changed Value');
      simulateKeyPress(textarea, 'Escape');

      // Verify original value preserved
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Original');

      // Verify editor closed
      expect(env.cellEditor.isActive()).toBe(false);
    });
  });

  describe('Formula Workflows', () => {
    it('WORKFLOW: enter formula referencing other cells', () => {
      // Set up source cells
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(1, 0, '20');

      // Enter formula
      env.cellEditor.activate(2, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '=A1+A2');
      simulateKeyPress(textarea, 'Enter');

      // Verify formula stored and evaluated
      const cellData = WasmBridge.getCellData(2, 0);
      expect(cellData.formula).toBe('=A1+A2');
      expect(cellData.displayValue).toBe('30');
    });

    it('WORKFLOW: edit existing formula', () => {
      // Create initial formula
      WasmBridge.setCellValue(0, 0, '=5+3');

      // Edit it
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;

      // Should show formula expression
      expect(textarea.value).toBe('=5+3');

      // Modify formula
      textarea.value = '=5+10';
      simulateKeyPress(textarea, 'Enter');

      // Verify updated
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.formula).toBe('=5+10');
      expect(cellData.displayValue).toBe('15');
    });

    it('WORKFLOW: enter SUM formula', () => {
      // Set up range
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(1, 0, '20');
      WasmBridge.setCellValue(2, 0, '30');

      // Enter SUM formula
      env.cellEditor.activate(3, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '=SUM(A1:A3)');
      simulateKeyPress(textarea, 'Enter');

      // Verify result
      const cellData = WasmBridge.getCellData(3, 0);
      expect(cellData.formula).toBe('=SUM(A1:A3)');
      expect(cellData.displayValue).toBe('60');
    });
  });

  describe('Multi-Cell Editing Workflows', () => {
    it('WORKFLOW: create table of data with Enter navigation', () => {
      // Row 1
      env.cellEditor.activate(0, 0);
      let textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Product');
      simulateKeyPress(textarea, 'Enter');

      // Row 2 - need to activate to continue editing
      env.cellEditor.activate(1, 0);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Apples');
      simulateKeyPress(textarea, 'Enter');

      // Row 3 - need to activate to continue editing
      env.cellEditor.activate(2, 0);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Oranges');
      simulateKeyPress(textarea, 'Enter');

      // Verify all values
      expect(WasmBridge.getCellData(0, 0).value).toBe('Product');
      expect(WasmBridge.getCellData(1, 0).value).toBe('Apples');
      expect(WasmBridge.getCellData(2, 0).value).toBe('Oranges');

      // Verify ended at row 3
      expect(env.renderer.getActiveCell().row).toBe(3);
    });

    it('WORKFLOW: create row of data with Tab navigation', () => {
      // Column A
      env.cellEditor.activate(0, 0);
      let textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Name');
      simulateKeyPress(textarea, 'Tab');

      // Column B - need to activate to continue editing
      env.cellEditor.activate(0, 1);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Age');
      simulateKeyPress(textarea, 'Tab');

      // Column C - need to activate to continue editing
      env.cellEditor.activate(0, 2);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'City');
      simulateKeyPress(textarea, 'Tab');

      // Verify all values
      expect(WasmBridge.getCellData(0, 0).value).toBe('Name');
      expect(WasmBridge.getCellData(0, 1).value).toBe('Age');
      expect(WasmBridge.getCellData(0, 2).value).toBe('City');

      // Verify ended at column 3
      expect(env.renderer.getActiveCell().col).toBe(3);
    });
  });

  describe('Complex Interaction Workflows', () => {
    it('WORKFLOW: mix of Enter, Tab, and formula editing', () => {
      // Setup: Create a simple calculation table

      // Header row (Tab navigation)
      env.cellEditor.activate(0, 0);
      let textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Price');
      simulateKeyPress(textarea, 'Tab');

      // Need to activate after Tab to continue editing
      env.cellEditor.activate(0, 1);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Quantity');
      simulateKeyPress(textarea, 'Tab');

      // Need to activate after Tab to continue editing
      env.cellEditor.activate(0, 2);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Total');
      simulateKeyPress(textarea, 'Enter');

      // Data row (mixed navigation)
      // Move to (1, 0) manually
      env.cellEditor.activate(1, 0);

      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '10');
      simulateKeyPress(textarea, 'Tab');

      // Need to activate after Tab to continue editing
      env.cellEditor.activate(1, 1);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '5');
      simulateKeyPress(textarea, 'Tab');

      // Formula for total - need to activate after Tab
      env.cellEditor.activate(1, 2);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '=A2*B2');
      simulateKeyPress(textarea, 'Enter');

      // Verify all values
      expect(WasmBridge.getCellData(0, 0).value).toBe('Price');
      expect(WasmBridge.getCellData(0, 1).value).toBe('Quantity');
      expect(WasmBridge.getCellData(0, 2).value).toBe('Total');
      expect(WasmBridge.getCellData(1, 0).value).toBe('10');
      expect(WasmBridge.getCellData(1, 1).value).toBe('5');

      const totalCell = WasmBridge.getCellData(1, 2);
      expect(totalCell?.formula).toBe('=A2*B2');
      expect(totalCell?.displayValue).toBe('50');
    });
  });
});
