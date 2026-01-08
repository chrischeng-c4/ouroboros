/**
 * Worker module exports
 */

export { RenderController } from './RenderController';
export type * from './types';

/**
 * Check if OffscreenCanvas is supported in the current browser.
 */
export function isOffscreenCanvasSupported(): boolean {
  return (
    typeof OffscreenCanvas !== 'undefined' &&
    typeof HTMLCanvasElement.prototype.transferControlToOffscreen === 'function'
  );
}
