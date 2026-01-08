/**
 * Worker message types for offscreen canvas rendering.
 *
 * Communication flow:
 * Main Thread → Worker: INIT, SCROLL, RESIZE, SELECT, UPDATE_DATA
 * Worker → Main Thread: READY, SELECTION_CHANGED, REQUEST_DATA
 */

import type { CellFormat } from '../types';

// =============================================================================
// Shared Types
// =============================================================================

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface CellPosition {
  row: number;
  col: number;
}

export interface CellRange {
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
}

export interface ViewportCellData {
  row: number;
  col: number;
  displayValue: string;
  numericValue: number; // NaN for non-numeric
  format: CellFormat;
}

// =============================================================================
// Main Thread → Worker Messages
// =============================================================================

export interface InitMessage {
  type: 'INIT';
  canvas: OffscreenCanvas;
  viewportSize: Size;
  devicePixelRatio: number;
}

export interface ScrollMessage {
  type: 'SCROLL';
  offset: Point;
}

export interface ResizeMessage {
  type: 'RESIZE';
  viewportSize: Size;
  devicePixelRatio: number;
}

export interface SelectMessage {
  type: 'SELECT';
  cell: CellPosition;
  range?: CellRange;
}

export interface UpdateDataMessage {
  type: 'UPDATE_DATA';
  cells: ViewportCellData[];
  rowHeights: Map<number, number> | Record<number, number>;
  colWidths: Map<number, number> | Record<number, number>;
}

export interface RenderMessage {
  type: 'RENDER';
}

export interface SetDimensionMessage {
  type: 'SET_DIMENSION';
  dimension: 'row' | 'col';
  index: number;
  size: number;
}

export type MainToWorkerMessage =
  | InitMessage
  | ScrollMessage
  | ResizeMessage
  | SelectMessage
  | UpdateDataMessage
  | RenderMessage
  | SetDimensionMessage;

// =============================================================================
// Worker → Main Thread Messages
// =============================================================================

export interface ReadyMessage {
  type: 'READY';
}

export interface SelectionChangedMessage {
  type: 'SELECTION_CHANGED';
  cell: CellPosition;
  range?: CellRange;
}

export interface RequestDataMessage {
  type: 'REQUEST_DATA';
  visibleRange: CellRange;
}

export interface ErrorMessage {
  type: 'ERROR';
  message: string;
  stack?: string;
}

export type WorkerToMainMessage =
  | ReadyMessage
  | SelectionChangedMessage
  | RequestDataMessage
  | ErrorMessage;
