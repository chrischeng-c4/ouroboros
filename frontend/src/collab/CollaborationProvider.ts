import * as Y from 'yjs';
import { WebsocketProvider } from 'y-websocket';
import { rusheet, type CellChangeEvent } from '../core/RusheetAPI';
import * as WasmBridge from '../core/WasmBridge';
import type { CellFormat } from '../types';

export interface CollabUser {
  id: string;
  name: string;
  color: string;
  cursor?: { row: number; col: number };
}

export interface CollaborationConfig {
  serverUrl: string;
  workbookId: string;
  userName?: string;
  userColor?: string;
}

/**
 * Collaboration provider that syncs RuSheet state via Yjs/WebSocket
 */
export class CollaborationProvider {
  private doc: Y.Doc;
  private provider: WebsocketProvider | null = null;
  private cells: Y.Map<Y.Map<unknown>>;
  private meta: Y.Map<unknown>;
  private sheets: Y.Array<string>;
  private config: CollaborationConfig;
  private isApplyingRemote = false;
  private unsubscribers: (() => void)[] = [];
  private userId: string;

  constructor(config: CollaborationConfig) {
    this.config = config;
    this.userId = crypto.randomUUID();
    this.doc = new Y.Doc();

    // Yjs shared types
    // cells: Map<"sheetIndex:row,col", Map<"value"|"formula"|"format", any>>
    this.cells = this.doc.getMap('cells');
    this.meta = this.doc.getMap('meta');
    this.sheets = this.doc.getArray('sheets');

    // Initialize meta if needed (suppress unused warning)
    if (!this.meta.has('version')) {
      this.meta.set('version', 1);
    }
  }

  /**
   * Connect to the collaboration server
   */
  connect(): void {
    const wsUrl = this.config.serverUrl.replace(/^http/, 'ws');

    this.provider = new WebsocketProvider(
      wsUrl,
      this.config.workbookId,
      this.doc,
      { connect: true }
    );

    // Set up awareness (user presence)
    this.provider.awareness.setLocalStateField('user', {
      id: this.userId,
      name: this.config.userName || `User-${this.userId.slice(0, 4)}`,
      color: this.config.userColor || this.generateColor(),
    });

    // Listen for connection status
    this.provider.on('status', (event: { status: string }) => {
      console.log('[Collab] Connection status:', event.status);
    });

    // Listen for sync completion
    this.provider.on('sync', (isSynced: boolean) => {
      console.log('[Collab] Synced:', isSynced);
      if (isSynced) {
        this.loadFromYjs();
      }
    });

    // Listen for remote changes
    this.cells.observeDeep((events) => {
      if (this.isApplyingRemote) return;
      this.handleRemoteChanges(events);
    });

    // Subscribe to local RuSheet events
    this.subscribeToLocalEvents();
  }

  /**
   * Disconnect from the collaboration server
   */
  disconnect(): void {
    this.unsubscribers.forEach(unsub => unsub());
    this.unsubscribers = [];

    if (this.provider) {
      this.provider.disconnect();
      this.provider.destroy();
      this.provider = null;
    }

    this.doc.destroy();
  }

  /**
   * Get current connection status
   */
  isConnected(): boolean {
    return this.provider?.wsconnected ?? false;
  }

  /**
   * Get connected users from awareness
   */
  getUsers(): CollabUser[] {
    if (!this.provider) return [];

    const users: CollabUser[] = [];
    this.provider.awareness.getStates().forEach((state, clientId) => {
      if (state.user && clientId !== this.doc.clientID) {
        users.push(state.user as CollabUser);
      }
    });
    return users;
  }

  /**
   * Update local user's cursor position
   */
  updateCursor(row: number, col: number): void {
    if (!this.provider) return;

    const currentState = this.provider.awareness.getLocalState();
    this.provider.awareness.setLocalStateField('user', {
      ...currentState?.user,
      cursor: { row, col },
    });
  }

  /**
   * Subscribe to awareness changes (for cursor updates)
   */
  onUsersChange(callback: (users: CollabUser[]) => void): () => void {
    if (!this.provider) return () => {};

    const handler = () => callback(this.getUsers());
    this.provider.awareness.on('change', handler);
    return () => this.provider?.awareness.off('change', handler);
  }

