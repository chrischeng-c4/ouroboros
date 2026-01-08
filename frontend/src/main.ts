// Import CSS styles
import './styles/main.css';

// Import core modules
import { rusheet } from './core/RusheetAPI';
import GridRenderer from './canvas/GridRenderer';
import { RenderController, isOffscreenCanvasSupported } from './worker';
import type { IGridRenderer, RemoteCursor } from './types/renderer';
import InputController from './ui/InputController';
import CellEditor from './ui/CellEditor';
import { PersistenceManager } from './core/PersistenceManager';
import {
  initCollaboration,
  createCollaborationUI,
  type CollaborationProvider,
} from './collab';

// Configuration: Set to true to use OffscreenCanvas worker rendering
const USE_OFFSCREEN_CANVAS = false; // Set to true to enable worker-based rendering

// Collaboration server URL (can be overridden via environment or URL params)
const COLLAB_SERVER_URL = (import.meta.env?.VITE_COLLAB_SERVER_URL as string) || 'http://localhost:3000';

/**
 * Convert column index to Excel-style letter notation
 * 0 -> A, 1 -> B, ..., 25 -> Z, 26 -> AA, etc.
 */
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

/**
 * Get workbook ID from URL params or generate a new one
 */
function getWorkbookId(): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get('workbook');
}

/**
 * Get user name from URL params or localStorage
 */
function getUserName(): string {
  const params = new URLSearchParams(window.location.search);
  const urlName = params.get('user');
  if (urlName) return urlName;

  const storedName = localStorage.getItem('rusheet.userName');
  if (storedName) return storedName;

  const defaultName = `User-${Math.random().toString(36).slice(2, 6)}`;
  localStorage.setItem('rusheet.userName', defaultName);
  return defaultName;
}

/**
 * Initialize collaboration mode if workbook ID is present in URL
 */
function initCollaborationMode(): { provider: CollaborationProvider; workbookId: string } | null {
  const workbookId = getWorkbookId();
  if (!workbookId) {
    console.log('[RuSheet] No workbook ID in URL, running in offline mode');
    return null;
  }

  console.log('[RuSheet] Initializing collaboration for workbook:', workbookId);

  const provider = initCollaboration({
    serverUrl: COLLAB_SERVER_URL,
    workbookId,
    userName: getUserName(),
  });

  return { provider, workbookId };
}

/**
 * Main application entry point
 */
