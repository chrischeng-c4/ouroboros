# Frontend Quick Start Guide

## Quick Commands (from project root)

```bash
# Build WASM module
just build-wasm

# Start dev server (auto-builds WASM)
just dev-frontend

# Run tests
just test-frontend-unit

# Build for production
just build-frontend
```

## Manual Build (without just)

```bash
# 1. Build WASM
wasm-pack build crates/data-bridge-sheet-wasm --target web --out-dir ../../frontend/pkg

# 2. Install & run frontend
cd frontend
pnpm install
pnpm run dev
```

## Development Workflow

1. **Make Rust changes** in `crates/data-bridge-sheet-*`
2. **Rebuild WASM**: `just build-wasm`
3. **Frontend auto-reloads** via Vite HMR
4. **Test**: `just test-frontend-unit`

## Common Tasks

| Task | Command |
|------|---------|
| Dev server | `just dev-frontend` |
| Build WASM | `just build-wasm` |
| Run tests | `just test-frontend` |
| Unit tests | `just test-frontend-unit` |
| Integration tests | `just test-frontend-integration` |
| E2E tests | `just test-frontend-e2e` |
| Production build | `just build-frontend` |
| Library build | `just build-frontend-lib` |
| Clean artifacts | `just clean-frontend` |

## File Structure

```
frontend/
├── src/               # TypeScript source
│   ├── core/         # Core logic & WASM bridge
│   ├── canvas/       # Rendering engine
│   └── ui/           # UI components
├── pkg/              # WASM output (generated)
│   ├── data_bridge_sheet_wasm.js
│   ├── data_bridge_sheet_wasm_bg.wasm
│   └── data_bridge_sheet_wasm.d.ts
└── dist/             # Build output (generated)
```

## Troubleshooting

### "Cannot find module '../../pkg/data_bridge_sheet_wasm'"
→ Run `just build-wasm` to generate the WASM module

### "Failed to load WASM module"
→ Ensure `pkg/` directory exists with WASM files

### Tests fail to load WASM
→ The `pkg/` directory must exist before running tests

### Vite dev server won't start
→ Run `pnpm install` in the `frontend/` directory

## Package Info

- **Package name**: `@data-bridge/sheet`
- **WASM module**: `data_bridge_sheet_wasm`
- **Build target**: ES2020 + ESNext
- **Bundle formats**: ES modules + UMD

## NPM Scripts (from frontend/)

```bash
pnpm run dev          # Dev server
pnpm run build        # Build WASM + frontend
pnpm run build:wasm   # Build WASM only
pnpm run build:lib    # Build library version
pnpm test             # All tests
pnpm test:unit        # Unit tests
pnpm test:integration # Integration tests
pnpm test:e2e         # E2E tests
```

## Need More Help?

- See [BUILD.md](./BUILD.md) for detailed build instructions
- See [/CLAUDE.md](/CLAUDE.md) for project architecture
- See [/PHASE5_SUMMARY.md](/PHASE5_SUMMARY.md) for migration details
