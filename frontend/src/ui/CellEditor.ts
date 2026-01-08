import type { IGridRenderer } from '../types/renderer';
import { rusheet } from '../core/RusheetAPI';
import { theme } from '../canvas/theme';
import { AutocompleteEngine } from './AutocompleteEngine';
import { AutocompleteUI } from './AutocompleteUI';

export default class CellEditor {
  private container: HTMLElement;
  private renderer: IGridRenderer;
  private formulaBar: HTMLInputElement;
  private textarea: HTMLTextAreaElement;
  private isEditing: boolean = false;
  private currentRow: number = -1;
  private currentCol: number = -1;
  private onCellChange?: (row: number, col: number) => void;
  private autocompleteEngine: AutocompleteEngine;
  private autocompleteUI: AutocompleteUI;
  private autocompleteEnabled: boolean = true;
  private autocompleteDebounce: number | null = null;

  constructor(
    container: HTMLElement,
    renderer: IGridRenderer,
    formulaBar: HTMLInputElement
  ) {
    this.container = container;
    this.renderer = renderer;
    this.formulaBar = formulaBar;
    this.textarea = this.createTextarea();
    this.autocompleteEngine = new AutocompleteEngine();
    this.autocompleteUI = new AutocompleteUI(container);
    this.setupEventListeners();
  }

  /**
   * Create the textarea element for inline editing
   */
  private createTextarea(): HTMLTextAreaElement {
    const textarea = document.createElement('textarea');
    textarea.style.position = 'absolute';
    textarea.style.display = 'none';
    textarea.style.border = `${theme.activeCellBorderWidth}px solid ${theme.activeCellBorder}`;
    textarea.style.outline = 'none';
    textarea.style.resize = 'none';
    textarea.style.overflow = 'hidden';
    textarea.style.padding = `${theme.cellPadding}px`;
    textarea.style.margin = '0';
    textarea.style.font = theme.cellFont;
    textarea.style.color = theme.cellTextColor;
    textarea.style.backgroundColor = theme.backgroundColor;
    textarea.style.zIndex = '1000';
    textarea.style.boxSizing = 'border-box';

    this.container.appendChild(textarea);
    return textarea;
  }

  /**
   * Setup event listeners for the textarea and formula bar
   */
  private setupEventListeners(): void {
    // Textarea events
    this.textarea.addEventListener('keydown', this.handleTextareaKeydown.bind(this));
    this.textarea.addEventListener('input', this.handleTextareaInput.bind(this));
    this.textarea.addEventListener('blur', this.handleTextareaBlur.bind(this));

    // Formula bar events
    this.formulaBar.addEventListener('keydown', this.handleFormulaBarKeydown.bind(this));
    this.formulaBar.addEventListener('input', this.handleFormulaBarInput.bind(this));
    this.formulaBar.addEventListener('focus', this.handleFormulaBarFocus.bind(this));

    // Click outside to commit
    document.addEventListener('mousedown', this.handleDocumentClick.bind(this));
  }