async function main(): Promise<void> {
  try {
    // Step 1: Initialize WASM module
    await rusheet.init();

    // Step 1.5: Initialize collaboration if workbook ID in URL
    const collabResult = initCollaborationMode();
    let _collabUI: ReturnType<typeof createCollaborationUI> | null = null;

    // Step 1.6: Initialize persistence and try to load saved data
    // Skip loading local data if in collaboration mode (will sync from server)
    const persistence = new PersistenceManager();
    const hasData = collabResult ? false : persistence.load();

    // Step 2: Get DOM elements
    const canvas = document.getElementById('spreadsheet-canvas') as HTMLCanvasElement;
    const formulaInput = document.getElementById('formula-input') as HTMLInputElement;
    const cellAddress = document.getElementById('cell-address') as HTMLSpanElement;
    const addSheetBtn = document.getElementById('add-sheet-btn') as HTMLButtonElement;
    const container = document.getElementById('spreadsheet-container') as HTMLElement;

    if (!canvas || !formulaInput || !cellAddress || !addSheetBtn || !container) {
      throw new Error('Failed to find required DOM elements');
    }

    // Step 3: Create GridRenderer (or RenderController for offscreen mode)
    let renderer: IGridRenderer;

    const useOffscreen = USE_OFFSCREEN_CANVAS && isOffscreenCanvasSupported();
    if (useOffscreen) {
      renderer = new RenderController(canvas, {
        onReady: () => {
          renderer.render();
        },
        onError: (msg) => {
          console.error('[RuSheet] Render worker error:', msg);
        },
      });
    } else {
      renderer = new GridRenderer(canvas);
    }

    // Step 4: Create CellEditor
    const cellEditor = new CellEditor(container, renderer, formulaInput);

    // Step 5: Create InputController with edit mode callback
    const editModeCallback = (row: number, col: number) => {
      cellEditor.activate(row, col);
    };
    // InputController sets up event listeners in constructor
    new InputController(canvas, renderer, editModeCallback);

    // Step 6: Set up window resize handler
    const resizeCanvas = () => {
      const containerRect = container.getBoundingClientRect();
      const width = containerRect.width;
      const height = containerRect.height;

      // Set canvas size to fill container
      canvas.width = width;
      canvas.height = height;

      // Update renderer viewport and re-render
      renderer.updateViewportSize();
      renderer.render();

      // Update cell editor position if editing
      cellEditor.updatePosition();
    };

    // Initial resize
    resizeCanvas();

    // Listen for window resize events
    window.addEventListener('resize', resizeCanvas);

    // Step 7: Update cell address display when selection changes
    // Helper to update cell address display
    const updateCellAddressDisplay = () => {
      const activeCell = renderer.getActiveCell();
      cellAddress.textContent = `${colToLetter(activeCell.col)}${activeCell.row + 1}`;
    };

    // Create a render wrapper to update cell address
    const renderWithAddressUpdate = () => {
      renderer.render();
      updateCellAddressDisplay();
    };

    // Listen to filter changes and re-render
    rusheet.onFilterChange(() => {
      renderer.render();
    });

    // Set callback for cell editor to update address when moving cells
    cellEditor.setCellChangeCallback((row: number, col: number) => {
      cellAddress.textContent = `${colToLetter(col)}${row + 1}`;
      persistence.scheduleSave(); // Auto-save after edit
      renderWithAddressUpdate();
    });

    // Step 8: Set up add sheet button
    let sheetCounter = 2;
    addSheetBtn.addEventListener('click', () => {
      const sheetName = `Sheet${sheetCounter}`;
      const newIndex = rusheet.addSheet(sheetName, 'user');

      // Create new sheet tab
      const sheetTab = document.createElement('div');
      sheetTab.className = 'sheet-tab';
      sheetTab.setAttribute('data-index', String(newIndex));
      sheetTab.textContent = sheetName;

      // Remove active class from all tabs
      document.querySelectorAll('.sheet-tab').forEach(tab => tab.classList.remove('active'));

      // Add active class to new tab
      sheetTab.classList.add('active');

      // Insert before add button
      addSheetBtn.parentElement?.insertBefore(sheetTab, addSheetBtn);

      // Set as active sheet
      rusheet.setActiveSheet(newIndex, 'user');

      sheetCounter++;
      renderWithAddressUpdate();
    });

    // Set up sheet tab click handlers
    document.getElementById('sheet-tabs')?.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      if (target.classList.contains('sheet-tab')) {
        const index = parseInt(target.getAttribute('data-index') || '0', 10);

        // Remove active class from all tabs
        document.querySelectorAll('.sheet-tab').forEach(tab => tab.classList.remove('active'));

        // Add active class to clicked tab
        target.classList.add('active');

        // Set active sheet in WASM
        rusheet.setActiveSheet(index, 'user');

        renderWithAddressUpdate();
      }
    });

    // Set up persistence buttons
    const saveBtn = document.getElementById('save-btn');
    const loadBtn = document.getElementById('load-btn');
    const exportBtn = document.getElementById('export-btn');
    const importInput = document.getElementById('import-input') as HTMLInputElement;

    if (saveBtn) {
      saveBtn.addEventListener('click', () => {
        if (persistence.save()) {
          alert('Workbook saved successfully!');
        } else {
          alert('Failed to save workbook');
        }
      });
    }

    if (loadBtn) {
      loadBtn.addEventListener('click', () => {
        if (persistence.load()) {
          alert('Workbook loaded successfully!');
          renderWithAddressUpdate();
        } else {
          alert('No saved workbook found or load failed');
        }
      });
    }

    if (exportBtn) {
      exportBtn.addEventListener('click', () => {
        const filename = prompt('Enter filename:', 'rusheet-workbook.json');
        if (filename) {
          persistence.exportToFile(filename);
        }
      });
    }

    if (importInput) {
      importInput.addEventListener('change', (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file) {
          persistence.importFromFile(file, (success) => {
            if (success) {
              alert('Workbook imported successfully!');
              renderWithAddressUpdate();
            } else {
              alert('Failed to import workbook');
            }
          });
        }
      });
    }

    // Save before page unload and cleanup collaboration
    window.addEventListener('beforeunload', () => {
      persistence.save();
      if (collabResult && _collabUI) {
        _collabUI.destroy();
        collabResult.provider.disconnect();
      }
    });

    // Set up autocomplete toggle
    const autocompleteToggle = document.getElementById('autocomplete-toggle') as HTMLInputElement;
    if (autocompleteToggle) {
      // Load preference
      const enabled = localStorage.getItem('rusheet.autocomplete.enabled') !== 'false';
      autocompleteToggle.checked = enabled;
      cellEditor.toggleAutocomplete(enabled);

      autocompleteToggle.addEventListener('change', () => {
        const isEnabled = autocompleteToggle.checked;
        cellEditor.toggleAutocomplete(isEnabled);
        localStorage.setItem('rusheet.autocomplete.enabled', String(isEnabled));
      });
    }

    // Step 9: Set up collaboration UI if in collaboration mode
    if (collabResult) {
      _collabUI = createCollaborationUI(collabResult.provider);

      // Set up remote cursor updates
      const updateRemoteCursors = () => {
        const users = collabResult.provider.getUsers();
        const remoteCursors: RemoteCursor[] = users
          .filter(user => user.cursor !== undefined)
          .map(user => ({
            id: user.id,
            name: user.name,
            color: user.color,
            row: user.cursor!.row,
            col: user.cursor!.col,
          }));
        renderer.setRemoteCursors(remoteCursors);
        renderer.render();
      };

      // Listen for user changes and update remote cursors
      collabResult.provider.onUsersChange(updateRemoteCursors);

      // Initial update
      updateRemoteCursors();

      // Update share button/link
      const shareUrl = `${window.location.origin}${window.location.pathname}?workbook=${collabResult.workbookId}`;
      console.log('[RuSheet] Share URL:', shareUrl);

      // Add share button if not exists
      let shareBtn = document.getElementById('share-btn');
      if (!shareBtn) {
        shareBtn = document.createElement('button');
        shareBtn.id = 'share-btn';
        shareBtn.textContent = 'Copy Share Link';
        shareBtn.style.cssText = 'margin-left: 8px; padding: 4px 8px; cursor: pointer;';
        document.querySelector('.toolbar')?.appendChild(shareBtn);
      }

      shareBtn.addEventListener('click', () => {
        navigator.clipboard.writeText(shareUrl).then(() => {
          const originalText = shareBtn!.textContent;
          shareBtn!.textContent = 'Copied!';
          setTimeout(() => {
            shareBtn!.textContent = originalText;
          }, 2000);
        });
      });
    } else {
      // Add "Start Collaboration" button for offline mode
      let startCollabBtn = document.getElementById('start-collab-btn');
      if (!startCollabBtn) {
        startCollabBtn = document.createElement('button');
        startCollabBtn.id = 'start-collab-btn';
        startCollabBtn.textContent = 'Start Collaboration';
        startCollabBtn.style.cssText = 'margin-left: 8px; padding: 4px 8px; cursor: pointer;';
        document.querySelector('.toolbar')?.appendChild(startCollabBtn);
      }

      startCollabBtn.addEventListener('click', async () => {
        try {
          // Create a new workbook on the server
          const response = await fetch(`${COLLAB_SERVER_URL}/api/workbooks`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name: 'Untitled Workbook' }),
          });

          if (!response.ok) {
            throw new Error(`Failed to create workbook: ${response.statusText}`);
          }

          const workbook = await response.json();

          // Redirect to collaboration mode with the new workbook ID
          const collabUrl = `${window.location.origin}${window.location.pathname}?workbook=${workbook.id}`;
          window.location.href = collabUrl;
        } catch (error) {
          console.error('[RuSheet] Failed to start collaboration:', error);
          alert('Failed to start collaboration. Is the server running?');
        }
      });
    }

    // Step 10: Add test data to demonstrate functionality (only if no data was loaded)
    if (!hasData) {
      // Add header row
      rusheet.setCellValue(0, 0, 'Product', 'api');
      rusheet.setCellValue(0, 1, 'Quantity', 'api');
      rusheet.setCellValue(0, 2, 'Price', 'api');
      rusheet.setCellValue(0, 3, 'Total', 'api');

      // Add data rows
      rusheet.setCellValue(1, 0, 'Apples', 'api');
      rusheet.setCellValue(1, 1, '10', 'api');
      rusheet.setCellValue(1, 2, '1.5', 'api');
      rusheet.setCellValue(1, 3, '=B2*C2', 'api');

      rusheet.setCellValue(2, 0, 'Oranges', 'api');
      rusheet.setCellValue(2, 1, '15', 'api');
      rusheet.setCellValue(2, 2, '2.0', 'api');
      rusheet.setCellValue(2, 3, '=B3*C3', 'api');

      rusheet.setCellValue(3, 0, 'Bananas', 'api');
      rusheet.setCellValue(3, 1, '20', 'api');
      rusheet.setCellValue(3, 2, '0.75', 'api');
      rusheet.setCellValue(3, 3, '=B4*C4', 'api');

      // Add summary row
      rusheet.setCellValue(4, 0, 'Total:', 'api');
      rusheet.setCellValue(4, 3, '=SUM(D2:D4)', 'api');

      // Format header row
      rusheet.setRangeFormat(0, 0, 0, 3, {
        bold: true,
        backgroundColor: '#f0f0f0',
      }, 'api');

      // Format summary row
      rusheet.setRangeFormat(4, 0, 4, 3, {
        bold: true,
      }, 'api');
    }

    // Step 10: Initial render
    renderWithAddressUpdate();

  } catch (error) {
    console.error('[RuSheet] Initialization failed:', error);

    // Display error message to user
    const app = document.getElementById('app');
    if (app) {
      app.innerHTML = `
        <div style="display: flex; align-items: center; justify-content: center; height: 100vh; flex-direction: column; font-family: sans-serif;">
          <h1 style="color: #d32f2f;">Failed to initialize RuSheet</h1>
          <p style="color: #666; margin-top: 16px;">${error instanceof Error ? error.message : String(error)}</p>
          <p style="color: #999; margin-top: 8px; font-size: 12px;">Check the console for more details.</p>
        </div>
      `;
    }

    throw error;
  }
}

// Start the application
main();
