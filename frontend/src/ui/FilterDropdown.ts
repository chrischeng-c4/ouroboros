import { rusheet } from '../core/RusheetAPI';

export interface FilterDropdownOptions {
  onClose?: () => void;
}

/**
 * FilterDropdown - UI component for column filtering
 * Shows a checkbox list of unique values in a column
 */
export class FilterDropdown {
  private container: HTMLDivElement;
  private col: number = -1;
  private allValues: string[] = [];
  private selectedValues: Set<string> = new Set();
  private onClose?: () => void;

  constructor(options: FilterDropdownOptions = {}) {
    this.onClose = options.onClose;
    this.container = this.createContainer();
    document.body.appendChild(this.container);

    // Close on outside click
    document.addEventListener('mousedown', this.handleOutsideClick);
  }

  private createContainer(): HTMLDivElement {
    const container = document.createElement('div');
    container.className = 'filter-dropdown';
    container.style.display = 'none';
    return container;
  }

  private handleOutsideClick = (e: MouseEvent) => {
    if (!this.container.contains(e.target as Node) && this.container.style.display !== 'none') {
      this.hide();
    }
  };

  /**
   * Show the filter dropdown for a specific column
   */
  show(col: number, x: number, y: number): void {
    this.col = col;
    this.allValues = rusheet.getUniqueValuesInColumn(col);

    // Get currently active filter for this column
    const activeFilters = rusheet.getActiveFilters();
    const existingFilter = activeFilters.find(f => f.col === col);

    if (existingFilter) {
      this.selectedValues = new Set(existingFilter.visibleValues);
    } else {
      // All values selected by default
      this.selectedValues = new Set(this.allValues);
    }

    this.render();

    // Position the dropdown
    this.container.style.left = `${x}px`;
    this.container.style.top = `${y}px`;
    this.container.style.display = 'block';

    // Ensure it doesn't go off screen
    const rect = this.container.getBoundingClientRect();
    if (rect.right > window.innerWidth) {
      this.container.style.left = `${window.innerWidth - rect.width - 10}px`;
    }
    if (rect.bottom > window.innerHeight) {
      this.container.style.top = `${window.innerHeight - rect.height - 10}px`;
    }
  }

  hide(): void {
    this.container.style.display = 'none';
    this.onClose?.();
  }

  isVisible(): boolean {
    return this.container.style.display !== 'none';
  }

  private render(): void {
    const hasActiveFilter = this.selectedValues.size < this.allValues.length;

    this.container.innerHTML = `
      <div class="filter-header">
        <span>Filter Column ${String.fromCharCode(65 + this.col)}</span>
        ${hasActiveFilter ? '<span class="filter-active-badge">Filtered</span>' : ''}
      </div>
      <div class="filter-actions">
        <button class="filter-btn filter-select-all">Select All</button>
        <button class="filter-btn filter-clear-all">Clear All</button>
      </div>
      <div class="filter-items">
        ${this.allValues.length === 0
          ? '<div class="filter-empty">No values found</div>'
          : this.allValues.map(value => `
            <label class="filter-item">
              <input type="checkbox" value="${this.escapeHtml(value)}"
                ${this.selectedValues.has(value) ? 'checked' : ''}>
              <span>${this.escapeHtml(value) || '(Empty)'}</span>
            </label>
          `).join('')
        }
      </div>
      <div class="filter-footer">
        <button class="filter-btn filter-ok">OK</button>
        <button class="filter-btn filter-cancel">Cancel</button>
        ${hasActiveFilter ? '<button class="filter-btn filter-clear">Clear Filter</button>' : ''}
      </div>
    `;

    // Add event listeners
    this.container.querySelector('.filter-select-all')?.addEventListener('click', () => {
      this.selectedValues = new Set(this.allValues);
      this.render();
    });

    this.container.querySelector('.filter-clear-all')?.addEventListener('click', () => {
      this.selectedValues.clear();
      this.render();
    });

    this.container.querySelectorAll('.filter-item input').forEach(checkbox => {
      checkbox.addEventListener('change', (e) => {
        const target = e.target as HTMLInputElement;
        if (target.checked) {
          this.selectedValues.add(target.value);
        } else {
          this.selectedValues.delete(target.value);
        }
      });
    });

    this.container.querySelector('.filter-ok')?.addEventListener('click', () => {
      this.applyFilter();
      this.hide();
    });

    this.container.querySelector('.filter-cancel')?.addEventListener('click', () => {
      this.hide();
    });

    this.container.querySelector('.filter-clear')?.addEventListener('click', () => {
      rusheet.clearColumnFilter(this.col);
      this.hide();
    });
  }

  private applyFilter(): void {
    if (this.selectedValues.size === this.allValues.length) {
      // All selected = no filter
      rusheet.clearColumnFilter(this.col);
    } else if (this.selectedValues.size === 0) {
      // None selected = hide all (show empty set)
      rusheet.applyColumnFilter(this.col, []);
    } else {
      rusheet.applyColumnFilter(this.col, Array.from(this.selectedValues));
    }
  }

  private escapeHtml(str: string): string {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  destroy(): void {
    document.removeEventListener('mousedown', this.handleOutsideClick);
    this.container.remove();
  }
}
