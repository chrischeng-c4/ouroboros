import type { CellFormat } from './index';

// Event source type
export type EventSource = 'user' | 'api' | 'undo' | 'redo' | 'system';

// Cell change event (existing, adding timestamp)
export interface CellChangeEvent {
  row: number;
  col: number;
  oldValue: string | null;
  newValue: string | null;
  source: EventSource;
}

// Selection change event (existing)
export interface SelectionChangeEvent {
  row: number;
  col: number;
  previousRow: number;
  previousCol: number;
}

// Cell edit event (existing)
export interface CellEditEvent {
  row: number;
  col: number;
  value: string;
  phase: 'start' | 'change' | 'end' | 'cancel';
}

// Format change event (NEW)
export interface FormatChangeEvent {
  type: 'cell' | 'range';
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
  format: CellFormat;
  source: EventSource;
}

// Sheet events (NEW)
export interface SheetAddEvent {
  index: number;
  name: string;
  source: EventSource;
}

export interface SheetDeleteEvent {
  index: number;
  name: string;
  source: EventSource;
}

export interface SheetRenameEvent {
  index: number;
  oldName: string;
  newName: string;
  source: EventSource;
}

export interface ActiveSheetChangeEvent {
  previousIndex: number;
  newIndex: number;
  previousName: string;
  newName: string;
  source: EventSource;
}

// Undo/Redo events (NEW)
export interface UndoEvent {
  affectedCells: [number, number][];
}

export interface RedoEvent {
  affectedCells: [number, number][];
}

// Row/Column Insert/Delete events (NEW)
export interface RowsInsertEvent {
  atRow: number;
  count: number;
  affectedCells: [number, number][];
  source: EventSource;
}

export interface RowsDeleteEvent {
  atRow: number;
  count: number;
  affectedCells: [number, number][];
  source: EventSource;
}

export interface ColsInsertEvent {
  atCol: number;
  count: number;
  affectedCells: [number, number][];
  source: EventSource;
}

export interface ColsDeleteEvent {
  atCol: number;
  count: number;
  affectedCells: [number, number][];
  source: EventSource;
}

// Sort range event
export interface SortRangeEvent {
  startRow: number;
  endRow: number;
  startCol: number;
  endCol: number;
  sortCol: number;
  ascending: boolean;
  affectedCells: [number, number][];
  source: EventSource;
}

// Merge cells event
export interface MergeCellsEvent {
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
  affectedCells: [number, number][];
  source: EventSource;
}

// Unmerge cells event
export interface UnmergeCellsEvent {
  row: number;
  col: number;
  affectedCells: [number, number][];
  source: EventSource;
}

// Data loaded event
export interface DataLoadedEvent {
  rows: number;
}

// Filter change event
export interface FilterChangeEvent {
  col?: number;
  visibleValues?: string[];
  cleared?: boolean;
  all?: boolean;
  affected: [number, number][];
}
