import type { IGridRenderer } from '../types/renderer';
import * as WasmBridge from '../core/WasmBridge';
import { FilterDropdown } from './FilterDropdown';

type EditModeCallback = (row: number, col: number) => void;

export default class InputController {
  private canvas: HTMLCanvasElement;
  private gridRenderer: IGridRenderer;
  private editModeCallback: EditModeCallback;
  private filterDropdown: FilterDropdown;

  // Event handler references for cleanup
  private mouseDownHandler: (e: MouseEvent) => void;
  private wheelHandler: (e: WheelEvent) => void;
  private keyDownHandler: (e: KeyboardEvent) => void;

  constructor(
    canvas: HTMLCanvasElement,
    gridRenderer: IGridRenderer,
    editModeCallback: EditModeCallback
  ) {
    this.canvas = canvas;
    this.gridRenderer = gridRenderer;
    this.editModeCallback = editModeCallback;

    // Initialize filter dropdown
    this.filterDropdown = new FilterDropdown({
      onClose: () => this.gridRenderer.render()
    });

    // Bind event handlers
    this.mouseDownHandler = this.handleMouseDown.bind(this);
    this.wheelHandler = this.handleWheel.bind(this);
    this.keyDownHandler = this.handleKeyDown.bind(this);

    // Attach event listeners
    this.attachEventListeners();
    this.canvas.addEventListener('dblclick', this.handleDoubleClick.bind(this));
  }

  /**
   * Attach all event listeners
   */
  private attachEventListeners(): void {
    this.canvas.addEventListener('mousedown', this.mouseDownHandler);
    this.canvas.addEventListener('wheel', this.wheelHandler);
    document.addEventListener('keydown', this.keyDownHandler);
  }

  /**
   * Handle mouse down events on canvas
   */
  private handleMouseDown(e: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Check if clicking on filter button (before other click handling)
    const filterCol = this.gridRenderer.isOnFilterButton(x, y);
    if (filterCol >= 0) {
      e.preventDefault();
      e.stopPropagation();

      // Calculate dropdown position
      const colPos = this.gridRenderer.gridToScreen(0, filterCol);
      const headerHeight = 24; // from theme

      // Show filter dropdown below the header
      this.filterDropdown.show(
        filterCol,
        rect.left + colPos.x,
        rect.top + headerHeight + 2
      );
      return;
    }

    // Convert screen coordinates to grid coordinates
    const cellPosition = this.gridRenderer.screenToGrid(x, y);

    // Set active cell
    this.gridRenderer.setActiveCell(cellPosition.row, cellPosition.col);

    // Re-render to show selection
    this.gridRenderer.render();
  }

  /**
   * Handle double-click events - activate cell editing
   */
  private handleDoubleClick(event: MouseEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    const cell = this.gridRenderer.screenToGrid(x, y);

    if (cell) {
      // Activate editing on double-click
      this.editModeCallback(cell.row, cell.col);
    }
  }

  /**
   * Handle wheel events for scrolling
   */
  private handleWheel(e: WheelEvent): void {
    e.preventDefault();

    // Update scroll offset based on wheel delta
    // deltaY is positive when scrolling down, negative when scrolling up
    // deltaX is positive when scrolling right, negative when scrolling left
    const scrollSpeed = 1; // Adjust as needed
    const deltaX = e.deltaX * scrollSpeed;
    const deltaY = e.deltaY * scrollSpeed;

    // Get current scroll position by reading from a temporary grid position
    // We'll need to track scroll internally
    const currentScroll = this.getCurrentScrollOffset();
    const newScrollX = Math.max(0, currentScroll.x + deltaX);
    const newScrollY = Math.max(0, currentScroll.y + deltaY);

    // Set new scroll offset
    this.gridRenderer.setScrollOffset(newScrollX, newScrollY);

    // Re-render
    this.gridRenderer.render();
  }

  /**
   * Get current scroll offset
   * Note: This is a helper method since GridRenderer doesn't expose scroll offset
   */
  private getCurrentScrollOffset(): { x: number; y: number } {
    // We can infer scroll by converting grid (0,0) to screen coordinates
    // and comparing with expected position
    const screenPos = this.gridRenderer.gridToScreen(0, 0);
    const theme = { headerWidth: 50, headerHeight: 30 }; // Match theme values

    return {
      x: theme.headerWidth - screenPos.x,
      y: theme.headerHeight - screenPos.y
    };
  }

  /**
   * Handle keyboard events
   */
  private handleKeyDown(e: KeyboardEvent): void {
    const activeCell = this.gridRenderer.getActiveCell();
    let handled = false;

    // Arrow key navigation
    if (e.key === 'ArrowUp') {
      if (activeCell.row > 0) {
        this.gridRenderer.setActiveCell(activeCell.row - 1, activeCell.col);
        handled = true;
      }
    } else if (e.key === 'ArrowDown') {
      if (activeCell.row < 999) {
        this.gridRenderer.setActiveCell(activeCell.row + 1, activeCell.col);
        handled = true;
      }
    } else if (e.key === 'ArrowLeft') {
      if (activeCell.col > 0) {
        this.gridRenderer.setActiveCell(activeCell.row, activeCell.col - 1);
        handled = true;
      }
    } else if (e.key === 'ArrowRight') {
      if (activeCell.col < 999) {
        this.gridRenderer.setActiveCell(activeCell.row, activeCell.col + 1);
        handled = true;
      }
    }
    // Tab navigation
    else if (e.key === 'Tab') {
      if (e.shiftKey) {
        // Shift+Tab: Move left
        if (activeCell.col > 0) {
          this.gridRenderer.setActiveCell(activeCell.row, activeCell.col - 1);
        }
      } else {
        // Tab: Move right
        if (activeCell.col < 999) {
          this.gridRenderer.setActiveCell(activeCell.row, activeCell.col + 1);
        }
      }
      handled = true;
    }
    // Enter: Trigger edit mode
    else if (e.key === 'Enter') {
      this.editModeCallback(activeCell.row, activeCell.col);
      handled = true;
    }
    // Delete/Backspace: Clear cell
    else if (e.key === 'Delete' || e.key === 'Backspace') {
      WasmBridge.setCellValue(activeCell.row, activeCell.col, '');
      handled = true;
    }
    // Ctrl+Z: Undo
    else if (e.key === 'z' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      if (WasmBridge.canUndo()) {
        WasmBridge.undo();
        handled = true;
      }
    }
    // Ctrl+Y or Ctrl+Shift+Z: Redo
    else if (
      (e.key === 'y' && (e.ctrlKey || e.metaKey)) ||
      (e.key === 'z' && (e.ctrlKey || e.metaKey) && e.shiftKey)
    ) {
      if (WasmBridge.canRedo()) {
        WasmBridge.redo();
        handled = true;
      }
    }

    // Prevent default browser behavior for handled keys
    if (handled) {
      e.preventDefault();
      // Re-render after state changes
      this.gridRenderer.render();
    }
  }

  /**
   * Cleanup method to remove event listeners
   */
  public cleanup(): void {
    this.canvas.removeEventListener('mousedown', this.mouseDownHandler);
    this.canvas.removeEventListener('wheel', this.wheelHandler);
    document.removeEventListener('keydown', this.keyDownHandler);
    this.filterDropdown?.destroy();
  }
}
