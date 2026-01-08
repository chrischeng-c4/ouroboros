# Data Bridge Sheet - Frontend Build Guide

This guide explains how to build and develop the Data Bridge Sheet frontend application.

## Prerequisites

- Node.js 18+ (with pnpm)
- Rust toolchain (with wasm-pack)
- MongoDB (for backend services)

## Project Structure

```
frontend/
├── src/                    # TypeScript source code
│   ├── core/              # Core functionality & WASM bridge
│   ├── canvas/            # Canvas rendering
│   ├── ui/                # UI components
│   ├── collab/            # Collaboration features
│   └── worker/            # Web workers
├── pkg/                   # WASM output (generated)
├── dist/                  # Build output (generated)
└── vite.config*.ts        # Vite configurations
```

## Build Steps

### 1. Build WASM Module

The WASM module must be built before the frontend can run.

```bash
# From the frontend directory
cd frontend

# Build WASM module (development)
pnpm run build:wasm

# Or build directly with wasm-pack
cd ..
wasm-pack build crates/data-bridge-sheet-wasm --target web --out-dir ../frontend/pkg
```

This will:
- Compile the Rust code in `crates/data-bridge-sheet-wasm`
- Generate WASM bindings
- Output to `frontend/pkg/` directory

**Output files:**
- `pkg/data_bridge_sheet_wasm.js` - JavaScript bindings
- `pkg/data_bridge_sheet_wasm_bg.wasm` - WebAssembly binary
- `pkg/data_bridge_sheet_wasm.d.ts` - TypeScript definitions

### 2. Install Dependencies

```bash
# From the frontend directory
pnpm install
```

### 3. Development Server

```bash
# Start Vite dev server
pnpm run dev
```

This will:
- Start the development server on http://localhost:5173
- Enable hot module replacement
- Load WASM from `pkg/` directory

### 4. Build for Production

```bash
# Build everything (WASM + frontend)
pnpm run build

# Or build library version
pnpm run build:lib
```

**Output:**
- `dist/data-bridge-sheet.es.js` - ES module
- `dist/data-bridge-sheet.umd.js` - UMD module
- `dist/index.d.ts` - TypeScript definitions

## Testing

```bash
# Run all tests
pnpm test

# Run unit tests only (no integration tests)
pnpm test:unit

# Run integration tests (requires browser)
pnpm test:integration

# Run tests in watch mode
pnpm test:watch

# Run with coverage
pnpm test:coverage

# Run E2E tests with Playwright
pnpm test:e2e
```

## WASM Module Import

The frontend imports the WASM module from `pkg/data_bridge_sheet_wasm`:

```typescript
// In src/core/WasmBridge.ts
wasmModule = await import('../../pkg/data_bridge_sheet_wasm');
```

The Vite configuration excludes this from optimization:

```typescript
// vite.config.ts
optimizeDeps: {
  exclude: ['data-bridge-sheet-wasm'],
}
```

## Common Issues

### Issue: "Failed to load WASM module"

**Solution:** Make sure you've built the WASM module first:
```bash
pnpm run build:wasm
```

### Issue: "Cannot find module '../../pkg/data_bridge_sheet_wasm'"

**Solution:** The `pkg/` directory is generated during the WASM build. Run:
```bash
pnpm run build:wasm
```

### Issue: Tests fail to load WASM

**Solution:** The test setup file (`src/__tests__/setup.ts`) handles WASM loading in Node environment. Ensure the `pkg/` directory exists and contains the WASM files.

## Development Workflow

1. **Make Rust changes** in `crates/data-bridge-sheet-*`
2. **Rebuild WASM**: `pnpm run build:wasm`
3. **Vite will auto-reload** the changes
4. **Run tests** to verify: `pnpm test`

## Integration with Data Bridge

This frontend is part of the Data Bridge project. The WASM module integrates with:

- `data-bridge-sheet-core` - Core spreadsheet engine
- `data-bridge-sheet-formula` - Formula parsing and evaluation
- `data-bridge-sheet-history` - Undo/redo functionality
- `data-bridge-sheet-db` - Database persistence layer

## Package Configuration

The package is published as `@data-bridge/sheet`:

```json
{
  "name": "@data-bridge/sheet",
  "main": "./dist/data-bridge-sheet.umd.js",
  "module": "./dist/data-bridge-sheet.es.js",
  "types": "./dist/index.d.ts"
}
```

## Next Steps

- See [CLAUDE.md](/CLAUDE.md) for project architecture
- See [crates/data-bridge-sheet-wasm/README.md] for WASM module details
- Run `pnpm run docs:dev` to view full documentation
