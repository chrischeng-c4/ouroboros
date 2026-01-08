/**
 * RenderController - Bridge between main thread and render worker.
 *
 * Provides the same API as GridRenderer but delegates rendering to a Web Worker
 * using OffscreenCanvas for non-blocking rendering.
 */

import type {
  MainToWorkerMessage,
  WorkerToMainMessage,
  Size,
  CellRange,
  ViewportCellData,
} from './types';
import type { IGridRenderer, Point, CellPosition } from '../types/renderer';
import * as WasmBridge from '../core/WasmBridge';
import { theme } from '../canvas/theme';

export interface RenderControllerOptions {
  onSelectionChanged?: (cell: CellPosition, range?: CellRange) => void;
  onReady?: () => void;
  onError?: (message: string) => void;
}

export class RenderController implements IGridRenderer {
  private worker: Worker;
  private canvas: HTMLCanvasElement;
  private scrollOffset: Point = { x: 0, y: 0 };
  private activeCell: CellPosition = { row: 0, col: 0 };
  private viewportSize: Size = { width: 0, height: 0 };
  private isReady = false;
  private options: RenderControllerOptions;
  // Remote cursors are stored but rendering happens in main thread GridRenderer
  // This is kept for API compatibility - worker rendering doesn't use this yet
  private _remoteCursors: Array<{
    id: string;
    name: string;
    color: string;
    row: number;
    col: number;
  }> = [];

  constructor(canvas: HTMLCanvasElement, options: RenderControllerOptions = {}) {
    this.canvas = canvas;
    this.options = options;

    // Create worker
    this.worker = new Worker(new URL('./render.worker.ts', import.meta.url), {
      type: 'module',
    });

    // Set up message handler
    this.worker.onmessage = this.handleWorkerMessage.bind(this);
    this.worker.onerror = (error) => {
      options.onError?.(`Worker error: ${error.message}`);
    };

    // Initialize
    this.initialize();
  }

  private initialize(): void {
    // Transfer canvas to worker
    const offscreen = this.canvas.transferControlToOffscreen();

    this.viewportSize = {
      width: this.canvas.clientWidth,
      height: this.canvas.clientHeight,
    };

    const message: MainToWorkerMessage = {
      type: 'INIT',
      canvas: offscreen,
      viewportSize: this.viewportSize,
      devicePixelRatio: window.devicePixelRatio || 1,
    };

    this.worker.postMessage(message, [offscreen]);
  }

  private handleWorkerMessage(event: MessageEvent<WorkerToMainMessage>): void {
    const msg = event.data;

    switch (msg.type) {
      case 'READY':
        this.isReady = true;
        this.options.onReady?.();
        break;

      case 'SELECTION_CHANGED':
        this.activeCell = msg.cell;
        this.options.onSelectionChanged?.(msg.cell, msg.range);
        break;

      case 'REQUEST_DATA':
        this.sendViewportData(msg.visibleRange);
        break;

      case 'ERROR':
        this.options.onError?.(msg.message);
        break;
    }
  }

  private sendViewportData(range: CellRange): void {
    // Get cell data from WASM
    const wasmData = WasmBridge.getViewportData(
      range.startRow,
      range.endRow,
      range.startCol,
      range.endCol
    );

    // Convert to worker format
    const cells: ViewportCellData[] = wasmData.map((cell) => ({
      row: cell.row,
      col: cell.col,
      displayValue: cell.display_value,
      numericValue: typeof cell.display_value === 'string' ? parseFloat(cell.display_value) : NaN,
      format: cell.format,
    }));

    // Collect row heights and column widths for visible range
    const rowHeights: Record<number, number> = {};
    const colWidths: Record<number, number> = {};

    for (let r = range.startRow; r <= range.endRow; r++) {
      rowHeights[r] = WasmBridge.getRowHeight(r);
    }

    for (let c = range.startCol; c <= range.endCol; c++) {
      colWidths[c] = WasmBridge.getColWidth(c);
    }

    const message: MainToWorkerMessage = {
      type: 'UPDATE_DATA',
      cells,
      rowHeights,
      colWidths,
    };

    this.worker.postMessage(message);
  }

  // =============================================================================
  // Public API (matches GridRenderer interface)
  // =============================================================================

