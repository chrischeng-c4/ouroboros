import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

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
    environment: 'happy-dom',
    setupFiles: ['./src/__tests__/setup.ts'],
    include: ['src/**/*.{test,spec}.ts', 'src/**/*.integration.test.ts'],
    exclude: ['**/node_modules/**', 'e2e/**', '**/e2e/**', 'example/**'],
    pool: 'forks',
    isolate: true,
    // Vitest 4: poolOptions moved to top-level
    singleFork: true,
    server: {
      deps: {
        inline: ['data-bridge-sheet-wasm'],
      },
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'text-summary', 'html', 'json', 'lcov'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: ['node_modules/', 'src/__tests__/**', '**/*.test.ts', '**/*.spec.ts', '**/*.integration.test.ts', 'src/main.ts', 'pkg/', '.vite/'],
      thresholds: { branches: 60, functions: 60, lines: 60, statements: 60 },
    },
  },
  server: {
    fs: {
      allow: ['..'],
    },
  },
});
