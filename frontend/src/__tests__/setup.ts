/**
 * Test Setup File
 *
 * This file is run before all tests to configure the test environment.
 * It creates a lightweight Canvas API mock that's sufficient for testing
 * and configures WASM loading for the Node test environment.
 */

import { readFileSync } from 'fs';
import { join } from 'path';

// Override fetch for WASM loading in Node environment
// Must override even if fetch exists because Node's fetch can't load local files
const originalFetch = globalThis.fetch;

// Disable WebAssembly.instantiateStreaming to force the fallback path
// This is necessary because Node's instantiateStreaming doesn't accept happy-dom Response
// @ts-ignore
globalThis.WebAssembly.instantiateStreaming = undefined;

// @ts-ignore - override for test environment
globalThis.fetch = async (url: string | URL | Request, init?: RequestInit) => {
  const urlString = typeof url === 'string' ? url : url.toString();

  // Intercept WASM file requests
  if (urlString.includes('.wasm') || urlString.includes('data_bridge_sheet_wasm_bg')) {
    const wasmPath = join(process.cwd(), 'pkg', 'data_bridge_sheet_wasm_bg.wasm');
    try {
      const buffer = readFileSync(wasmPath);

      // Convert Node Buffer to proper ArrayBuffer by copying to a new ArrayBuffer
      const arrayBuffer = new ArrayBuffer(buffer.length);
      const view = new Uint8Array(arrayBuffer);
      for (let i = 0; i < buffer.length; i++) {
        view[i] = buffer[i];
      }

      // Create a Response-like object that supports arrayBuffer() for fallback path
      // Note: We use 'cors' type and proper headers to trigger the fallback in wasm-pack
      const blob = new Blob([arrayBuffer], { type: 'application/wasm' });
      const response = new Response(blob, {
        status: 200,
        statusText: 'OK',
        headers: { 'Content-Type': 'application/wasm' },
      });
      // Set the type property that wasm-pack checks
      Object.defineProperty(response, 'type', {
        value: 'basic',
        writable: false,
      });
      return response;
    } catch (error) {
      console.error('Failed to load WASM file from:', wasmPath, error);
      throw error;
    }
  }

  // Fall back to original fetch for other URLs
  if (originalFetch) {
    return originalFetch(url as any, init);
  }

  throw new Error(`Fetch not available for: ${urlString}`);
};

// Create a minimal Canvas 2D Context mock
class MockCanvas2DContext {
  canvas: any;
  fillStyle: string = '#000000';
  strokeStyle: string = '#000000';
  lineWidth: number = 1;
  font: string = '10px sans-serif';
  textAlign: string = 'start';
  textBaseline: string = 'alphabetic';

  constructor(canvas: any) {
    this.canvas = canvas;
  }

  // Drawing methods (no-ops for tests, but prevent errors)
  fillRect() {}
  strokeRect() {}
  clearRect() {}
  fillText() {}
  strokeText() {}
  measureText(text: string) {
    return { width: text.length * 8 }; // Rough approximation
  }
  beginPath() {}
  closePath() {}
  moveTo() {}
  lineTo() {}
  arc() {}
  rect() {}
  clip() {}
  fill() {}
  stroke() {}
  save() {}
  restore() {}
  translate() {}
  rotate() {}
  scale() {}
  setTransform() {}
  drawImage() {}
}

// Polyfill HTMLCanvasElement.getContext() for tests
if (typeof HTMLCanvasElement !== 'undefined') {
  const originalGetContext = HTMLCanvasElement.prototype.getContext;

  HTMLCanvasElement.prototype.getContext = function(contextType: string, options?: any) {
    // Try the original method first (Happy-DOM might support it)
    if (originalGetContext) {
      try {
        const result = originalGetContext.call(this, contextType, options);
        if (result) return result;
      } catch (e) {
        // Ignore errors from original method
      }
    }

    // Fallback to our mock for 2d context
    if (contextType === '2d') {
      return new MockCanvas2DContext(this) as any;
    }

    return null;
  };
}

// Set up global test environment variables
globalThis.TEST_ENV = true;