  public updateViewportSize(): void {
    this.viewportSize = {
      width: this.canvas.clientWidth,
      height: this.canvas.clientHeight,
    };

    if (this.isReady) {
      const message: MainToWorkerMessage = {
        type: 'RESIZE',
        viewportSize: this.viewportSize,
        devicePixelRatio: window.devicePixelRatio || 1,
      };
      this.worker.postMessage(message);
    }
  }

  public setScrollOffset(x: number, y: number): void {
    this.scrollOffset = { x, y };

    if (this.isReady) {
      const message: MainToWorkerMessage = {
        type: 'SCROLL',
        offset: this.scrollOffset,
      };
      this.worker.postMessage(message);
    }
  }

  public setActiveCell(row: number, col: number): void {
    this.activeCell = { row, col };

    if (this.isReady) {
      const message: MainToWorkerMessage = {
        type: 'SELECT',
        cell: this.activeCell,
      };
      this.worker.postMessage(message);
    }
  }

  public getActiveCell(): CellPosition {
    return { ...this.activeCell };
  }

  public setRemoteCursors(cursors: Array<{
    id: string;
    name: string;
    color: string;
    row: number;
    col: number;
  }>): void {
    this._remoteCursors = cursors;
    // Store locally for now - can be forwarded to worker later if needed
    this.render();
  }

  public render(): void {
    if (this.isReady) {
      const message: MainToWorkerMessage = {
        type: 'RENDER',
      };
      this.worker.postMessage(message);
    }
  }

  public resize(width: number, height: number): void {
    // Note: Canvas size is managed by worker now
    this.viewportSize = { width, height };

    if (this.isReady) {
      const message: MainToWorkerMessage = {
        type: 'RESIZE',
        viewportSize: this.viewportSize,
        devicePixelRatio: window.devicePixelRatio || 1,
      };
      this.worker.postMessage(message);
    }
  }

  /**
   * Convert grid coordinates to screen coordinates.
   * Needed for positioning the cell editor overlay.
   */
  public gridToScreen(row: number, col: number): Point {
    let x = theme.headerWidth;
    let y = theme.headerHeight;

    for (let c = 0; c < col; c++) {
      x += WasmBridge.getColWidth(c);
    }
    x -= this.scrollOffset.x;

    for (let r = 0; r < row; r++) {
      y += WasmBridge.getRowHeight(r);
    }
    y -= this.scrollOffset.y;

    return { x, y };
  }

  /**
   * Convert screen coordinates to grid coordinates.
   * Needed for mouse event handling.
   */
  public screenToGrid(x: number, y: number): CellPosition {
    const adjustedX = x - theme.headerWidth + this.scrollOffset.x;
    const adjustedY = y - theme.headerHeight + this.scrollOffset.y;

    let col = 0;
    let accumulatedWidth = 0;
    while (accumulatedWidth < adjustedX && col < 1000) {
      accumulatedWidth += WasmBridge.getColWidth(col);
      if (accumulatedWidth >= adjustedX) break;
      col++;
    }

    let row = 0;
    let accumulatedHeight = 0;
    while (accumulatedHeight < adjustedY && row < 1000) {
      accumulatedHeight += WasmBridge.getRowHeight(row);
      if (accumulatedHeight >= adjustedY) break;
      row++;
    }

    return { row, col };
  }

  /**
   * Check if coordinates are within a filter button area
   * Returns the column index if on a filter button, -1 otherwise
   */
  public isOnFilterButton(screenX: number, screenY: number): number {
    const { headerHeight, headerWidth } = theme;

    // Must be in header row (y < headerHeight)
    if (screenY > headerHeight || screenX < headerWidth) {
      return -1;
    }

    // Find which column
    const { col } = this.screenToGrid(screenX, 0);

    // Get column position
    const colPos = this.gridToScreen(0, col);
    const colWidth = WasmBridge.getColWidth(col);

    // Check if click is in the filter button area (rightmost 20px of header)
    const filterButtonLeft = colPos.x + colWidth - 20;
    if (screenX >= filterButtonLeft && screenX <= colPos.x + colWidth) {
      return col;
    }

    return -1;
  }

  /**
   * Terminate the worker.
   */
  public destroy(): void {
    this.worker.terminate();
  }
}
