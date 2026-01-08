import * as WasmBridge from './WasmBridge';

export class PersistenceManager {
  private static readonly STORAGE_KEY = 'rusheet_workbook';
  private static readonly AUTO_SAVE_DELAY_MS = 1000;
  private saveTimeout: number | null = null;

  /**
   * Load workbook from localStorage
   * Returns true if data was loaded successfully
   */
  public load(): boolean {
    try {
      const json = localStorage.getItem(PersistenceManager.STORAGE_KEY);
      if (!json) {
        return false;
      }

      const success = WasmBridge.deserialize(json);
      return success;
    } catch (error) {
      return false;
    }
  }

  /**
   * Save workbook to localStorage immediately
   */
  public save(): boolean {
    try {
      const json = WasmBridge.serialize();
      localStorage.setItem(PersistenceManager.STORAGE_KEY, json);
      return true;
    } catch (error) {
      return false;
    }
  }

  /**
   * Schedule auto-save with debounce
   * Cancels previous pending save if called again within delay
   */
  public scheduleSave(): void {
    // Clear existing timeout
    if (this.saveTimeout !== null) {
      clearTimeout(this.saveTimeout);
    }

    // Schedule new save
    this.saveTimeout = window.setTimeout(() => {
      this.save();
      this.saveTimeout = null;
    }, PersistenceManager.AUTO_SAVE_DELAY_MS);
  }

  /**
   * Clear saved data from localStorage
   */
  public clear(): boolean {
    try {
      localStorage.removeItem(PersistenceManager.STORAGE_KEY);
      return true;
    } catch (error) {
      return false;
    }
  }

  /**
   * Export workbook as downloadable file
   */
  public exportToFile(filename: string = 'workbook.json'): void {
    try {
      const json = WasmBridge.serialize();
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);

      const link = document.createElement('a');
      link.href = url;
      link.download = filename;
      link.click();

      URL.revokeObjectURL(url);
    } catch (error) {
      // Silent failure
    }
  }

  /**
   * Import workbook from file
   */
  public importFromFile(file: File, onComplete: (success: boolean) => void): void {
    const reader = new FileReader();

    reader.onload = (e) => {
      try {
        const json = e.target?.result as string;
        const success = WasmBridge.deserialize(json);

        if (success) {
          // Save to localStorage after successful import
          this.save();
        }

        onComplete(success);
      } catch (error) {
        onComplete(false);
      }
    };

    reader.onerror = () => {
      onComplete(false);
    };

    reader.readAsText(file);
  }
}
