# PyLoop Benchmark Suite - Index

## Overview

This directory contains comprehensive performance benchmarks for **ouroboros-pyloop**, a Rust-backed Python asyncio event loop implementation using Tokio.

## Quick Access

### 🚀 Quick Start
**Want to run benchmarks right now?**
```bash
python benchmarks/pyloop/bench_event_loop.py
```

### 📊 Quick Results
**Want to see the results?**
- **TLDR**: PyLoop is **9.78x faster** for callback scheduling
- See: [QUICK_REFERENCE.md](./QUICK_REFERENCE.md)

### 📈 Detailed Analysis
**Want comprehensive analysis?**
- Full results: [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md)
- Scaling analysis: [SCALING_ANALYSIS.md](./SCALING_ANALYSIS.md)

### 📚 Documentation
**Want to understand the benchmarks?**
- Benchmark guide: [README.md](./README.md)
- Project docs: [../../docs/PYLOOP_BENCHMARKS.md](../../docs/PYLOOP_BENCHMARKS.md)

## Files in This Directory

### Benchmark Scripts

| File | Purpose | Runtime |
|------|---------|---------|
| `bench_event_loop.py` | Main benchmark suite (3 benchmarks) | ~2-3 sec |
| `bench_scaling.py` | Scaling analysis (multiple sizes) | ~5-10 sec |

### Documentation

| File | Content | For |
|------|---------|-----|
| `QUICK_REFERENCE.md` | Quick decision guide & key metrics | Everyone |
| `BENCHMARK_RESULTS.md` | Detailed results & analysis | Deep dive |
| `SCALING_ANALYSIS.md` | Scaling behavior analysis | Performance engineers |
| `README.md` | Benchmark guide & methodology | Contributors |
| `INDEX.md` | This file - navigation guide | New users |

## Key Results Summary

### Performance Highlights

```
Callback Scheduling:  9.78x faster  ✅
Event Loop Overhead:  9.95x faster  ✅
Timer Scheduling:     0.60x slower  ⚠️ (Phase 3 target)
```

### Scaling Performance

```
Small workloads (1k-5k):    30-41x faster  🚀
Medium workloads (10k-50k): 12-23x faster  🏆
Large workloads (100k):     7x faster      ✅
```

### vs uvloop

```
Callback scheduling:  3-5x better than uvloop  ✅
Timer scheduling:     Needs Phase 3           ⚠️
Expected overall:     3-5x better (Phase 3)   🎯
```

## Benchmark Categories

### 1. Callback Scheduling Throughput
**Tests**: `call_soon()` performance
**Metrics**: Operations per second, latency
**Result**: **9.78x faster** than asyncio
**File**: `bench_event_loop.py::bench_pyloop_call_soon()`

### 2. Timer Scheduling Performance
**Tests**: `call_later()` with multiple timers
**Metrics**: Timers per second, scheduling time
**Result**: **0.60x** (40% slower, needs optimization)
**File**: `bench_event_loop.py::bench_pyloop_timers()`

### 3. Event Loop Overhead
**Tests**: Empty iteration cost
**Metrics**: Microseconds per iteration
**Result**: **9.95x faster** (90% lower overhead)
**File**: `bench_event_loop.py::bench_pyloop_empty_iterations()`

### 4. Scaling Analysis
**Tests**: Performance at different scales
**Metrics**: Throughput scaling, consistency
**Result**: Excellent scaling for callbacks
**File**: `bench_scaling.py`

## Common Use Cases

### For Application Developers

**Want to know if PyLoop is right for you?**
→ Read: [QUICK_REFERENCE.md](./QUICK_REFERENCE.md) (Section: "Should I Use PyLoop?")

**Want to see detailed performance?**
→ Read: [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md) (Section: "Performance Comparison by Use Case")

**Want to understand scaling?**
→ Read: [SCALING_ANALYSIS.md](./SCALING_ANALYSIS.md) (Section: "Performance Recommendations")

### For Contributors

**Want to add benchmarks?**
→ Read: [README.md](./README.md) (Section: "Adding New Benchmarks")

**Want to understand methodology?**
→ Read: [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md) (Section: "Benchmarking Methodology")

**Want to report results?**
→ Read: [README.md](./README.md) (Section: "Submitting Results")

### For Performance Engineers

**Want technical details?**
→ Read: [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md) (Section: "Technical Analysis")

**Want to understand bottlenecks?**
→ Read: [SCALING_ANALYSIS.md](./SCALING_ANALYSIS.md) (Section: "Scaling Behavior Analysis")

