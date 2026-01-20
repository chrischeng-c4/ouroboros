# Talos vs Vite Benchmark: Excalidraw Project

## Overview

Performance benchmark comparing **Talos bundler** vs **Vite** using the real-world large-scale open-source project **Excalidraw** (77K GitHub stars).

**Test Project**: Excalidraw - Collaborative whiteboard drawing application
**Tech Stack**: React, TypeScript, SCSS
**Scale**: Large monorepo, 630+ modules
**Test Scope**: Entire project
**Comparison Metrics**:
1. Build time (cold start)
2. Build time (hot reload)
3. Bundle size
4. Memory usage

## Prerequisites

- macOS 10.15+ or Linux
- Rust 1.70+
- Node.js 18+
- Yarn 1.22+
- 16GB+ RAM
- 10GB free disk space

## Project Structure

```
benchmarks/excalidraw/
├── README.md                    # This file
├── setup.sh                     # Automated setup script
├── run_benchmark.sh             # Main benchmark executor
├── excalidraw_subset/           # Phase 2: Subset testing (30-50 modules)
│   ├── src/
│   ├── package.json
│   └── vite.config.ts
├── results/                     # Test results
│   ├── vite/
│   │   ├── cold_start.json
│   │   ├── hot_reload.json
│   │   └── bundle_stats.json
│   └── talos/
│       ├── cold_start.json
│       ├── hot_reload.json
│       └── bundle_stats.json
└── tools/
    ├── memory_profiler.sh       # Memory monitoring
    ├── compare_bundles.rs       # Bundle size analysis
    └── generate_report.rs       # Report generator
```

## Quick Start

### Step 1: Setup

```bash
cd /Users/chris.cheng/chris-project/ouroboros-talos/benchmarks/excalidraw
./setup.sh
```

This will:
- Clone Excalidraw to /tmp/excalidraw
- Install dependencies
- Run baseline Vite build
- Record baseline metrics

### Step 2: Run Benchmark

```bash
# Run subset benchmark (Phase 2)
./run_benchmark.sh subset

# Run full benchmark (Phase 3)
./run_benchmark.sh full
```

### Step 3: Generate Report

```bash
cd ../../
cargo run --release --bin compare_results -- \
  --vite benchmarks/excalidraw/results/vite/ \
  --talos benchmarks/excalidraw/results/talos/ \
  --output benchmarks/excalidraw/results/BENCHMARK_REPORT.md
```

## Methodology

### Fair Comparison Measures

1. **Minification**: DISABLED for both bundlers
2. **Source Maps**: DISABLED for both
3. **Code Splitting**: DISABLED (single bundle)
4. **External Dependencies**: Same configuration (react, react-dom)

### Known Limitations

**Talos does not implement**:
- Minification
- Tree shaking
- Code splitting
- Fine-grained HMR

**Vite features disabled for parity**:
- `build.minify = false`
- `build.sourcemap = false`
- `build.rollupOptions.output.manualChunks = undefined`

## Test Environment

- **OS**: macOS 23.6.0 (Darwin)
- **CPU**: Apple M1/M2
- **Memory**: 16 GB
- **Vite**: v5.x.x
- **Talos**: commit b0528ed
- **Project**: Excalidraw (latest)
- **Module Count**: 630 files

## Expected Run Time

- Setup: 30 minutes
- Subset benchmark: 4-6 hours
- Full benchmark: 6-8 hours
- Total: ~12-15 hours

## Interpreting Results

- **Speedup > 1.0**: Talos is faster
- **Speedup < 1.0**: Vite is faster
- **p < 0.05**: Statistically significant difference

## Incremental Validation

This benchmark follows a **progressive validation** approach:

1. **Phase 1**: Setup (2-3h)
2. **Phase 2**: Subset Benchmark (4-6h) [30-50 modules]
   - **Validation Checkpoint**: If subset fails, diagnose before proceeding
3. **Phase 3**: Full Benchmark (6-8h) [630+ modules]
4. **Phase 4**: Statistical Analysis (2-3h)
5. **Phase 5**: Documentation (1-2h)

## Troubleshooting

### Build Fails

Check logs in `results/vite/*.log` or `results/talos/*.log`

### Memory Issues

Increase ulimit: `ulimit -n 10000`

### Vite Version Mismatch

Ensure Excalidraw uses Vite 5.x

## References

- [Excalidraw GitHub](https://github.com/excalidraw/excalidraw)
- [Vite Documentation](https://vitejs.dev/)
- [Talos Bundler](https://github.com/your-org/ouroboros-talos)
