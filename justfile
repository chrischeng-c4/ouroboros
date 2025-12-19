# justfile for data-bridge development
# Install just: https://github.com/casey/just

# Load environment variables from .env file
set dotenv-load

# Default recipe (list all available commands)
default:
    @just --list

# ============================================================================
# BUILD COMMANDS
# ============================================================================

# Build Rust extension (debug mode)
build:
    uv run maturin develop

# Build Rust extension (release mode, optimized)
build-release:
    uv run maturin develop --release

# Build without installing
build-wheel:
    uv run maturin build --release

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/
    rm -rf .venv/
    rm -rf python/data_bridge/*.so
    find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
    find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true

# ============================================================================
# TEST COMMANDS
# ============================================================================

# Run all Rust tests
test-rust:
    cargo test

# Run Rust tests for specific crate
test-rust-crate CRATE:
    cargo test -p {{CRATE}}

# Run all Python tests (requires MongoDB)
test-python:
    uv run pytest tests/ -v

# Run Python unit tests only (no MongoDB required)
test-unit:
    uv run pytest tests/unit/ -v

# Run Python integration tests (requires MongoDB)
test-integration:
    uv run pytest tests/integration/ tests/mongo/ -v

# Run tests with coverage
test-coverage:
    uv run pytest --cov=data_bridge --cov-report=html --cov-report=term tests/

# Run all tests (Rust + Python)
test-all: test-rust test-python

# ============================================================================
# BENCHMARK COMMANDS
# ============================================================================

# Run insert benchmarks
bench-insert:
    MONGODB_URI="${MONGODB_BENCHMARK_URI}" uv run python -m tests.mongo.benchmarks bench_insert

# Run find benchmarks
bench-find:
    MONGODB_URI="${MONGODB_BENCHMARK_URI}" uv run python -m tests.mongo.benchmarks bench_find

# Run update benchmarks
bench-update:
    MONGODB_URI="${MONGODB_BENCHMARK_URI}" uv run python -m tests.mongo.benchmarks bench_update

# Run all benchmarks
bench-all:
    MONGODB_URI="${MONGODB_BENCHMARK_URI}" uv run python -m tests.mongo.benchmarks

# Run benchmark comparison (data-bridge vs Beanie vs PyMongo)
bench-comparison:
    MONGODB_URI="${MONGODB_COMPARISON_URI}" uv run python benchmarks/bench_comparison.py

# Profile Python/Rust boundary overhead
bench-profile:
    MONGODB_URI="${MONGODB_PROFILE_URI}" uv run python tests/mongo/benchmarks/profile_overhead.py

# ============================================================================
# CODE QUALITY COMMANDS
# ============================================================================

# Run Rust linter (clippy)
lint:
    cargo clippy -- -D warnings

# Run Rust linter with fixes
lint-fix:
    cargo clippy --fix --allow-dirty --allow-staged

# Format Rust code
fmt:
    cargo fmt

# Check Rust formatting without changes
fmt-check:
    cargo fmt -- --check

# Run all quality checks
check: fmt-check lint test-rust

# ============================================================================
# MONGODB COMMANDS
# ============================================================================

# Check MongoDB connection
mongo-check:
    @python -c "import pymongo; client = pymongo.MongoClient('mongodb://localhost:27017/', serverSelectionTimeoutMS=2000); client.server_info(); print('✓ MongoDB is running')" 2>&1 || echo "✗ MongoDB is not running"

# Start MongoDB (macOS with Homebrew)
mongo-start:
    @brew services start mongodb-community 2>/dev/null || mongod --config /usr/local/etc/mongod.conf --fork 2>/dev/null || echo "Please start MongoDB manually"

# Stop MongoDB (macOS with Homebrew)
mongo-stop:
    @brew services stop mongodb-community 2>/dev/null || killall mongod 2>/dev/null || echo "MongoDB not running"

# Clean test databases
mongo-clean:
    @python -c "import pymongo; c = pymongo.MongoClient('mongodb://localhost:27017/'); c.drop_database('data-bridge-test'); c.drop_database('data-bridge-benchmark'); c.drop_database('bench'); c.drop_database('bench_profile'); print('✓ Test databases dropped')"

# ============================================================================
# DEVELOPMENT WORKFLOW
# ============================================================================

# Full development build and test cycle
dev: build-release test-all
    @echo "✓ Build and tests complete"

# Quick development check (fast)
dev-quick: build test-rust test-unit
    @echo "✓ Quick check complete"

# Pre-commit checks (run before committing)
pre-commit: fmt lint test-rust
    @echo "✓ Pre-commit checks passed"

# Quick verification script for Feature 004 fast-path
verify-004: build-release mongo-check
    uv run python tests/mongo/benchmarks/verify_004_fast_path.py

# Performance verification workflow for feature 004
perf-004: build-release mongo-check bench-insert
    @echo ""
    @echo "==================================================================="
    @echo "Feature 004 Performance Verification Complete"
    @echo "==================================================================="
    @echo "Review the benchmark results above."
    @echo "Expected targets:"
    @echo "  - insert_one (fast-path): <1.0ms (2x faster than Beanie)"
    @echo "  - bulk_insert (fast-path): <15.0ms (3.9x faster than Beanie)"
    @echo "==================================================================="

# ============================================================================
# FEATURE DEVELOPMENT WORKFLOWS (SDD)
# ============================================================================

# Create new feature spec (requires feature number and name)
spec-new FEATURE:
    @mkdir -p .specify/specs/{{FEATURE}}
    @echo "Creating spec for feature: {{FEATURE}}"
    @echo "Run: /speckit:specify to generate spec.md"

# Run full SDD workflow for a feature
sdd-workflow FEATURE: (spec-new FEATURE)
    @echo "Starting SDD workflow for {{FEATURE}}"
    @echo "Steps:"
    @echo "1. Run /speckit:specify"
    @echo "2. Run /speckit:plan"
    @echo "3. Run /speckit:tasks"
    @echo "4. Implement tasks"
    @echo "5. Run: just test-all"
    @echo "6. Run: just bench-all (if performance feature)"

# ============================================================================
# DOCUMENTATION
# ============================================================================

# Generate API documentation
docs:
    cargo doc --no-deps --open

# Serve documentation locally
docs-serve:
    cargo doc --no-deps
    @echo "Documentation generated in target/doc/"

# ============================================================================
# SECURITY
# ============================================================================

# Run security audit
audit:
    cargo audit

# Update dependencies and check for vulnerabilities
update-deps:
    cargo update
    cargo audit

# ============================================================================
# SPECKIT WORKFLOWS (for reference)
# ============================================================================

# Note: These are slash commands to run with Claude Code, not just commands
# /speckit:specify     - Create feature specification
# /speckit:plan        - Create implementation plan
# /speckit:tasks       - Generate task breakdown
# /speckit:implement   - Implement feature
# /speckit:clarify     - Ask clarification questions
# /speckit:analyze     - Analyze consistency

# ============================================================================
# HELPERS
# ============================================================================

# Show project status
status:
    @echo "==================================================================="
    @echo "data-bridge Project Status"
    @echo "==================================================================="
    @git log -1 --oneline 2>/dev/null || echo "Not a git repository"
    @echo ""
    @echo "Rust Build:"
    @cargo --version 2>/dev/null || echo "  ✗ Cargo not found"
    @rustc --version 2>/dev/null || echo "  ✗ Rustc not found"
    @echo ""
    @echo "Python Environment:"
    @uv --version 2>/dev/null || echo "  ✗ uv not found"
    @python --version 2>/dev/null || echo "  ✗ Python not found"
    @echo ""
    @echo "MongoDB:"
    @just mongo-check
    @echo ""
    @echo "Recent commits:"
    @git log -3 --oneline 2>/dev/null || echo "  No git history"
    @echo "==================================================================="

# Install development dependencies
setup:
    @echo "Setting up development environment..."
    @command -v rustup >/dev/null || (echo "Installing rustup..." && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)
    @command -v uv >/dev/null || (echo "Installing uv..." && curl -LsSf https://astral.sh/uv/install.sh | sh)
    @command -v just >/dev/null || (echo "Installing just..." && cargo install just)
    uv sync
    @echo "✓ Development environment ready"
    @echo ""
    @echo "Next steps:"
    @echo "  1. Start MongoDB: just mongo-start"
    @echo "  2. Build project: just build-release"
    @echo "  3. Run tests: just test-all"

# Show environment info
info:
    @echo "Build Configuration:"
    @echo "  Rust toolchain: $(rustc --version)"
    @echo "  Cargo version: $(cargo --version)"
    @echo "  Python version: $(python --version)"
    @echo "  uv version: $(uv --version)"
    @echo ""
    @echo "Project Structure:"
    @find crates -name "Cargo.toml" -exec echo "  {}" \;
    @echo ""
    @echo "Test Stats:"
    @echo "  Python tests: $(find tests -name "test_*.py" | wc -l | tr -d ' ') files"
    @echo "  Rust tests: $(grep -r "#\[test\]" crates | wc -l | tr -d ' ') tests"
