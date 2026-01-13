# CLAUDE.md - Implementation Essentials

## Build Commands

```bash
# Rust build
maturin develop                      # Debug build
maturin develop --release            # Release build (optimized)

# Rust tests & checks
cargo test                           # All Rust tests
cargo test -p data-bridge-mongodb    # MongoDB crate only
cargo clippy                         # Lint check
cargo audit                          # Security audit

# Python tests (use data-bridge-test, NOT pytest)
uv run python tests/unit/test_*.py                   # Unit tests (no MongoDB)
uv run python tests/integration/test_*.py            # Integration tests (MongoDB required)
uv run python tests/api/test_*.py                    # API tests

# Performance benchmarks
uv run python benchmarks/bench_comparison.py --rounds 5 --warmup 2
uv run python tests/api/benchmarks/bench_comparison_rust.py --rounds 5 --warmup 2
```

## Quick Reference

**Development Workflow:**
1. Understand requirements and create plan
2. Implement feature/fix
3. Run tests (cargo test + data-bridge-test)
4. Run benchmarks (if performance-related)
5. Create commit and PR

**Build Cycle:**
```bash
maturin develop                    # Build Python extension
cargo test                         # Rust tests
uv run python tests/unit/test_*.py # Python tests (data-bridge-test)
cargo clippy                       # Lint check
```

**Test Modes:**
- Unit: `uv run python tests/unit/test_*.py`
- Integration: `uv run python tests/integration/test_*.py`
- API: `uv run python tests/api/test_*.py`
- Benchmarks: `uv run python benchmarks/bench_*.py --rounds 5 --warmup 2`

## Test Framework

**Use data-bridge-test (NOT pytest) for all new tests.**

Basic pattern:
```python
from data_bridge_test import TestSuite, test, expect

class MyTests(TestSuite):
    @test
    def test_feature(self):
        result = some_operation()
        expect(result).to_equal(expected)

if __name__ == "__main__":
    MyTests().run()
```

Run: `uv run python tests/unit/test_*.py`

## Commit Format

```
feat(NNN): add email/url constraint validation
fix(NNN): correct GIL release in bulk operations
test(NNN): add complex type validation tests
perf(NNN): optimize bulk insert for 50K+ documents
```

## Pre-commit Checklist

**Code Quality:**
- [ ] cargo build passes
- [ ] cargo test passes
- [ ] cargo clippy clean
- [ ] No unwrap() in production code
- [ ] Proper error handling (thiserror)
- [ ] GIL released during expensive operations

**Testing:**
- [ ] Rust unit tests written and passing
- [ ] Python unit tests written (data-bridge-test)
- [ ] Integration tests cover CRUD lifecycle
- [ ] Edge cases covered

**Performance (if relevant):**
- [ ] Benchmarks run and results recorded
- [ ] Performance targets met
- [ ] No regression vs previous version

**Security:**
- [ ] Input validation at PyO3 boundary
- [ ] Collection/field name validation
- [ ] No unwrap() that could panic Python

**Commit:**
- [ ] Commit message format correct: `feat(NNN):` or `fix(NNN):`
- [ ] Changes focused (not mixed features)
- [ ] Documentation updated if needed
