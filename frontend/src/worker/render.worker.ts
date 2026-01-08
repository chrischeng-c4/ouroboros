/**
 * Offscreen Canvas Render Worker
 *
 * Handles all canvas rendering in a separate thread to keep the main thread responsive.
 * Receives data and render commands from the main thread, renders to OffscreenCanvas.
 */

import type {
  MainToWorkerMessage,
  WorkerToMainMessage,
  Point,
  Size,
  CellPosition,
  CellRange,
  ViewportCellData,
} from './types';

// =============================================================================
// Theme (copied from canvas/theme.ts to avoid import issues in worker)
// =============================================================================

const theme = {
  // Grid
  backgroundColor: '#ffffff',
  gridLineColor: '#e0e0e0',
  gridLineWidth: 1,

  // Headers
  headerBackground: '#f5f5f5',
  headerText: '#333333',
  headerFont: '500 12px -apple-system, BlinkMacSystemFont, sans-serif',
  headerHeight: 24,
  headerWidth: 50,

  // Cells
  cellFont: '13px -apple-system, BlinkMacSystemFont, sans-serif',
  cellTextColor: '#000000',
  cellPadding: 4,
  defaultRowHeight: 21,
  defaultColWidth: 100,

  // Selection
  selectionBackground: 'rgba(26, 115, 232, 0.1)',
  selectionBorder: '#1a73e8',
  selectionBorderWidth: 2,
  activeCellBorder: '#1a73e8',
  activeCellBorderWidth: 2,
};

// =============================================================================
// Worker State
// =============================================================================

let canvas: OffscreenCanvas | null = null;
let ctx: OffscreenCanvasRenderingContext2D | null = null;
let viewportSize: Size = { width: 0, height: 0 };
let scrollOffset: Point = { x: 0, y: 0 };
let activeCell: CellPosition = { row: 0, col: 0 };
let devicePixelRatio = 1;

// Cell data cache
let cellsData: ViewportCellData[] = [];
let rowHeights: Map<number, number> = new Map();
let colWidths: Map<number, number> = new Map();

// Render loop
let animationFrameId: number | null = null;
let needsRender = false;

// =============================================================================
// Helper Functions
// =============================================================================

function postMessage(message: WorkerToMainMessage): void {
  self.postMessage(message);
}

function getRowHeight(row: number): number {
  return rowHeights.get(row) ?? theme.defaultRowHeight;
}

function getColWidth(col: number): number {
  return colWidths.get(col) ?? theme.defaultColWidth;
}

function colToLetter(col: number): string {
  let result = '';
  let num = col;
  while (num >= 0) {
    result = String.fromCharCode(65 + (num % 26)) + result;
    num = Math.floor(num / 26) - 1;
    if (num < 0) break;
  }
  return result;
}

function gridToScreen(row: number, col: number): Point {
  let x = theme.headerWidth;
  let y = theme.headerHeight;

  for (let c = 0; c < col; c++) {
    x += getColWidth(c);
  }
  x -= scrollOffset.x;

  for (let r = 0; r < row; r++) {
    y += getRowHeight(r);
  }
  y -= scrollOffset.y;

  return { x, y };
}

function screenToGrid(x: number, y: number): CellPosition {
  const adjustedX = x - theme.headerWidth + scrollOffset.x;
  const adjustedY = y - theme.headerHeight + scrollOffset.y;

  let col = 0;
  let accumulatedWidth = 0;
  while (accumulatedWidth < adjustedX && col < 1000) {
    accumulatedWidth += getColWidth(col);
    if (accumulatedWidth >= adjustedX) break;
    col++;
  }

  let row = 0;
  let accumulatedHeight = 0;
  while (accumulatedHeight < adjustedY && row < 1000) {
    accumulatedHeight += getRowHeight(row);
    if (accumulatedHeight >= adjustedY) break;
    row++;
  }

  return { row, col };
}

function calculateVisibleRange(): CellRange {
  const startPos = screenToGrid(theme.headerWidth, theme.headerHeight);
  const endPos = screenToGrid(viewportSize.width, viewportSize.height);

  return {
    startRow: Math.max(0, startPos.row),
    endRow: Math.min(999, endPos.row + 1),
    startCol: Math.max(0, startPos.col),
    endCol: Math.min(999, endPos.col + 1),
  };
}

// =============================================================================
// Rendering Functions
// =============================================================================