  /**
   * Subscribe to local RuSheet events and sync to Yjs
   */
  private subscribeToLocalEvents(): void {
    // Cell value changes
    const unsubChange = rusheet.onChange((event) => {
      if (event.source === 'api') return; // Skip remote-applied changes
      this.syncCellToYjs(event);
    });
    this.unsubscribers.push(unsubChange);

    // Selection changes (for cursor awareness)
    const unsubSelection = rusheet.onSelectionChange((event) => {
      this.updateCursor(event.row, event.col);
    });
    this.unsubscribers.push(unsubSelection);

    // Format changes
    const unsubFormat = rusheet.onFormatChange((event) => {
      if (event.source === 'api') return;
      this.syncFormatToYjs(event);
    });
    this.unsubscribers.push(unsubFormat);

    // Sheet operations
    const unsubSheetAdd = rusheet.onSheetAdd((event) => {
      if (event.source === 'api') return;
      this.doc.transact(() => {
        this.sheets.push([event.name]);
      });
    });
    this.unsubscribers.push(unsubSheetAdd);

    const unsubSheetDelete = rusheet.onSheetDelete((event) => {
      if (event.source === 'api') return;
      this.doc.transact(() => {
        this.sheets.delete(event.index, 1);
      });
    });
    this.unsubscribers.push(unsubSheetDelete);

    const unsubSheetRename = rusheet.onSheetRename((event) => {
      if (event.source === 'api') return;
      this.doc.transact(() => {
        this.sheets.delete(event.index, 1);
        this.sheets.insert(event.index, [event.newName]);
      });
    });
    this.unsubscribers.push(unsubSheetRename);

    // Row/Column operations
    const unsubRowInsert = rusheet.onRowsInsert((event) => {
      if (event.source === 'api') return;
      this.syncRowInsertToYjs(event.atRow, event.count);
    });
    this.unsubscribers.push(unsubRowInsert);

    const unsubRowDelete = rusheet.onRowsDelete((event) => {
      if (event.source === 'api') return;
      this.syncRowDeleteToYjs(event.atRow, event.count);
    });
    this.unsubscribers.push(unsubRowDelete);

    const unsubColInsert = rusheet.onColsInsert((event) => {
      if (event.source === 'api') return;
      this.syncColInsertToYjs(event.atCol, event.count);
    });
    this.unsubscribers.push(unsubColInsert);

    const unsubColDelete = rusheet.onColsDelete((event) => {
      if (event.source === 'api') return;
      this.syncColDeleteToYjs(event.atCol, event.count);
    });
    this.unsubscribers.push(unsubColDelete);
  }

  /**
   * Sync a cell change to Yjs
   */
  private syncCellToYjs(event: CellChangeEvent): void {
    const sheetIndex = rusheet.getActiveSheetIndex();
    const key = `${sheetIndex}:${event.row},${event.col}`;

    this.doc.transact(() => {
      if (event.newValue === null || event.newValue === '') {
        this.cells.delete(key);
      } else {
        let cellMap = this.cells.get(key);
        if (!cellMap) {
          cellMap = new Y.Map();
          this.cells.set(key, cellMap);
        }
        cellMap.set('value', event.newValue);

        // Check if it's a formula
        if (event.newValue.startsWith('=')) {
          cellMap.set('formula', event.newValue);
        } else {
          cellMap.delete('formula');
        }
      }
    });
  }

  /**
   * Sync format change to Yjs
   */
  private syncFormatToYjs(event: {
    startRow: number; startCol: number;
    endRow: number; endCol: number;
    format: CellFormat
  }): void {
    const sheetIndex = rusheet.getActiveSheetIndex();

    this.doc.transact(() => {
      for (let row = event.startRow; row <= event.endRow; row++) {
        for (let col = event.startCol; col <= event.endCol; col++) {
          const key = `${sheetIndex}:${row},${col}`;
          let cellMap = this.cells.get(key);
          if (!cellMap) {
            cellMap = new Y.Map();
            this.cells.set(key, cellMap);
          }
          cellMap.set('format', event.format);
        }
      }
    });
  }

