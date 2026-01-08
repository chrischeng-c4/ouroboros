import * as WasmBridge from '../../../core/WasmBridge';
import GridRenderer from '../../../canvas/GridRenderer';
import CellEditor from '../../CellEditor';
import InputController from '../../InputController';

/**
 * Create a complete test environment with real components (not mocked)
 * This setup uses actual WASM, GridRenderer, and CellEditor instances
 */
export async function createTestEnvironment() {
  // Initialize REAL WASM module
  await WasmBridge.initWasm();

  // Create REAL canvas element
  const canvas = document.createElement('canvas');
  canvas.width = 800;
  canvas.height = 600;
  document.body.appendChild(canvas);

  // Create REAL GridRenderer
  const renderer = new GridRenderer(canvas);

  // Create REAL CellEditor components
  const container = document.createElement('div');
  const formulaBar = document.createElement('input');
  document.body.appendChild(container);
  document.body.appendChild(formulaBar);

  const cellEditor = new CellEditor(container, renderer, formulaBar);

  // Create REAL InputController
  const editCallback = (row: number, col: number) => {
    cellEditor.activate(row, col);
  };
  const inputController = new InputController(canvas, renderer, editCallback);

  return {
    canvas,
    renderer,
    container,
    formulaBar,
    cellEditor,
    inputController,
  };
}

/**
 * Clean up test environment
 */
export function cleanupTestEnvironment(env: {
  canvas: HTMLCanvasElement;
  container: HTMLElement;
  formulaBar: HTMLInputElement;
}) {
  // Clear WASM cell data to prevent test pollution
  // Clear a reasonable range of cells that tests might have used
  WasmBridge.clearRange(0, 0, 20, 20);

  if (env.canvas.parentNode) {
    env.canvas.parentNode.removeChild(env.canvas);
  }
  if (env.container.parentNode) {
    env.container.parentNode.removeChild(env.container);
  }
  if (env.formulaBar.parentNode) {
    env.formulaBar.parentNode.removeChild(env.formulaBar);
  }
}

/**
 * Simulate typing text into an input element
 */
export function simulateTyping(element: HTMLElement, text: string) {
  if (element instanceof HTMLTextAreaElement || element instanceof HTMLInputElement) {
    element.value = text;
    element.dispatchEvent(new Event('input', { bubbles: true }));
  } else {
    throw new Error('Element must be textarea or input');
  }
}

/**
 * Simulate a keyboard key press
 */
export function simulateKeyPress(
  element: HTMLElement,
  key: string,
  options: Partial<KeyboardEventInit> = {}
) {
  const event = new KeyboardEvent('keydown', {
    key,
    bubbles: true,
    cancelable: true,
    ...options,
  });
  element.dispatchEvent(event);
  return event;
}

/**
 * Simulate a mouse click at specific coordinates
 */
export function simulateClick(
  element: HTMLElement,
  x: number,
  y: number,
  options: Partial<MouseEventInit> = {}
) {
  const event = new MouseEvent('click', {
    clientX: x,
    clientY: y,
    bubbles: true,
    cancelable: true,
    ...options,
  });
  element.dispatchEvent(event);
  return event;
}

/**
 * Simulate a double-click at specific coordinates
 */
export function simulateDoubleClick(
  element: HTMLElement,
  x: number,
  y: number,
  options: Partial<MouseEventInit> = {}
) {
  const event = new MouseEvent('dblclick', {
    clientX: x,
    clientY: y,
    bubbles: true,
    cancelable: true,
    ...options,
  });
  element.dispatchEvent(event);
  return event;
}

/**
 * Wait for a specified amount of time (for async operations)
 */
export function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