  /**
   * Handle textarea keydown events
   */
  private handleTextareaKeydown(event: KeyboardEvent): void {
    // Handle autocomplete shortcuts first
    if (this.autocompleteUI.isVisible()) {
      if (event.key === 'Tab') {
        event.preventDefault();
        this.acceptAutocomplete();
        return;
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        this.autocompleteUI.navigate(1);
        return;
      }
      if (event.key === 'ArrowUp') {
        event.preventDefault();
        this.autocompleteUI.navigate(-1);
        return;
      }
      if (event.key === 'Escape') {
        event.preventDefault();
        this.autocompleteUI.hide();
        return;
      }
    }

    // Existing key handling
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      event.stopPropagation(); // Prevent InputController from re-activating
      this.commit();
      // Move down to next cell
      const activeCell = this.renderer.getActiveCell();
      this.renderer.setActiveCell(activeCell.row + 1, activeCell.col);
      if (this.onCellChange) {
        this.onCellChange(activeCell.row + 1, activeCell.col);
      }
    } else if (event.key === 'Tab') {
      event.preventDefault();
      event.stopPropagation(); // Prevent InputController from also handling Tab
      this.commit();
      // Move right to next cell
      const activeCell = this.renderer.getActiveCell();
      const newCol = event.shiftKey ? activeCell.col - 1 : activeCell.col + 1;
      this.renderer.setActiveCell(activeCell.row, Math.max(0, newCol));
      if (this.onCellChange) {
        this.onCellChange(activeCell.row, Math.max(0, newCol));
      }
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation(); // Prevent InputController from handling Escape
      this.cancel();
    }
  }

  /**
   * Handle textarea input events
   */
  private handleTextareaInput(): void {
    // Sync with formula bar
    this.formulaBar.value = this.textarea.value;

    // Auto-resize textarea height
    this.textarea.style.height = 'auto';
    this.textarea.style.height = this.textarea.scrollHeight + 'px';

    // Emit cell edit change event
    if (this.isEditing) {
      rusheet.emitCellEdit(this.currentRow, this.currentCol, this.textarea.value, 'change');
    }

    // Trigger autocomplete with debounce
    if (this.autocompleteEnabled) {
      if (this.autocompleteDebounce !== null) {
        clearTimeout(this.autocompleteDebounce);
      }

      this.autocompleteDebounce = window.setTimeout(() => {
        this.showAutocomplete();
        this.autocompleteDebounce = null;
      }, 150);
    }
  }

  /**
   * Handle textarea blur events
   */
  private handleTextareaBlur(event: FocusEvent): void {
    // Don't commit if focus moved to formula bar or autocomplete dropdown
    const relatedTarget = event.relatedTarget as HTMLElement;
    if (relatedTarget === this.formulaBar) {
      return;
    }

    // Check if focus moved to autocomplete dropdown
    if (relatedTarget && relatedTarget.closest('.autocomplete-dropdown')) {
      return;
    }

    // Commit immediately instead of with delay
    if (this.isEditing) {
      this.commit();
    }
  }

  /**
   * Handle formula bar keydown events
   */
  private handleFormulaBarKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      event.stopPropagation(); // Prevent InputController from re-activating
      this.commit();
      // Move down to next cell
      const activeCell = this.renderer.getActiveCell();
      this.renderer.setActiveCell(activeCell.row + 1, activeCell.col);
      if (this.onCellChange) {
        this.onCellChange(activeCell.row + 1, activeCell.col);
      }
    } else if (event.key === 'Tab') {
      event.preventDefault();
      event.stopPropagation(); // Prevent InputController from also handling Tab
      this.commit();
      // Move right to next cell
      const activeCell = this.renderer.getActiveCell();
      const newCol = event.shiftKey ? activeCell.col - 1 : activeCell.col + 1;
      this.renderer.setActiveCell(activeCell.row, Math.max(0, newCol));
      if (this.onCellChange) {
        this.onCellChange(activeCell.row, Math.max(0, newCol));
      }
    } else if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation(); // Prevent InputController from handling Escape
      this.cancel();
    }
  }

  /**
   * Handle formula bar input events
   */
  private handleFormulaBarInput(): void {
    if (this.isEditing) {
      // Sync with cell editor
      this.textarea.value = this.formulaBar.value;
      this.textarea.style.height = 'auto';
      this.textarea.style.height = this.textarea.scrollHeight + 'px';
    }
  }

  /**
   * Handle formula bar focus events
   */
  private handleFormulaBarFocus(): void {
    if (!this.isEditing) {
      const activeCell = this.renderer.getActiveCell();
      this.activate(activeCell.row, activeCell.col);
    }
  }

  /**
   * Handle document click events
   */
  private handleDocumentClick(event: MouseEvent): void {
    if (!this.isEditing) return;

    const target = event.target as HTMLElement;

    // Don't commit if clicking on textarea or formula bar
    if (target === this.textarea || target === this.formulaBar) {
      return;
    }

    // Don't commit if clicking inside container but might be on canvas
    // Let the canvas handle cell selection
    if (this.container.contains(target)) {
      this.commit();
    }
  }

  /**
   * Activate the cell editor at the specified position
   */
  public activate(row: number, col: number): void {
    this.currentRow = row;
    this.currentCol = col;
    this.isEditing = true;

    // Sync renderer's active cell with editor position
    this.renderer.setActiveCell(row, col);

    // Get current cell value for editing
    // For formulas: show the formula expression (e.g., "=SUM(A1:B10)")
    // For regular values: show the original input (e.g., "10", "Hello")
    const cellData = rusheet.getCellData(row, col);
    const value = cellData?.formula || cellData?.value || '';

    // Set values
    this.textarea.value = value;
    this.formulaBar.value = value;

    // Position the textarea
    this.positionTextarea(row, col);

    // Show and focus the textarea
    this.textarea.style.display = 'block';
    this.textarea.focus();
    this.textarea.select();

    // Auto-resize
    this.textarea.style.height = 'auto';
    this.textarea.style.height = this.textarea.scrollHeight + 'px';

    // Emit cell edit start event
    rusheet.emitCellEdit(row, col, value, 'start');
  }

  /**
   * Position the textarea over the cell
   */
  private positionTextarea(row: number, col: number): void {
    const pos = this.renderer.gridToScreen(row, col);
    const colWidth = rusheet.getColWidth(col);
    const rowHeight = rusheet.getRowHeight(row);

    this.textarea.style.left = `${pos.x}px`;
    this.textarea.style.top = `${pos.y}px`;
    this.textarea.style.width = `${colWidth}px`;
    this.textarea.style.minHeight = `${rowHeight}px`;
  }

  /**
   * Commit the current value and hide the editor
   */
  public commit(): void {
    if (!this.isEditing) return;

    const value = this.textarea.value;
    const row = this.currentRow;
    const col = this.currentCol;

    // Save value via rusheet API
    rusheet.setCellValue(row, col, value, 'user');

    // Emit cell edit end event
    rusheet.emitCellEdit(row, col, value, 'end');

    // Hide editor
    this.hide();

    // NEW: Trigger re-render to show the updated value
    this.renderer.render();
  }

  /**
   * Cancel editing and hide the editor
   */
  public cancel(): void {
    if (!this.isEditing) return;

    const row = this.currentRow;
    const col = this.currentCol;

    // Restore original value to formula bar
    const cellData = rusheet.getCellData(row, col);
    const value = cellData?.formula || cellData?.value || '';
    this.formulaBar.value = value;

    // Emit cell edit cancel event
    rusheet.emitCellEdit(row, col, value, 'cancel');

    // Hide editor
    this.hide();
  }

  /**
   * Hide the editor
   */
  private hide(): void {
    this.isEditing = false;
    this.textarea.style.display = 'none';
    this.textarea.value = '';
    this.currentRow = -1;
    this.currentCol = -1;
  }

  /**
   * Check if the editor is currently active
   */
  public isActive(): boolean {
    return this.isEditing;
  }

  /**
   * Set callback for cell change events
   */
  public setCellChangeCallback(callback: (row: number, col: number) => void): void {
    this.onCellChange = callback;
  }

  /**
   * Update the editor position (e.g., after scrolling)
   */
  public updatePosition(): void {
    if (this.isEditing) {
      this.positionTextarea(this.currentRow, this.currentCol);
    }
  }

  /**
   * Get the current editing position
   */
  public getCurrentPosition(): { row: number; col: number } | null {
    if (!this.isEditing) return null;
    return { row: this.currentRow, col: this.currentCol };
  }

  /**
   * Show autocomplete suggestions based on current input
   */
  private showAutocomplete(): void {
    if (!this.textarea) return;

    const text = this.textarea.value;
    const cursorPos = this.textarea.selectionStart || 0;
    const context = this.autocompleteEngine.createContext(text, cursorPos);

    if (!context.isFormula) {
      this.autocompleteUI.hide();
      return;
    }

    const suggestions = this.autocompleteEngine.getSuggestions(context);

    if (suggestions.length > 0) {
      // Calculate cursor position for dropdown placement
      const rect = this.textarea.getBoundingClientRect();
      this.autocompleteUI.show(suggestions, {
        x: rect.left,
        y: rect.bottom,
      });
    } else {
      this.autocompleteUI.hide();
    }
  }

  /**
   * Accept the currently selected autocomplete suggestion
   */
  private acceptAutocomplete(): void {
    const suggestion = this.autocompleteUI.getSelected();
    if (!suggestion || !this.textarea) return;

    const text = this.textarea.value;
    const cursorPos = this.textarea.selectionStart || 0;
    const context = this.autocompleteEngine.createContext(text, cursorPos);

    // Replace current token with suggestion
    const before = text.substring(0, context.tokenStartPos);
    const after = text.substring(cursorPos);
    const newText = before + suggestion.insertText + after;

    this.textarea.value = newText;
    this.formulaBar.value = newText;

    // Set cursor position after inserted text
    const newCursorPos = context.tokenStartPos + suggestion.insertText.length;
    this.textarea.setSelectionRange(newCursorPos, newCursorPos);

    this.autocompleteUI.hide();
    this.textarea.focus();
  }

  /**
   * Toggle autocomplete feature on or off
   */
  public toggleAutocomplete(enabled: boolean): void {
    this.autocompleteEnabled = enabled;
    if (!enabled) {
      this.autocompleteUI.hide();
    }
  }
}