  /**
   * Sync row insert to Yjs (shift existing cells down)
   */
  private syncRowInsertToYjs(atRow: number, count: number): void {
    const sheetIndex = rusheet.getActiveSheetIndex();
    const prefix = `${sheetIndex}:`;

    this.doc.transact(() => {
      // Collect cells that need to be shifted
      const toMove: [string, Y.Map<unknown>][] = [];

      this.cells.forEach((cellMap, key) => {
        if (!key.startsWith(prefix)) return;
        const [, coords] = key.split(':');
        const [rowStr] = coords.split(',');
        const row = Number(rowStr);

        if (row >= atRow) {
          toMove.push([key, cellMap.clone()]);
        }
      });

      // Delete old keys and insert at new positions
      toMove.forEach(([key]) => this.cells.delete(key));
      toMove.forEach(([key, cellMap]) => {
        const [, coords] = key.split(':');
        const parts = coords.split(',').map(Number);
        const newKey = `${sheetIndex}:${parts[0] + count},${parts[1]}`;
        this.cells.set(newKey, cellMap);
      });
    });
  }

  /**
   * Sync row delete to Yjs (shift cells up)
   */
  private syncRowDeleteToYjs(atRow: number, count: number): void {
    const sheetIndex = rusheet.getActiveSheetIndex();
    const prefix = `${sheetIndex}:`;

    this.doc.transact(() => {
      const toDelete: string[] = [];
      const toMove: [string, Y.Map<unknown>][] = [];

      this.cells.forEach((cellMap, key) => {
        if (!key.startsWith(prefix)) return;
        const [, coords] = key.split(':');
        const [rowStr] = coords.split(',');
        const row = Number(rowStr);

        if (row >= atRow && row < atRow + count) {
          toDelete.push(key);
        } else if (row >= atRow + count) {
          toMove.push([key, cellMap.clone()]);
        }
      });

      toDelete.forEach(key => this.cells.delete(key));
      toMove.forEach(([key]) => this.cells.delete(key));
      toMove.forEach(([key, cellMap]) => {
        const [, coords] = key.split(':');
        const parts = coords.split(',').map(Number);
        const newKey = `${sheetIndex}:${parts[0] - count},${parts[1]}`;
        this.cells.set(newKey, cellMap);
      });
    });
  }

  /**
   * Sync column insert to Yjs
   */
  private syncColInsertToYjs(atCol: number, count: number): void {
    const sheetIndex = rusheet.getActiveSheetIndex();
    const prefix = `${sheetIndex}:`;

    this.doc.transact(() => {
      const toMove: [string, Y.Map<unknown>][] = [];

      this.cells.forEach((cellMap, key) => {
        if (!key.startsWith(prefix)) return;
        const [, coords] = key.split(':');
        const parts = coords.split(',').map(Number);
        const col = parts[1];

        if (col >= atCol) {
          toMove.push([key, cellMap.clone()]);
        }
      });

      toMove.forEach(([key]) => this.cells.delete(key));
      toMove.forEach(([key, cellMap]) => {
        const [, coords] = key.split(':');
        const parts = coords.split(',').map(Number);
        const newKey = `${sheetIndex}:${parts[0]},${parts[1] + count}`;
        this.cells.set(newKey, cellMap);
      });
    });
  }

  /**
   * Sync column delete to Yjs
   */
  private syncColDeleteToYjs(atCol: number, count: number): void {
    const sheetIndex = rusheet.getActiveSheetIndex();
    const prefix = `${sheetIndex}:`;

    this.doc.transact(() => {
      const toDelete: string[] = [];
      const toMove: [string, Y.Map<unknown>][] = [];

      this.cells.forEach((cellMap, key) => {
        if (!key.startsWith(prefix)) return;
        const [, coords] = key.split(':');
        const parts = coords.split(',').map(Number);
        const col = parts[1];

        if (col >= atCol && col < atCol + count) {
          toDelete.push(key);
        } else if (col >= atCol + count) {
          toMove.push([key, cellMap.clone()]);
        }
      });

      toDelete.forEach(key => this.cells.delete(key));
      toMove.forEach(([key]) => this.cells.delete(key));
      toMove.forEach(([key, cellMap]) => {
        const [, coords] = key.split(':');
        const parts = coords.split(',').map(Number);
        const newKey = `${sheetIndex}:${parts[0]},${parts[1] - count}`;
        this.cells.set(newKey, cellMap);
      });
    });
  }

