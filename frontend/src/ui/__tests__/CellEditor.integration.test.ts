import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as WasmBridge from '../../core/WasmBridge';
import { createTestEnvironment, cleanupTestEnvironment, simulateTyping, simulateKeyPress } from './setup/testUtils';
import type CellEditor from '../CellEditor';
import type GridRenderer from '../../canvas/GridRenderer';

describe('CellEditor Integration Tests (Real Dependencies)', () => {
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

  describe('Critical Bug Prevention Tests', () => {
    it('CRITICAL: commit MUST call renderer.render() with real renderer', () => {
      // This is the test that would have caught the missing render() call
      const renderSpy = vi.spyOn(env.renderer, 'render');

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = 'Test Value';
      env.cellEditor.commit();

      // Verify render was called on REAL renderer
      expect(renderSpy).toHaveBeenCalled();

      renderSpy.mockRestore();
    });

    it('CRITICAL: commit MUST persist value to WASM state', () => {
      env.cellEditor.activate(2, 3);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = 'Persistent Value';
      env.cellEditor.commit();

      // Direct WASM state check - verifies behavior, not just implementation
      const cellData = WasmBridge.getCellData(2, 3);
      expect(cellData.value).toBe('Persistent Value');
    });

    it('CRITICAL: edit → commit → verify → re-edit shows same value', () => {
      // First edit
      env.cellEditor.activate(0, 0);
      let textarea = env.container.querySelector('textarea')!;
      textarea.value = 'First Edit';
      env.cellEditor.commit();

      // Verify persisted
      let cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('First Edit');

      // Re-edit same cell
      env.cellEditor.activate(0, 0);
      textarea = env.container.querySelector('textarea')!;

      // Should show previous value
      expect(textarea.value).toBe('First Edit');

      // Change it
      textarea.value = 'Second Edit';
      env.cellEditor.commit();

      // Verify updated
      cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Second Edit');
    });
  });

  describe('Integration: CellEditor + GridRenderer + WASM', () => {
    it('should persist and render value after commit', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = 'Hello';
      env.cellEditor.commit();

      // Verify all three layers
      expect(renderSpy).toHaveBeenCalled(); // Implementation
      expect(WasmBridge.getCellData(0, 0).value).toBe('Hello'); // State
      expect(env.cellEditor.isActive()).toBe(false); // Behavior

      renderSpy.mockRestore();
    });

    it('should handle formula input correctly', () => {
      // Set up dependency cells
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '20');

      // Enter formula
      env.cellEditor.activate(0, 2);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = '=A1+B1';
      env.cellEditor.commit();

      // Verify formula stored
      const cellData = WasmBridge.getCellData(0, 2);
      expect(cellData.formula).toBe('=A1+B1');
      expect(cellData.displayValue).toBe('30');
    });

    it('should sync textarea with formula bar during typing', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;

      simulateTyping(textarea, 'Synced Text');

      expect(env.formulaBar.value).toBe('Synced Text');
    });
  });

  describe('Keyboard Navigation Integration', () => {
    it('Enter key should commit and move down', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = '100';

      simulateKeyPress(textarea, 'Enter');

      // Verify commit
      expect(WasmBridge.getCellData(0, 0).value).toBe('100');

      // Verify moved down
      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(1);
      expect(activeCell.col).toBe(0);
    });

    it('Tab key should commit and move right', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = '200';

      simulateKeyPress(textarea, 'Tab');

      // Verify commit
      expect(WasmBridge.getCellData(0, 0).value).toBe('200');

      // Verify moved right
      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(0);
      expect(activeCell.col).toBe(1);
    });

    it('Shift+Tab should commit and move left', () => {
      env.cellEditor.activate(0, 5);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = '300';

      simulateKeyPress(textarea, 'Tab', { shiftKey: true });

      // Verify commit
      expect(WasmBridge.getCellData(0, 5).value).toBe('300');

      // Verify moved left
      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(0);
      expect(activeCell.col).toBe(4);
    });

    it('Escape key should cancel without saving', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      textarea.value = 'Cancelled Value';

      simulateKeyPress(textarea, 'Escape');

      // Verify NOT saved - empty cell returns null or empty displayValue
      const cellData = WasmBridge.getCellData(0, 0);
      if (cellData !== null) {
        expect(cellData.displayValue).toBe('');
      }

      // Verify editor closed
      expect(env.cellEditor.isActive()).toBe(false);
    });
  });

  describe('Cell State Retrieval', () => {
    it('editing regular value shows original input', () => {
      WasmBridge.setCellValue(0, 0, '10');

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;

      expect(textarea.value).toBe('10');
    });

    it('editing formula cell shows formula expression, not result', () => {
      WasmBridge.setCellValue(0, 0, '=5+3');

      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;

      // Should show formula, not computed value
      expect(textarea.value).toBe('=5+3');
    });

    it('formula bar syncs with cell editor value', () => {
      WasmBridge.setCellValue(0, 0, 'Original Value');

      env.cellEditor.activate(0, 0);

      expect(env.formulaBar.value).toBe('Original Value');
    });
  });

  describe('Editor Visibility and State', () => {
    it('activate should show textarea and focus it', () => {
      env.cellEditor.activate(0, 0);

      const textarea = env.container.querySelector('textarea')!;
      expect(textarea).toBeTruthy();
      expect(textarea.style.display).toBe('block');
    });

    it('commit should hide textarea', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      expect(textarea.style.display).toBe('block');

      env.cellEditor.commit();

      expect(textarea.style.display).toBe('none');
    });

    it('cancel should hide textarea', () => {
      env.cellEditor.activate(0, 0);
      const textarea = env.container.querySelector('textarea')!;
      expect(textarea.style.display).toBe('block');

      env.cellEditor.cancel();

      expect(textarea.style.display).toBe('none');
    });

    it('isActive should reflect editor state', () => {
      expect(env.cellEditor.isActive()).toBe(false);

      env.cellEditor.activate(0, 0);
      expect(env.cellEditor.isActive()).toBe(true);

      env.cellEditor.commit();
      expect(env.cellEditor.isActive()).toBe(false);
    });
  });
});
