# Frontend Migration Map - Rusheet → Data Bridge Sheet

## Package & Module Names

| Category | Old Name | New Name |
|----------|----------|----------|
| **NPM Package** | `rusheet` | `@data-bridge/sheet` |
| **WASM Crate** | `rusheet-wasm` | `data-bridge-sheet-wasm` |
| **WASM Module (JS)** | `rusheet_wasm` | `data_bridge_sheet_wasm` |
| **Library Name** | `Rusheet` | `DataBridgeSheet` |
| **Author** | RuSheet Contributors | Data Bridge Contributors |

## File Paths

### WASM Build Paths

```diff
# Old
- crates/rusheet-wasm/
- pkg/rusheet_wasm.js
- pkg/rusheet_wasm_bg.wasm

# New
+ crates/data-bridge-sheet-wasm/
+ frontend/pkg/data_bridge_sheet_wasm.js
+ frontend/pkg/data_bridge_sheet_wasm_bg.wasm
```

### Build Commands

```diff
# Old
- wasm-pack build crates/rusheet-wasm --target web --out-dir ../../pkg

# New
+ wasm-pack build crates/data-bridge-sheet-wasm --target web --out-dir ../../frontend/pkg
```

## Configuration Changes

### package.json

```diff
{
-  "name": "rusheet",
+  "name": "@data-bridge/sheet",
-  "author": "RuSheet Contributors",
+  "author": "Data Bridge Contributors",
-  "main": "./dist/rusheet.umd.js",
-  "module": "./dist/rusheet.es.js",
+  "main": "./dist/data-bridge-sheet.umd.js",
+  "module": "./dist/data-bridge-sheet.es.js",
  "scripts": {
-    "build:wasm": "wasm-pack build crates/rusheet-wasm --target web --out-dir ../../pkg",
+    "build:wasm": "wasm-pack build ../crates/data-bridge-sheet-wasm --target web --out-dir ../../frontend/pkg",
  }
}
```

### vite.config.ts

```diff
export default defineConfig({
  optimizeDeps: {
-    exclude: ['rusheet-wasm'],
+    exclude: ['data-bridge-sheet-wasm'],
  },
  test: {
    server: {
      deps: {
-        inline: ['rusheet-wasm'],
+        inline: ['data-bridge-sheet-wasm'],
      },
    },
  },
});
```

### vite.config.lib.ts

```diff
export default defineConfig({
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
-      name: 'Rusheet',
+      name: 'DataBridgeSheet',
      formats: ['es', 'umd'],
-      fileName: (format) => `rusheet.${format}.js`,
+      fileName: (format) => `data-bridge-sheet.${format}.js`,
    },
  },
});
```

## TypeScript Changes

### WasmBridge.ts

```diff
// Dynamic import for WASM module
- let wasmModule: typeof import('../../pkg/rusheet_wasm') | null = null;
- let engine: InstanceType<typeof import('../../pkg/rusheet_wasm').SpreadsheetEngine> | null = null;
+ let wasmModule: typeof import('../../pkg/data_bridge_sheet_wasm') | null = null;
+ let engine: InstanceType<typeof import('../../pkg/data_bridge_sheet_wasm').SpreadsheetEngine> | null = null;

export async function initWasm(): Promise<void> {
  if (wasmModule) return;
  try {
-    wasmModule = await import('../../pkg/rusheet_wasm');
+    wasmModule = await import('../../pkg/data_bridge_sheet_wasm');
    await wasmModule.default();
    engine = new wasmModule.SpreadsheetEngine();
  }
}
```

### Test Setup (setup.ts)

```diff
globalThis.fetch = async (url: string | URL | Request, init?: RequestInit) => {
  const urlString = typeof url === 'string' ? url : url.toString();

  // Intercept WASM file requests
-  if (urlString.includes('.wasm') || urlString.includes('rusheet_wasm_bg')) {
-    const wasmPath = join(process.cwd(), 'pkg', 'rusheet_wasm_bg.wasm');
+  if (urlString.includes('.wasm') || urlString.includes('data_bridge_sheet_wasm_bg')) {
+    const wasmPath = join(process.cwd(), 'pkg', 'data_bridge_sheet_wasm_bg.wasm');
  }
};
```