function renderGrid(range: CellRange): void {
  if (!ctx) return;

  ctx.strokeStyle = theme.gridLineColor;
  ctx.lineWidth = theme.gridLineWidth;

  // Vertical lines
  let x = theme.headerWidth;
  for (let col = 0; col <= range.endCol; col++) {
    const colWidth = getColWidth(col);
    x += colWidth;
    const screenX = x - scrollOffset.x;

    if (screenX >= theme.headerWidth && screenX <= viewportSize.width) {
      ctx.beginPath();
      ctx.moveTo(screenX, theme.headerHeight);
      ctx.lineTo(screenX, viewportSize.height);
      ctx.stroke();
    }
  }

  // Horizontal lines
  let y = theme.headerHeight;
  for (let row = 0; row <= range.endRow; row++) {
    const rowHeight = getRowHeight(row);
    y += rowHeight;
    const screenY = y - scrollOffset.y;

    if (screenY >= theme.headerHeight && screenY <= viewportSize.height) {
      ctx.beginPath();
      ctx.moveTo(theme.headerWidth, screenY);
      ctx.lineTo(viewportSize.width, screenY);
      ctx.stroke();
    }
  }
}

function renderCellContent(): void {
  if (!ctx) return;

  for (const cellData of cellsData) {
    if (!cellData.displayValue) continue;

    const { row, col } = cellData;
    const pos = gridToScreen(row, col);
    const colWidth = getColWidth(col);
    const rowHeight = getRowHeight(row);

    // Skip if outside viewport
    if (pos.x + colWidth < theme.headerWidth || pos.x > viewportSize.width) continue;
    if (pos.y + rowHeight < theme.headerHeight || pos.y > viewportSize.height) continue;

    // Background color
    if (cellData.format.background_color) {
      ctx.fillStyle = cellData.format.background_color;
      ctx.fillRect(pos.x, pos.y, colWidth, rowHeight);
    }

    // Text styling
    let fontStyle = '';
    if (cellData.format.bold) fontStyle += 'bold ';
    if (cellData.format.italic) fontStyle += 'italic ';
    ctx.font = fontStyle + theme.cellFont;
    ctx.fillStyle = cellData.format.text_color || theme.cellTextColor;
    ctx.textBaseline = 'top';

    // Horizontal alignment
    let textX = pos.x + theme.cellPadding;
    const horizontalAlign = cellData.format.horizontal_align || 'left';
    const metrics = ctx.measureText(cellData.displayValue);
    const textWidth = metrics.width;

    if (horizontalAlign === 'center') {
      textX = pos.x + (colWidth - textWidth) / 2;
    } else if (horizontalAlign === 'right') {
      textX = pos.x + colWidth - textWidth - theme.cellPadding;
    }

    // Vertical alignment
    let textY = pos.y + theme.cellPadding;
    const verticalAlign = cellData.format.vertical_align || 'top';

    if (verticalAlign === 'middle') {
      textY = pos.y + (rowHeight - 13) / 2;
    } else if (verticalAlign === 'bottom') {
      textY = pos.y + rowHeight - 13 - theme.cellPadding;
    }

    // Clip and draw
    ctx.save();
    ctx.beginPath();
    ctx.rect(pos.x, pos.y, colWidth, rowHeight);
    ctx.clip();

    ctx.fillText(cellData.displayValue, textX, textY);

    if (cellData.format.underline) {
      ctx.strokeStyle = cellData.format.text_color || theme.cellTextColor;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(textX, textY + 14);
      ctx.lineTo(textX + textWidth, textY + 14);
      ctx.stroke();
    }

    ctx.restore();
  }
}

function renderHeaders(range: CellRange): void {
  if (!ctx) return;

  // Header backgrounds
  ctx.fillStyle = theme.headerBackground;
  ctx.fillRect(0, 0, theme.headerWidth, theme.headerHeight);
  ctx.fillRect(theme.headerWidth, 0, viewportSize.width - theme.headerWidth, theme.headerHeight);
  ctx.fillRect(0, theme.headerHeight, theme.headerWidth, viewportSize.height - theme.headerHeight);

  // Header text
  ctx.font = theme.headerFont;
  ctx.fillStyle = theme.headerText;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';

  // Column headers
  for (let col = range.startCol; col <= range.endCol; col++) {
    const pos = gridToScreen(0, col);
    const colWidth = getColWidth(col);
    const centerX = pos.x + colWidth / 2;
    const centerY = theme.headerHeight / 2;

    if (centerX >= theme.headerWidth && centerX <= viewportSize.width) {
      ctx.fillText(colToLetter(col), centerX, centerY);
    }
  }

  // Row headers
  for (let row = range.startRow; row <= range.endRow; row++) {
    const pos = gridToScreen(row, 0);
    const rowHeight = getRowHeight(row);
    const centerX = theme.headerWidth / 2;
    const centerY = pos.y + rowHeight / 2;

    if (centerY >= theme.headerHeight && centerY <= viewportSize.height) {
      ctx.fillText(String(row + 1), centerX, centerY);
    }
  }

  // Header borders
  ctx.strokeStyle = theme.gridLineColor;
  ctx.lineWidth = theme.gridLineWidth;

  ctx.beginPath();
  ctx.moveTo(theme.headerWidth, 0);
  ctx.lineTo(theme.headerWidth, viewportSize.height);
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(0, theme.headerHeight);
  ctx.lineTo(viewportSize.width, theme.headerHeight);
  ctx.stroke();
}

