import { defineConfig } from 'vite';
import { resolve } from 'path';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'DataBridgeSheet',
      formats: ['es', 'umd'],
      fileName: (format) => `data-bridge-sheet.${format}.js`,
    },
    rollupOptions: {
      // No external dependencies for WASM library
      external: [],
      output: {
        globals: {},
      },
    },
    outDir: 'dist',
    emptyOutDir: false,
    sourcemap: true,
  },
});
