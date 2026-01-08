import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as WasmBridge from '../../core/WasmBridge';
import { createTestEnvironment, cleanupTestEnvironment, simulateKeyPress, simulateDoubleClick } from './setup/testUtils';
import type GridRenderer from '../../canvas/GridRenderer';
import type InputController from '../InputController';
import type CellEditor from '../CellEditor';

describe('InputController Integration Tests (Real Dependencies)', () => {
  let env: {
    canvas: HTMLCanvasElement;
    renderer: GridRenderer;
    container: HTMLElement;
    formulaBar: HTMLInputElement;
    inputController: InputController;
    cellEditor: CellEditor;
  };

  beforeEach(async () => {
    env = await createTestEnvironment();
  });

  afterEach(() => {
    env.inputController.cleanup();
    cleanupTestEnvironment(env);
  });

  describe('Keyboard Navigation Tests', () => {
    it('Arrow Down should move active cell down', () => {
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowDown');

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(6);
      expect(activeCell.col).toBe(5);
    });

    it('Arrow Up should move active cell up', () => {
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowUp');

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(4);
      expect(activeCell.col).toBe(5);
    });

    it('Arrow Right should move active cell right', () => {
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowRight');

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(5);
      expect(activeCell.col).toBe(6);
    });

    it('Arrow Left should move active cell left', () => {
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowLeft');

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(5);
      expect(activeCell.col).toBe(4);
    });

    it('Arrow Up should stop at row boundary (0)', () => {
      env.renderer.setActiveCell(0, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowUp');

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(0);
      expect(activeCell.col).toBe(5);
    });

    it('Arrow Left should stop at column boundary (0)', () => {
      env.renderer.setActiveCell(5, 0);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowLeft');

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(5);
      expect(activeCell.col).toBe(0);
    });

    it('Tab should move active cell right', () => {
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'Tab');

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(5);
      expect(activeCell.col).toBe(6);
    });

    it('Shift+Tab should move active cell left', () => {
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'Tab', { shiftKey: true });

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell.row).toBe(5);
      expect(activeCell.col).toBe(4);
    });

    it('CRITICAL: Arrow keys should call renderer.render()', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowDown');

      expect(renderSpy).toHaveBeenCalled();
      renderSpy.mockRestore();
    });
  });

  describe('Cell Clearing Tests', () => {
    it('Delete key should clear cell value', () => {
      WasmBridge.setCellValue(5, 5, 'Test Value');
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'Delete');

      const cellData = WasmBridge.getCellData(5, 5);
      expect(cellData.value).toBe('');
    });

    it('Backspace key should clear cell value', () => {
      WasmBridge.setCellValue(7, 8, 'Another Value');
      env.renderer.setActiveCell(7, 8);

      simulateKeyPress(document as unknown as HTMLElement, 'Backspace');

      const cellData = WasmBridge.getCellData(7, 8);
      expect(cellData.value).toBe('');
    });

    it('Delete should clear formula cell', () => {
      WasmBridge.setCellValue(2, 2, '=5+5');
      env.renderer.setActiveCell(2, 2);

      simulateKeyPress(document as unknown as HTMLElement, 'Delete');

      const cellData = WasmBridge.getCellData(2, 2);
      expect(cellData.value).toBe('');
    });

    it('CRITICAL: Delete should call renderer.render()', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');
      WasmBridge.setCellValue(5, 5, 'Value');
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'Delete');

      expect(renderSpy).toHaveBeenCalled();
      renderSpy.mockRestore();
    });
  });

  describe('Undo/Redo Tests', () => {
    it('Ctrl+Z should trigger undo when canUndo is true', () => {
      WasmBridge.setCellValue(0, 0, 'Original');
      WasmBridge.setCellValue(0, 0, 'Modified');

      expect(WasmBridge.canUndo()).toBe(true);

      simulateKeyPress(document as unknown as HTMLElement, 'z', { ctrlKey: true });

      const cellData = WasmBridge.getCellData(0, 0);
      expect(cellData.value).toBe('Original');
    });

    it('Cmd+Z should trigger undo on Mac (metaKey)', () => {
      WasmBridge.setCellValue(1, 1, 'Original');
      WasmBridge.setCellValue(1, 1, 'Modified');

      expect(WasmBridge.canUndo()).toBe(true);

      simulateKeyPress(document as unknown as HTMLElement, 'z', { metaKey: true });

      const cellData = WasmBridge.getCellData(1, 1);
      expect(cellData.value).toBe('Original');
    });

    it('Ctrl+Y should trigger redo', () => {
      WasmBridge.setCellValue(2, 2, 'Value');
      WasmBridge.undo();

      expect(WasmBridge.canRedo()).toBe(true);

      simulateKeyPress(document as unknown as HTMLElement, 'y', { ctrlKey: true });

      const cellData = WasmBridge.getCellData(2, 2);
      expect(cellData.value).toBe('Value');
    });

    it('Ctrl+Shift+Z should trigger redo', () => {
      WasmBridge.setCellValue(3, 3, 'Value');
      WasmBridge.undo();

      expect(WasmBridge.canRedo()).toBe(true);

      simulateKeyPress(document as unknown as HTMLElement, 'z', { ctrlKey: true, shiftKey: true });

      const cellData = WasmBridge.getCellData(3, 3);
      expect(cellData.value).toBe('Value');
    });
  });

  describe('Mouse Interaction Tests', () => {
    it('Click should select cell at correct position', () => {
      const rect = env.canvas.getBoundingClientRect();

      const event = new MouseEvent('mousedown', {
        clientX: rect.left + 100,
        clientY: rect.top + 100,
        bubbles: true,
        cancelable: true,
      });
      env.canvas.dispatchEvent(event);

      const activeCell = env.renderer.getActiveCell();
      expect(activeCell).toBeDefined();
      expect(activeCell.row).toBeGreaterThanOrEqual(0);
      expect(activeCell.col).toBeGreaterThanOrEqual(0);
    });

    it('CRITICAL: Click should call renderer.render()', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');
      const rect = env.canvas.getBoundingClientRect();

      const event = new MouseEvent('mousedown', {
        clientX: rect.left + 100,
        clientY: rect.top + 100,
        bubbles: true,
        cancelable: true,
      });
      env.canvas.dispatchEvent(event);

      expect(renderSpy).toHaveBeenCalled();
      renderSpy.mockRestore();
    });

    it('Double-click should trigger edit mode callback', () => {
      const rect = env.canvas.getBoundingClientRect();

      simulateDoubleClick(env.canvas, rect.left + 150, rect.top + 150);

      expect(env.cellEditor.isActive()).toBe(true);
    });
  });

  describe('Wheel Scroll Tests', () => {
    it('Wheel scroll down should update scroll offset', () => {
      const initialPos = env.renderer.gridToScreen(5, 5);

      const wheelEvent = new WheelEvent('wheel', {
        deltaX: 0,
        deltaY: 100,
        bubbles: true,
        cancelable: true,
      });
      env.canvas.dispatchEvent(wheelEvent);

      const newPos = env.renderer.gridToScreen(5, 5);
      expect(newPos.y).not.toBe(initialPos.y);
    });

    it('CRITICAL: Wheel scroll should call renderer.render()', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');

      const wheelEvent = new WheelEvent('wheel', {
        deltaX: 0,
        deltaY: 50,
        bubbles: true,
        cancelable: true,
      });
      env.canvas.dispatchEvent(wheelEvent);

      expect(renderSpy).toHaveBeenCalled();
      renderSpy.mockRestore();
    });
  });

  describe('Cleanup Tests', () => {
    it('cleanup() should remove keyboard event listener', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');

      env.inputController.cleanup();

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowDown');

      expect(renderSpy).not.toHaveBeenCalled();
      renderSpy.mockRestore();
    });

    it('cleanup() should remove mousedown event listener', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');

      env.inputController.cleanup();

      const rect = env.canvas.getBoundingClientRect();
      const event = new MouseEvent('mousedown', {
        clientX: rect.left + 100,
        clientY: rect.top + 100,
        bubbles: true,
        cancelable: true,
      });
      env.canvas.dispatchEvent(event);

      expect(renderSpy).not.toHaveBeenCalled();
      renderSpy.mockRestore();
    });

    it('cleanup() should allow multiple calls without error', () => {
      expect(() => {
        env.inputController.cleanup();
        env.inputController.cleanup();
        env.inputController.cleanup();
      }).not.toThrow();
    });
  });

  describe('Integration: InputController + GridRenderer + WASM', () => {
    it('should update cell and persist to WASM after Delete key', () => {
      WasmBridge.setCellValue(5, 5, 'To Delete');
      env.renderer.setActiveCell(5, 5);

      simulateKeyPress(document as unknown as HTMLElement, 'Delete');

      const cellData = WasmBridge.getCellData(5, 5);
      expect(cellData.value).toBe('');
    });

    it('should navigate and preserve active cell across operations', () => {
      env.renderer.setActiveCell(0, 0);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowRight');
      expect(env.renderer.getActiveCell().col).toBe(1);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowDown');
      expect(env.renderer.getActiveCell().row).toBe(1);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowLeft');
      expect(env.renderer.getActiveCell().col).toBe(0);

      simulateKeyPress(document as unknown as HTMLElement, 'ArrowUp');
      expect(env.renderer.getActiveCell().row).toBe(0);
    });

    it('should handle Tab navigation across multiple cells', () => {
      env.renderer.setActiveCell(0, 0);

      for (let i = 0; i < 5; i++) {
        simulateKeyPress(document as unknown as HTMLElement, 'Tab');
      }

      expect(env.renderer.getActiveCell().col).toBe(5);

      for (let i = 0; i < 3; i++) {
        simulateKeyPress(document as unknown as HTMLElement, 'Tab', { shiftKey: true });
      }

      expect(env.renderer.getActiveCell().col).toBe(2);
    });
  });

  describe('Edge Cases and Error Handling', () => {
    it('should handle rapid arrow key presses', () => {
      env.renderer.setActiveCell(10, 10);

      for (let i = 0; i < 20; i++) {
        simulateKeyPress(document as unknown as HTMLElement, 'ArrowDown');
      }

      expect(env.renderer.getActiveCell().row).toBe(30);

      for (let i = 0; i < 20; i++) {
        simulateKeyPress(document as unknown as HTMLElement, 'ArrowUp');
      }

      expect(env.renderer.getActiveCell().row).toBe(10);
    });

    it('should handle Delete on empty cell gracefully', () => {
      env.renderer.setActiveCell(100, 100);

      expect(() => {
        simulateKeyPress(document as unknown as HTMLElement, 'Delete');
      }).not.toThrow();

      const cellData = WasmBridge.getCellData(100, 100);
      expect(cellData.value).toBe('');
    });

    it('should not interfere with non-handled keys', () => {
      const renderSpy = vi.spyOn(env.renderer, 'render');
      renderSpy.mockClear();

      simulateKeyPress(document as unknown as HTMLElement, 'a');
      simulateKeyPress(document as unknown as HTMLElement, 'b');
      simulateKeyPress(document as unknown as HTMLElement, '1');

      expect(renderSpy).not.toHaveBeenCalled();
      renderSpy.mockRestore();
    });
  });
});
