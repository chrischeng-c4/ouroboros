import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import CellEditor from '../CellEditor';
import GridRenderer from '../../canvas/GridRenderer';

/**
 * UNIT TESTS (with mocks for fast execution)
 *
 * NOTE: These tests use mocks for fast unit testing of isolated logic.
 * For comprehensive integration tests with REAL components, see:
 * - CellEditor.integration.test.ts (CellEditor + GridRenderer + WASM integration)
 * - EditWorkflow.integration.test.ts (Complete user workflows)
 * - specifications/CellEditingSpec.test.ts (Specification/behavior tests)
 *
 * The integration tests are the PRIMARY tests - they would have caught
 * the renderer.render() bug. These mocked tests are secondary smoke tests.
 */

// Mock rusheet API
vi.mock('../../core/RusheetAPI', () => ({
  rusheet: {
    getCellData: vi.fn(() => ({ value: '', formula: '', displayValue: '' })),
    setCellValue: vi.fn(),
    getColWidth: vi.fn(() => 100),
    getRowHeight: vi.fn(() => 25),
    emitCellEdit: vi.fn(),
  },
}));

// Mock GridRenderer
vi.mock('../../canvas/GridRenderer', () => ({
  default: class MockGridRenderer {
    getActiveCell() {
      return { row: 0, col: 0 };
    }
    setActiveCell = vi.fn()
    gridToScreen() {
      return { x: 0, y: 0 };
    }
    render = vi.fn()
  }
}));

describe('CellEditor - Unit Tests (Mocked)', () => {
  let cellEditor: CellEditor;
  let container: HTMLElement;
  let formulaBar: HTMLInputElement;
  let renderer: GridRenderer;

  beforeEach(() => {
    vi.clearAllMocks();

    container = document.createElement('div');
    formulaBar = document.createElement('input');
    document.body.appendChild(container);
    document.body.appendChild(formulaBar);

    renderer = new GridRenderer(document.createElement('canvas'));
    cellEditor = new CellEditor(container, renderer, formulaBar);
  });

  afterEach(() => {
    document.body.removeChild(container);
    document.body.removeChild(formulaBar);
  });

  describe('Basic DOM Interaction (Mocked)', () => {
    it('should create textarea on activation', () => {
      cellEditor.activate(0, 0);

      const textarea = container.querySelector('textarea');
      expect(textarea).toBeTruthy();
      expect(textarea?.style.display).toBe('block');
    });

    it('should sync textarea with formula bar on input', () => {
      cellEditor.activate(0, 0);
      const textarea = container.querySelector('textarea');

      if (textarea) {
        textarea.value = 'Hello';
        textarea.dispatchEvent(new Event('input', { bubbles: true }));

        expect(formulaBar.value).toBe('Hello');
      }
    });

    it('should not prevent regular keyboard input', () => {
      cellEditor.activate(0, 0);
      const textarea = container.querySelector('textarea');

      if (textarea) {
        const event = new KeyboardEvent('keydown', {
          key: 'x',
          cancelable: true
        });

        const prevented = !textarea.dispatchEvent(event);
        expect(prevented).toBe(false);
      }
    });
  });
});