## Build Output

### Distribution Files

```diff
dist/
- ├── rusheet.umd.js
- ├── rusheet.es.js
+ ├── data-bridge-sheet.umd.js
+ ├── data-bridge-sheet.es.js
  └── index.d.ts
```

## Usage Examples

### Old Usage

```javascript
import Rusheet from 'rusheet';

const sheet = new Rusheet();
```

### New Usage

```javascript
import DataBridgeSheet from '@data-bridge/sheet';

const sheet = new DataBridgeSheet();
```

## Just Commands

### New Frontend Commands

```bash
# Build Commands
just build-wasm                    # Build WASM module
just build-frontend                # Build frontend (WASM + TypeScript)
just build-frontend-lib            # Build library version

# Development
just dev-frontend                  # Start dev server

# Testing
just test-frontend                 # All tests
just test-frontend-unit            # Unit tests only
just test-frontend-integration     # Integration tests
just test-frontend-e2e             # E2E tests

# Cleanup
just clean-frontend                # Clean frontend artifacts
just clean-all                     # Clean backend + frontend
```

## Integration with Data Bridge

### Crate Dependencies

```
data-bridge-sheet-wasm
├── data-bridge-sheet-core         (Core spreadsheet engine)
├── data-bridge-sheet-formula      (Formula parser/evaluator)
├── data-bridge-sheet-history      (Undo/redo)
└── wasm-bindgen                   (WASM bindings)
```

### Project Structure

```
merge-rusheet/
├── crates/
│   ├── data-bridge/               (Python MongoDB ORM)
│   ├── data-bridge-sheet-core/    (Sheet engine)
│   ├── data-bridge-sheet-wasm/    (WASM bindings)
│   ├── data-bridge-sheet-server/  (WebSocket server)
│   └── ...
└── frontend/
    ├── src/                       (TypeScript frontend)
    └── pkg/                       (WASM output)
```

## Testing Migration

### Verify Build Process

```bash
# 1. Clean everything
just clean-all

# 2. Build WASM
just build-wasm

# 3. Verify output
ls -la frontend/pkg/
# Should see:
# - data_bridge_sheet_wasm.js
# - data_bridge_sheet_wasm_bg.wasm
# - data_bridge_sheet_wasm.d.ts

# 4. Test frontend
cd frontend && pnpm install && pnpm test:unit
```

## Rollback Instructions

If you need to rollback these changes:

1. **Revert configuration files:**
   ```bash
   git checkout HEAD -- frontend/package.json
   git checkout HEAD -- frontend/vite.config*.ts
   git checkout HEAD -- frontend/src/core/WasmBridge.ts
   git checkout HEAD -- frontend/src/__tests__/setup.ts
   ```

2. **Restore old build paths:**
   - Update `package.json` build:wasm script
   - Point back to old crate location

3. **Remove new documentation:**
   ```bash
   rm frontend/BUILD.md
   rm frontend/QUICKSTART.md
   rm PHASE5_SUMMARY.md
   rm MIGRATION_MAP.md
   ```

## Success Indicators

✅ All frontend configuration files updated
✅ WASM module imports use new paths
✅ No references to old `rusheet-wasm` name
✅ Build scripts point to correct crate location
✅ Just commands available for frontend builds
✅ Documentation created (BUILD.md, QUICKSTART.md)
✅ Tests can load WASM from new location

## Notes

- The crate name uses hyphens: `data-bridge-sheet-wasm`
- The generated module uses underscores: `data_bridge_sheet_wasm`
- TypeScript imports use the underscore version
- All paths are relative to frontend directory
- No changes needed to TypeScript configuration
- All existing functionality preserved
