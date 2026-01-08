/**
 * RuSheet - A high-performance spreadsheet engine
 * Library entry point
 */

// Main API
export { RusheetAPI, rusheet } from './core/RusheetAPI';

// Event types
export type {
  EventSource,
  CellChangeEvent,
  SelectionChangeEvent,
  CellEditEvent,
  FormatChangeEvent,
  SheetAddEvent,
  SheetDeleteEvent,
  SheetRenameEvent,
  ActiveSheetChangeEvent,
  UndoEvent,
  RedoEvent,
  RowsInsertEvent,
  RowsDeleteEvent,
  ColsInsertEvent,
  ColsDeleteEvent,
  DataLoadedEvent,
} from './types/events';

// Data types
export type { CellData, CellFormat, Selection, Viewport } from './types';

// Core utilities (for advanced usage)
export { EventEmitter, emitter } from './core/EventEmitter';

// Low-level WASM bridge (for advanced usage)
export * as WasmBridge from './core/WasmBridge';
