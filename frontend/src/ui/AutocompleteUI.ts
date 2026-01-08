import { Suggestion } from './AutocompleteEngine';

export class AutocompleteUI {
  private container: HTMLElement;
  private dropdown: HTMLElement | null = null;
  private suggestions: Suggestion[] = [];
  private selectedIndex: number = 0;

  constructor(container: HTMLElement) {
    this.container = container;
  }

  /**
   * Show autocomplete dropdown with suggestions
   */
  public show(suggestions: Suggestion[], position: { x: number; y: number }): void {
    this.suggestions = suggestions;
    this.selectedIndex = 0;

    // Create dropdown if it doesn't exist
    if (!this.dropdown) {
      this.dropdown = document.createElement('div');
      this.dropdown.className = 'autocomplete-dropdown';
      this.container.appendChild(this.dropdown);
    }

    // Render suggestions
    this.dropdown.innerHTML = '';
    suggestions.forEach((suggestion, index) => {
      const item = document.createElement('div');
      item.className = 'autocomplete-item';
      if (index === this.selectedIndex) {
        item.classList.add('selected');
      }

      const mainText = document.createElement('div');
      mainText.className = 'autocomplete-main';
      mainText.textContent = suggestion.value;

      const syntaxText = document.createElement('div');
      syntaxText.className = 'autocomplete-syntax';
      syntaxText.textContent = suggestion.display;

      item.appendChild(mainText);
      item.appendChild(syntaxText);

      if (suggestion.description) {
        const descText = document.createElement('div');
        descText.className = 'autocomplete-desc';
        descText.textContent = suggestion.description;
        item.appendChild(descText);
      }

      item.addEventListener('click', () => {
        this.selectedIndex = index;
        this.triggerAccept();
      });

      this.dropdown!.appendChild(item);
    });

    // Position dropdown
    this.dropdown.style.left = `${position.x}px`;
    this.dropdown.style.top = `${position.y + 20}px`; // 20px below cursor
    this.dropdown.style.display = 'block';

    // Scroll selected item into view
    this.scrollToSelected();
  }

  /**
   * Hide autocomplete dropdown
   */
  public hide(): void {
    if (this.dropdown) {
      this.dropdown.style.display = 'none';
    }
    this.suggestions = [];
    this.selectedIndex = 0;
  }

  /**
   * Check if dropdown is visible
   */
  public isVisible(): boolean {
    return this.dropdown !== null && this.dropdown.style.display !== 'none';
  }

  /**
   * Navigate selection up or down
   */
  public navigate(delta: number): void {
    if (this.suggestions.length === 0) return;

    // Update selected index with wrapping
    this.selectedIndex = (this.selectedIndex + delta + this.suggestions.length) % this.suggestions.length;

    // Update visual selection
    if (this.dropdown) {
      const items = this.dropdown.querySelectorAll('.autocomplete-item');
      items.forEach((item, index) => {
        if (index === this.selectedIndex) {
          item.classList.add('selected');
        } else {
          item.classList.remove('selected');
        }
      });

      this.scrollToSelected();
    }
  }

  /**
   * Get currently selected suggestion
   */
  public getSelected(): Suggestion | null {
    if (this.selectedIndex >= 0 && this.selectedIndex < this.suggestions.length) {
      return this.suggestions[this.selectedIndex];
    }
    return null;
  }

  /**
   * Get selected index
   */
  public getSelectedIndex(): number {
    return this.selectedIndex;
  }

  /**
   * Scroll selected item into view
   */
  private scrollToSelected(): void {
    if (!this.dropdown) return;

    const items = this.dropdown.querySelectorAll('.autocomplete-item');
    if (items[this.selectedIndex]) {
      items[this.selectedIndex].scrollIntoView({ block: 'nearest' });
    }
  }

  /**
   * Trigger accept callback (for testing/external use)
   */
  private triggerAccept(): void {
    // This will be handled by CellEditor
    // Just update the UI
  }
}
