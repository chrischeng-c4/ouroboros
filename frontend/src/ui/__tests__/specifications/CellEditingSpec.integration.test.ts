import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as WasmBridge from '../../../core/WasmBridge';
import { createTestEnvironment, cleanupTestEnvironment, simulateTyping, simulateDoubleClick, simulateKeyPress } from '../setup/testUtils';
import type CellEditor from '../../CellEditor';
import type GridRenderer from '../../../canvas/GridRenderer';
import type InputController from '../../InputController';

/**
 * SPECIFICATION TESTS
 *
 * These tests define the REQUIRED BEHAVIOR of the spreadsheet application.
 * They should be written BEFORE implementing features, not after fixing bugs.
 *
 * If any of these tests fail, the application is not meeting its specification.
 */
describe('Cell Editing Specification (Required Behavior)', () => {
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

  describe('SPEC: Basic Cell Editing Requirements', () => {
    it('SPEC: editing a cell and committing MUST update the visual display', () => {
      // SPECIFICATION: When a user edits a cell and commits the edit,
      // the new value MUST be visible in the spreadsheet.

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Hello');
      env.cellEditor.commit();

      // This is the SPEC - after commit, value MUST be in WASM
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Hello');

      // And MUST be visible (editor closed = value persisted)
      expect(env.cellEditor.isActive()).toBe(false);
    });

    it('SPEC: double-clicking a cell MUST activate editing mode', () => {
      // SPECIFICATION: Standard spreadsheet behavior requires
      // double-click to activate cell editing.

      simulateDoubleClick(env.canvas, 10, 10);

      expect(env.cellEditor.isActive()).toBe(true);
    });

    it('SPEC: pressing Enter MUST commit the edit and move down', () => {
      // SPECIFICATION: Enter key commits and moves to cell below.

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Test');
      simulateKeyPress(textarea, 'Enter');

      // Value saved
      expect(WasmBridge.getCellData(0, 0).value).toBe('Test');

      // Moved down
      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(1);
      expect(activeCell.col).toBe(0);
    });

    it('SPEC: pressing Tab MUST commit the edit and move right', () => {
      // SPECIFICATION: Tab key commits and moves to cell on the right.

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Test');
      simulateKeyPress(textarea, 'Tab');

      // Value saved
      expect(WasmBridge.getCellData(0, 0).value).toBe('Test');

      // Moved right
      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(0);
      expect(activeCell.col).toBe(1);
    });

    it('SPEC: pressing Escape MUST cancel the edit without saving', () => {
      // SPECIFICATION: Escape key cancels editing without persisting changes.

      WasmBridge.setCellValue(0, 0, 'Original');

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Changed');
      simulateKeyPress(textarea, 'Escape');

      // Original value preserved
      expect(WasmBridge.getCellData(0, 0).value).toBe('Original');

      // Editor closed
      expect(env.cellEditor.isActive()).toBe(false);
    });
  });

  describe('SPEC: Formula Editing Requirements', () => {
    it('SPEC: entering a formula MUST evaluate and display the result', () => {
      // SPECIFICATION: Formulas starting with "=" must be evaluated.

      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '20');

      env.cellEditor.activate(0, 2);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '=A1+B1');
      env.cellEditor.commit();

      const cellData = WasmBridge.getCellData(0, 2);
      expect(cellData.formula).toBe('=A1+B1');
      expect(cellData.displayValue).toBe('30');
    });

    it('SPEC: editing a formula cell MUST show the formula, not the result', () => {
      // SPECIFICATION: When editing a cell with a formula,
      // the user must see the formula expression, not the computed value.

      WasmBridge.setCellValue(0, 0, '=5+3');

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;

      expect(textarea.value).toBe('=5+3');
      // NOT "8"
    });

    it('SPEC: formula updates MUST propagate to dependent cells', () => {
      // SPECIFICATION: When a cell value changes, all formulas
      // that reference it must recalculate.

      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '=A1*2');

      let result = WasmBridge.getCellData(0, 1);
      expect(result.displayValue).toBe('20');

      // Update source cell
      WasmBridge.setCellValue(0, 0, '5');

      // Formula must recalculate
      result = WasmBridge.getCellData(0, 1);
      expect(result.displayValue).toBe('10');
    });
  });

  describe('SPEC: Data Persistence Requirements', () => {
    it('SPEC: cell values MUST persist after editing multiple cells', () => {
      // SPECIFICATION: All edited cell values must remain in memory.

      env.cellEditor.activate(0, 0);
      let textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'First');
      simulateKeyPress(textarea, 'Enter');

      // Need to activate after Enter to continue editing
      env.cellEditor.activate(1, 0);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Second');
      simulateKeyPress(textarea, 'Enter');

      // Need to activate after Enter to continue editing
      env.cellEditor.activate(2, 0);
      textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Third');
      simulateKeyPress(textarea, 'Enter');

      // All values must persist
      expect(WasmBridge.getCellData(0, 0).value).toBe('First');
      expect(WasmBridge.getCellData(1, 0).value).toBe('Second');
      expect(WasmBridge.getCellData(2, 0).value).toBe('Third');
    });

    it('SPEC: re-editing a cell MUST show the previously saved value', () => {
      // SPECIFICATION: Opening a cell for editing must display
      // the current stored value.

      env.cellEditor.activate(0, 0);
      let textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Saved Value');
      env.cellEditor.commit();

      // Re-edit
      env.cellEditor.activate(0, 0);
      textarea = env.container.querySelector('textarea')!;

      expect(textarea.value).toBe('Saved Value');
    });
  });

  describe('SPEC: Input Preservation Requirements (Bug #1-3)', () => {
    it('SPEC: entering "10" MUST preserve "10" exactly, not corrupt it', () => {
      // SPECIFICATION: Original user input must be preserved exactly
      // as entered, without corruption.

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '10');
      env.cellEditor.commit();

      // Re-edit to verify original input preserved
      env.cellEditor.activate(0, 0);
      const textareaReopen = env.container.querySelector('textarea')!;

      expect(textareaReopen.value).toBe('10');
      // NOT "B2C2" or any corruption
    });

    it('SPEC: entering text MUST preserve text exactly', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Hello World');
      env.cellEditor.commit();

      // Verify
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Hello World');
    });

    it('SPEC: entering percentage MUST preserve format', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, '50%');
      env.cellEditor.commit();

      // Verify
      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('50%');
    });
  });

  describe('SPEC: UI Synchronization Requirements', () => {
    it('SPEC: typing in cell editor MUST sync with formula bar', () => {
      // SPECIFICATION: Cell editor and formula bar must stay in sync.

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      simulateTyping(textarea, 'Synced Text');

      expect(env.formulaBar.value).toBe('Synced Text');
    });

    it('SPEC: typing in formula bar MUST sync with cell editor', () => {
      // SPECIFICATION: Formula bar and cell editor must stay in sync.

      env.cellEditor.activate(0, 0);

      env.formulaBar.value = 'From Formula Bar';
      env.formulaBar.dispatchEvent(new Event('input'));

      const textarea = env.container.querySelector('textarea')!;
      expect(textarea.value).toBe('From Formula Bar');
    });
  });

  describe('SPEC: Visual Feedback Requirements', () => {
    it('SPEC: activating cell editor MUST show the textarea', () => {
      // SPECIFICATION: When editing starts, the textarea must be visible.

      env.cellEditor.activate(0, 0);

      const textarea = env.container.querySelector('textarea')!;
      expect(textarea.style.display).toBe('block');
    });

    it('SPEC: committing edit MUST hide the textarea', () => {
      // SPECIFICATION: When editing finishes, the textarea must be hidden.

      env.cellEditor.activate(0, 0);
      env.cellEditor.commit();

      const textarea = env.container.querySelector('textarea')!;
      expect(textarea.style.display).toBe('none');
    });

    it('SPEC: isActive() MUST reflect current editing state', () => {
      // SPECIFICATION: The API must accurately report whether editing is active.

      expect(env.cellEditor.isActive()).toBe(false);

      env.cellEditor.activate(0, 0);
      expect(env.cellEditor.isActive()).toBe(true);

      env.cellEditor.commit();
      expect(env.cellEditor.isActive()).toBe(false);
    });
  });
});
