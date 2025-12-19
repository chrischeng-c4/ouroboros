# Development Guide

## Quick Start

```bash
# Install dependencies (Rust, uv, just)
just setup

# Check environment
just status

# Build the project
just build-release

# Run all tests
just test-all

# Run benchmarks
just bench-all
```

## Common Commands

### Building

```bash
just build              # Debug build (faster compile, slower runtime)
just build-release      # Release build (optimized, for benchmarking)
just clean              # Clean all build artifacts
```

### Testing

```bash
just test-rust          # Run Rust tests only
just test-python        # Run all Python tests (requires MongoDB)
just test-unit          # Run Python unit tests (no MongoDB)
just test-integration   # Run integration tests (requires MongoDB)
just test-coverage      # Run tests with coverage report
just test-all           # Run all tests (Rust + Python)
```

### Benchmarking

```bash
just bench-insert       # Benchmark insert operations
just bench-find         # Benchmark find operations
just bench-update       # Benchmark update operations
just bench-all          # Run all benchmarks
just bench-comparison   # Compare with Beanie and PyMongo
```

### Code Quality

```bash
just lint               # Run clippy linter
just lint-fix           # Auto-fix clippy issues
just fmt                # Format Rust code
just fmt-check          # Check formatting without changes
just check              # Run all quality checks
just pre-commit         # Pre-commit checks (format + lint + test)
```

### MongoDB

```bash
just mongo-check        # Check if MongoDB is running
just mongo-start        # Start MongoDB (macOS)
just mongo-stop         # Stop MongoDB
just mongo-clean        # Drop all test databases
```

### Development Workflows

```bash
just dev                # Full build + test cycle
just dev-quick          # Quick check (build + Rust tests + unit tests)
just perf-004           # Verify Feature 004 performance improvements
```

## Feature Development (SDD Workflow)

For new features following the Software Design Document (SDD) approach:

```bash
# 1. Create feature directory
just spec-new 201-bulk-optimization

# 2. Run speckit commands (via Claude Code)
/speckit:specify     # Create spec.md
/speckit:plan        # Create plan.md
/speckit:tasks       # Create tasks.md
/speckit:implement   # Implement feature

# 3. Test and verify
just test-all
just bench-all       # For performance features

# 4. Commit
git add .
git commit -m "feat(201): optimize bulk operations"
```

## Performance Testing

### Feature 004: Fast-path Insert

To verify the fast-path insert performance improvements:

```bash
# 1. Ensure MongoDB is running
just mongo-check

# 2. Build in release mode (required for accurate benchmarks)
just build-release

# 3. Run performance verification
just perf-004
```

**Expected Results:**
- `insert_one (fast-path)`: <1.0ms (2x faster than Beanie)
- `bulk_insert (fast-path)`: <15.0ms (3.9x faster than Beanie)

### Running Individual Benchmarks

```bash
# Insert benchmarks (includes fast-path variants)
MONGODB_URI="mongodb://localhost:27017/bench" just bench-insert

# All CRUD operation benchmarks
MONGODB_URI="mongodb://localhost:27017/bench" just bench-all

# Compare with Beanie and PyMongo
just bench-comparison
```

## Testing Strategy

### Rust Tests (No MongoDB Required)

```bash
# All Rust unit tests
just test-rust

# Specific crate
just test-rust-crate data-bridge-mongodb
just test-rust-crate data-bridge-http
```

### Python Tests

```bash
# Unit tests only (no MongoDB, fast)
SKIP_INTEGRATION=true just test-unit

# Integration tests (requires MongoDB)
MONGODB_URI="mongodb://localhost:27017/test" just test-integration

# All tests with coverage
just test-coverage
```

## Environment Setup

### Prerequisites

- **Rust**: `rustup` (install from https://rustup.rs)
- **Python**: 3.12+
- **uv**: Python package manager (install from https://astral.sh/uv)
- **just**: Command runner (install: `cargo install just`)
- **MongoDB**: 3.1+ (macOS: `brew install mongodb-community`)

### First-time Setup

```bash
# Clone repository
git clone <repo-url>
cd data-bridge

# Install all dependencies and tools
just setup

# Start MongoDB
just mongo-start

# Verify everything works
just status
just dev-quick
```

## Common Issues

### MongoDB Connection Failed

```bash
# Check if MongoDB is running
just mongo-check

# Start MongoDB
just mongo-start

# Check MongoDB logs (macOS Homebrew)
tail -f /usr/local/var/log/mongodb/mongo.log
```

### Build Errors

```bash
# Clean and rebuild
just clean
just build-release

# Check Rust toolchain
rustc --version
cargo --version

# Update Rust
rustup update
```

### Test Failures

```bash
# Clean test databases
just mongo-clean

# Run specific test file
uv run pytest tests/mongo/unit/test_document.py -v

# Run with verbose output
just test-python -vv
```

## Performance Targets

data-bridge aims for **1.4-5.4x faster** performance than Beanie:

| Operation | Target | vs Beanie |
|-----------|--------|-----------|
| insert_one (fast-path) | <1.0ms | 2x faster |
| bulk_insert (1000 docs) | <12ms | 5x faster |
| find_many (100 docs) | <2.4ms | 3x faster |
| bulk_update | <4.2ms | 5x faster |

Verify performance after changes:

```bash
just build-release
just bench-all
```

## Contributing

1. Create feature branch: `git checkout -b feature/NNN-name`
2. Follow SDD workflow (see above)
3. Run pre-commit checks: `just pre-commit`
4. Run full test suite: `just test-all`
5. Verify performance (if applicable): `just bench-all`
6. Commit with conventional format: `feat(NNN): description`
7. Create PR targeting `main` branch

## Resources

- [CLAUDE.md](./CLAUDE.md) - AI-assisted development guide
- [ROADMAP.md](./ROADMAP.md) - Feature roadmap (1xx-9xx series)
- [README.md](./README.md) - Project overview and API docs
- [justfile](./justfile) - All available commands

## Quick Reference

```bash
just                    # List all commands
just status             # Show project status
just info               # Show environment info
just --list             # Show all recipes
```
