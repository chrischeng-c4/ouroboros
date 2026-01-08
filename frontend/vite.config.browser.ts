import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
import { playwright } from '@vitest/browser-playwright';

export default defineConfig({
  plugins: [
    wasm(),
    topLevelAwait(),
  ],
  build: {
    target: 'esnext',
  },
  optimizeDeps: {
    exclude: ['data-bridge-sheet-wasm'],
  },
  resolve: {
    alias: {
      '@': '/src',
    },
  },
  test: {
    globals: true,
    include: ['**/*.integration.test.ts'],
    browser: {
      enabled: true,
      provider: playwright(),
      instances: [{ browser: 'chromium' }],
      headless: true,
    },
    deps: {
      inline: ['data-bridge-sheet-wasm'],
    },
  },
  server: {
    fs: {
      allow: ['..'],
    },
  },
});