  /**
   * Handle remote changes from Yjs and apply to local state
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private handleRemoteChanges(events: Y.YEvent<any>[]): void {
    this.isApplyingRemote = true;

    try {
      events.forEach(event => {
        if (event.target === this.cells) {
          // Cell map changes
          const mapEvent = event as Y.YMapEvent<Y.Map<unknown>>;

          mapEvent.changes.keys.forEach((change, key) => {
            if (change.action === 'add' || change.action === 'update') {
              const cellMap = this.cells.get(key);
              if (cellMap) {
                this.applyCellFromYjs(key, cellMap);
              }
            } else if (change.action === 'delete') {
              this.deleteCellFromYjs(key);
            }
          });
        }
      });
    } finally {
      this.isApplyingRemote = false;
    }
  }

  /**
   * Apply a cell from Yjs to local state
   */
  private applyCellFromYjs(key: string, cellMap: Y.Map<unknown>): void {
    const [sheetPart, coordsPart] = key.split(':');
    const sheetIndex = parseInt(sheetPart, 10);
    const [row, col] = coordsPart.split(',').map(Number);

    // Only apply if on the active sheet
    if (sheetIndex !== rusheet.getActiveSheetIndex()) return;

    const value = cellMap.get('value') as string | undefined;
    const format = cellMap.get('format') as CellFormat | undefined;

    if (value !== undefined) {
      WasmBridge.setCellValue(row, col, value);
    }

    if (format) {
      WasmBridge.setCellFormat(row, col, format);
    }
  }

  /**
   * Delete a cell from local state (remote deletion)
   */
  private deleteCellFromYjs(key: string): void {
    const [sheetPart, coordsPart] = key.split(':');
    const sheetIndex = parseInt(sheetPart, 10);
    const [row, col] = coordsPart.split(',').map(Number);

    if (sheetIndex !== rusheet.getActiveSheetIndex()) return;

    WasmBridge.setCellValue(row, col, '');
  }

  /**
   * Load initial state from Yjs document
   */
  private loadFromYjs(): void {
    const sheetIndex = rusheet.getActiveSheetIndex();
    const prefix = `${sheetIndex}:`;

    this.isApplyingRemote = true;
    try {
      this.cells.forEach((cellMap, key) => {
        if (key.startsWith(prefix)) {
          this.applyCellFromYjs(key, cellMap);
        }
      });
    } finally {
      this.isApplyingRemote = false;
    }
  }

  /**
   * Sync current local state to Yjs (for initial sync)
   */
  syncToYjs(): void {
    const sheetIndex = rusheet.getActiveSheetIndex();

    // Get all non-empty cells and sync to Yjs
    this.doc.transact(() => {
      const data = rusheet.getData(0, 999, 0, 25);
      data.forEach((row, rowIdx) => {
        row.forEach((value, colIdx) => {
          if (value !== null && value !== '') {
            const key = `${sheetIndex}:${rowIdx},${colIdx}`;
            let cellMap = this.cells.get(key);
            if (!cellMap) {
              cellMap = new Y.Map();
              this.cells.set(key, cellMap);
            }
            cellMap.set('value', value);
          }
        });
      });
    });
  }

  /**
   * Generate a random color for user
   */
  private generateColor(): string {
    const colors = [
      '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4',
      '#FFEAA7', '#DDA0DD', '#98D8C8', '#F7DC6F',
      '#BB8FCE', '#85C1E9', '#F8B500', '#00CED1',
    ];
    return colors[Math.floor(Math.random() * colors.length)];
  }
}

// Singleton instance
let collabProvider: CollaborationProvider | null = null;

/**
 * Initialize collaboration for a workbook
 */
export function initCollaboration(config: CollaborationConfig): CollaborationProvider {
  if (collabProvider) {
    collabProvider.disconnect();
  }

  collabProvider = new CollaborationProvider(config);
  collabProvider.connect();

  return collabProvider;
}

/**
 * Get the current collaboration provider
 */
export function getCollabProvider(): CollaborationProvider | null {
  return collabProvider;
}

/**
 * Disconnect and cleanup collaboration
 */
export function disconnectCollaboration(): void {
  if (collabProvider) {
    collabProvider.disconnect();
    collabProvider = null;
  }
}
