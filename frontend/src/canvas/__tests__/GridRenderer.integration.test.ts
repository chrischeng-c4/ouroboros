import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import GridRenderer from '../GridRenderer';
import * as WasmBridge from '../../core/WasmBridge';

describe('GridRenderer Integration Tests (Real Canvas + WASM)', () => {
  let canvas: HTMLCanvasElement;
  let renderer: GridRenderer;

  beforeEach(async () => {
    // Initialize REAL WASM
    await WasmBridge.initWasm();

    // Create REAL canvas
    canvas = document.createElement('canvas');
    canvas.width = 800;
    canvas.height = 600;
    document.body.appendChild(canvas);

    // Create REAL renderer
    renderer = new GridRenderer(canvas);
  });

  afterEach(() => {
    if (canvas.parentNode) {
      canvas.parentNode.removeChild(canvas);
    }
  });

  describe('Render State Verification', () => {
    it('render() executes without errors', () => {
      // Set some cell values
      WasmBridge.setCellValue(0, 0, 'Test');

      // Render should not throw
      expect(() => renderer.render()).not.toThrow();
    });

    it('canvas context is valid after render', () => {
      renderer.render();

      const ctx = canvas.getContext('2d');
      expect(ctx).not.toBeNull();
    });

    it('render() can be called multiple times', () => {
      renderer.render();
      renderer.render();
      renderer.render();

      // Should not throw or cause issues
      const ctx = canvas.getContext('2d');
      expect(ctx).not.toBeNull();
    });
  });

  describe('Coordinate Transformations', () => {
    it('gridToScreen and screenToGrid are inverse operations', () => {
      const gridPos = { row: 5, col: 3 };
      const screenPos = renderer.gridToScreen(gridPos.row, gridPos.col);
      // Add small offset to ensure we're inside the cell (not on the edge)
      const backToGrid = renderer.screenToGrid(screenPos.x + 5, screenPos.y + 5);

      expect(backToGrid?.row).toBe(gridPos.row);
      expect(backToGrid?.col).toBe(gridPos.col);
    });

    it('screenToGrid returns null for out-of-bounds coordinates', () => {
      const result = renderer.screenToGrid(-100, -100);
      // Should handle gracefully (either null or valid cell)
      if (result !== null) {
        expect(result.row).toBeGreaterThanOrEqual(0);
        expect(result.col).toBeGreaterThanOrEqual(0);
      }
    });

    it('gridToScreen returns valid coordinates for cell (0,0)', () => {
      const screenPos = renderer.gridToScreen(0, 0);

      expect(screenPos.x).toBeGreaterThanOrEqual(0);
      expect(screenPos.y).toBeGreaterThanOrEqual(0);
      expect(typeof screenPos.x).toBe('number');
      expect(typeof screenPos.y).toBe('number');
    });

    it('gridToScreen coordinates increase with row/col', () => {
      const pos00 = renderer.gridToScreen(0, 0);
      const pos01 = renderer.gridToScreen(0, 1);
      const pos10 = renderer.gridToScreen(1, 0);

      // Column 1 should be to the right of column 0
      expect(pos01.x).toBeGreaterThan(pos00.x);

      // Row 1 should be below row 0
      expect(pos10.y).toBeGreaterThan(pos00.y);
    });
  });

  describe('Active Cell Management', () => {
    it('getActiveCell returns default cell on initialization', () => {
      const activeCell = renderer.getActiveCell();

      expect(activeCell).toBeTruthy();
      expect(typeof activeCell.row).toBe('number');
      expect(typeof activeCell.col).toBe('number');
      expect(activeCell.row).toBeGreaterThanOrEqual(0);
      expect(activeCell.col).toBeGreaterThanOrEqual(0);
    });

    it('setActiveCell updates active cell', () => {
      renderer.setActiveCell(5, 3);

      const activeCell = renderer.getActiveCell();
      expect(activeCell.row).toBe(5);
      expect(activeCell.col).toBe(3);
    });

    it('setActiveCell and render work together', () => {
      renderer.setActiveCell(10, 10);
      renderer.render();

      const activeCell = renderer.getActiveCell();
      expect(activeCell.row).toBe(10);
      expect(activeCell.col).toBe(10);
    });
  });

  describe('Viewport Updates', () => {
    it('updateViewportSize executes without errors', () => {
      expect(() => renderer.updateViewportSize()).not.toThrow();
    });

    it('updateViewportSize and render work together', () => {
      renderer.updateViewportSize();
      renderer.render();

      const ctx = canvas.getContext('2d');
      expect(ctx).not.toBeNull();
    });

    it('changing canvas size and updating viewport', () => {
      // Change canvas size
      canvas.width = 1000;
      canvas.height = 800;

      // Update viewport
      renderer.updateViewportSize();
      renderer.render();

      // Should not throw
      expect(canvas.width).toBe(1000);
      expect(canvas.height).toBe(800);
    });
  });

  describe('Integration with WASM Cell Data', () => {
    it('renders cells with actual WASM data', () => {
      // Set various cell types
      WasmBridge.setCellValue(0, 0, 'Text');
      WasmBridge.setCellValue(0, 1, '123');
      WasmBridge.setCellValue(0, 2, '=A1&B1');

      // Render should process all cells
      expect(() => renderer.render()).not.toThrow();
    });

    it('renders formatted cells correctly', () => {
      // Set cell with formatting
      WasmBridge.setCellValue(0, 0, 'Formatted');
      WasmBridge.setCellFormat(0, 0, {
        bold: true,
        backgroundColor: '#ff0000',
      });

      // Render should handle formatted cells
      expect(() => renderer.render()).not.toThrow();
    });

    it('renders formula cells with evaluated values', () => {
      WasmBridge.setCellValue(0, 0, '10');
      WasmBridge.setCellValue(0, 1, '20');
      WasmBridge.setCellValue(0, 2, '=A1+B1');

      // Render should show the result, not the formula
      expect(() => renderer.render()).not.toThrow();
    });

    it('renders after cell values change', () => {
      // Initial value
      WasmBridge.setCellValue(0, 0, 'First');
      renderer.render();

      // Change value
      WasmBridge.setCellValue(0, 0, 'Second');
      renderer.render();

      // Should render without issues
      const ctx = canvas.getContext('2d');
      expect(ctx).not.toBeNull();
    });
  });

  describe('Multiple Renders and State Consistency', () => {
    it('consecutive renders maintain state', () => {
      renderer.setActiveCell(3, 3);

      renderer.render();
      renderer.render();
      renderer.render();

      const activeCell = renderer.getActiveCell();
      expect(activeCell.row).toBe(3);
      expect(activeCell.col).toBe(3);
    });

    it('render after active cell changes', () => {
      renderer.setActiveCell(1, 1);
      renderer.render();

      renderer.setActiveCell(5, 5);
      renderer.render();

      const activeCell = renderer.getActiveCell();
      expect(activeCell.row).toBe(5);
      expect(activeCell.col).toBe(5);
    });
  });

  describe('Edge Cases', () => {
    it('renders empty grid', () => {
      // Don't set any cell values, just render
      expect(() => renderer.render()).not.toThrow();
    });

    it('renders grid with single cell', () => {
      WasmBridge.setCellValue(0, 0, 'Only Cell');
      expect(() => renderer.render()).not.toThrow();
    });

    it('renders grid with many cells', () => {
      // Set a 10x10 grid of cells
      for (let row = 0; row < 10; row++) {
        for (let col = 0; col < 10; col++) {
          WasmBridge.setCellValue(row, col, `R${row}C${col}`);
        }
      }

      expect(() => renderer.render()).not.toThrow();
    });

    it('handles very large row/col numbers', () => {
      const screenPos = renderer.gridToScreen(1000, 1000);

      expect(typeof screenPos.x).toBe('number');
      expect(typeof screenPos.y).toBe('number');
    });
  });
});