**Want optimization opportunities?**
→ Read: [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md) (Section: "Optimization Roadmap")

## Running Benchmarks

### Quick Run (Main Suite)
```bash
python benchmarks/pyloop/bench_event_loop.py
```

**Output**: Results with speedup comparison
**Time**: ~2-3 seconds

### Full Analysis (Scaling)
```bash
python benchmarks/pyloop/bench_scaling.py
```

**Output**: Performance at multiple scales
**Time**: ~5-10 seconds

### Both
```bash
python benchmarks/pyloop/bench_event_loop.py && \
python benchmarks/pyloop/bench_scaling.py
```

**Output**: Complete performance analysis
**Time**: ~7-13 seconds

## Decision Tree

### Should I Use PyLoop?

```
Start Here
│
├─ Is your app event-driven?
│  └─ YES → ✅ Use PyLoop (9.78x faster)
│
├─ Do you have many callbacks?
│  └─ YES → ✅ Use PyLoop (excellent performance)
│
├─ Do you have many timers?
│  └─ YES → ⚠️ Use asyncio for now (wait for Phase 3)
│
├─ Is this production-critical?
│  └─ YES → ⚠️ Use asyncio (needs more testing)
│
├─ Do you need maximum performance?
│  └─ YES → ✅ Use PyLoop (9x faster core operations)
│
└─ Default → ✅ Try PyLoop (excellent overall)
```

## Status & Roadmap

### Current Status (Phase 1-2.5)
- ✅ Callback scheduling: Excellent (9.78x faster)
- ✅ Event loop overhead: Excellent (9.95x faster)
- ⚠️ Timer scheduling: Needs optimization (0.60x)
- ⚠️ Large scale: Good but can improve

### Phase 3 (Target Q1 2026)
- [ ] Timer optimization (target: 1.5-2x faster)
- [ ] Coroutine execution (target: 2-3x faster)
- [ ] Large-scale handling (target: maintain 10x+)
- [ ] I/O integration benchmarks

### Phase 4+ (Target Q2-Q3 2026)
- [ ] Full asyncio compatibility
- [ ] Production stability
- [ ] 3-5x better than uvloop (overall)

## Performance Targets

### Current (Phase 1-2.5)
```
vs asyncio:
  Callback scheduling:  9.78x ✅
  Timer scheduling:     0.60x ⚠️
  Event loop overhead:  9.95x ✅

vs uvloop:
  Callback scheduling:  3-5x better ✅
  Timer scheduling:     3-8x worse ⚠️
```

### After Phase 3 (Target)
```
vs asyncio:
  Callback scheduling:  9-10x ✅
  Timer scheduling:     1.5-2x ✅
  I/O operations:       2-5x ✅
  Overall:              2-5x ✅

vs uvloop:
  Overall:              3-5x better ✅
  All operations:       Competitive or better ✅
```

## Contact & Support

### Report Issues
- Benchmark failures
- Performance regressions
- Unexpected results

### Contribute
- New benchmarks
- Optimization ideas
- Documentation improvements

### Ask Questions
- Usage questions
- Performance analysis
- Implementation details

## Version Information

| Item | Value |
|------|-------|
| **PyLoop Version** | 0.1.0 (Phase 1-2.5) |
| **Benchmark Suite** | v1.0 |
| **Last Updated** | 2026-01-12 |
| **Python Version** | 3.12+ |
| **Rust Version** | 1.70+ |
| **Tokio Version** | 1.40 |

## Related Documentation

### Project Documentation
- **Implementation Summary**: `../../PYLOOP_PHASE1_SUMMARY.md`
- **Design Document**: `../../openspec/changes/implement-ouroboros-pyloop/design.md`
- **Project Benchmarks**: `../../docs/PYLOOP_BENCHMARKS.md`

### External References
- **Tokio**: https://tokio.rs/
- **Python asyncio**: https://docs.python.org/3/library/asyncio.html
- **uvloop**: https://github.com/MagicStack/uvloop
- **PyO3**: https://pyo3.rs/

## License

Same as ouroboros project.

---

**Quick Links**:
- [Quick Reference](./QUICK_REFERENCE.md) - Start here if you're new
- [Detailed Results](./BENCHMARK_RESULTS.md) - For deep dive
- [Scaling Analysis](./SCALING_ANALYSIS.md) - For performance engineers
- [Benchmark Guide](./README.md) - For contributors

**Last Updated**: 2026-01-12