function renderSelection(): void {
  if (!ctx) return;

  const pos = gridToScreen(activeCell.row, activeCell.col);
  const colWidth = getColWidth(activeCell.col);
  const rowHeight = getRowHeight(activeCell.row);

  ctx.strokeStyle = theme.activeCellBorder;
  ctx.lineWidth = theme.activeCellBorderWidth;
  ctx.strokeRect(pos.x, pos.y, colWidth, rowHeight);
}

function render(): void {
  if (!ctx || !canvas) return;

  const { width, height } = viewportSize;

  // Clear
  ctx.fillStyle = theme.backgroundColor;
  ctx.fillRect(0, 0, width, height);

  const range = calculateVisibleRange();

  // Render layers
  renderGrid(range);
  renderCellContent();
  renderHeaders(range);
  renderSelection();

  needsRender = false;
}

function scheduleRender(): void {
  if (needsRender) return;
  needsRender = true;

  if (animationFrameId === null) {
    animationFrameId = requestAnimationFrame(() => {
      animationFrameId = null;
      if (needsRender) {
        render();
      }
    });
  }
}

function requestDataForVisibleRange(): void {
  const range = calculateVisibleRange();
  postMessage({
    type: 'REQUEST_DATA',
    visibleRange: range,
  });
}

// =============================================================================
// Message Handlers
// =============================================================================

function handleInit(msg: MainToWorkerMessage & { type: 'INIT' }): void {
  canvas = msg.canvas;
  viewportSize = msg.viewportSize;
  devicePixelRatio = msg.devicePixelRatio;

  const context = canvas.getContext('2d');
  if (!context) {
    postMessage({
      type: 'ERROR',
      message: 'Failed to get 2D context from OffscreenCanvas',
    });
    return;
  }
  ctx = context;

  // Scale for high-DPI displays
  ctx.scale(devicePixelRatio, devicePixelRatio);

  postMessage({ type: 'READY' });
  requestDataForVisibleRange();
}

function handleScroll(msg: MainToWorkerMessage & { type: 'SCROLL' }): void {
  scrollOffset = msg.offset;
  requestDataForVisibleRange();
  scheduleRender();
}

function handleResize(msg: MainToWorkerMessage & { type: 'RESIZE' }): void {
  viewportSize = msg.viewportSize;
  devicePixelRatio = msg.devicePixelRatio;

  if (canvas) {
    canvas.width = viewportSize.width * devicePixelRatio;
    canvas.height = viewportSize.height * devicePixelRatio;
  }

  if (ctx) {
    ctx.scale(devicePixelRatio, devicePixelRatio);
  }

  requestDataForVisibleRange();
  scheduleRender();
}

function handleSelect(msg: MainToWorkerMessage & { type: 'SELECT' }): void {
  activeCell = msg.cell;
  scheduleRender();
}

function handleUpdateData(msg: MainToWorkerMessage & { type: 'UPDATE_DATA' }): void {
  cellsData = msg.cells;

  // Convert row/col dimensions
  if (msg.rowHeights instanceof Map) {
    rowHeights = msg.rowHeights;
  } else {
    rowHeights = new Map(Object.entries(msg.rowHeights).map(([k, v]) => [Number(k), v]));
  }

  if (msg.colWidths instanceof Map) {
    colWidths = msg.colWidths;
  } else {
    colWidths = new Map(Object.entries(msg.colWidths).map(([k, v]) => [Number(k), v]));
  }

  scheduleRender();
}

function handleRender(): void {
  scheduleRender();
}

function handleSetDimension(msg: MainToWorkerMessage & { type: 'SET_DIMENSION' }): void {
  if (msg.dimension === 'row') {
    rowHeights.set(msg.index, msg.size);
  } else {
    colWidths.set(msg.index, msg.size);
  }
  scheduleRender();
}

// =============================================================================
// Main Message Handler
// =============================================================================

self.onmessage = (event: MessageEvent<MainToWorkerMessage>) => {
  const msg = event.data;

  try {
    switch (msg.type) {
      case 'INIT':
        handleInit(msg);
        break;
      case 'SCROLL':
        handleScroll(msg);
        break;
      case 'RESIZE':
        handleResize(msg);
        break;
      case 'SELECT':
        handleSelect(msg);
        break;
      case 'UPDATE_DATA':
        handleUpdateData(msg);
        break;
      case 'RENDER':
        handleRender();
        break;
      case 'SET_DIMENSION':
        handleSetDimension(msg);
        break;
    }
  } catch (error) {
    postMessage({
      type: 'ERROR',
      message: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
    });
  }
};
