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
# DBTEST COMMANDS (Primary Test & Benchmark Runner)
# ============================================================================

# Run all tests and benchmarks (auto-discovers tests/*)
test:
    uv run dbtest

# Run unit tests only
test-unit:
    uv run dbtest unit

# Run integration tests only
test-integration:
    uv run dbtest integration

# Run all benchmarks (auto-discovers bench_*.py)
bench:
    uv run dbtest bench

# Run PostgreSQL benchmarks (data-bridge vs SQLAlchemy)
bench-postgres:
    #!/usr/bin/env bash
    POSTGRES_URI="${POSTGRES_URI:-postgresql://rstn:rstn@localhost:5432/data_bridge_benchmark}" \
    uv run python benchmarks/bench_postgres_comparison.py

# Set up PostgreSQL test database
test-postgres-setup:
    #!/usr/bin/env bash
    echo "Setting up PostgreSQL test database..."
    docker exec rstn-postgres psql -U rstn -c "DROP DATABASE IF EXISTS data_bridge_test;" || true
    docker exec rstn-postgres psql -U rstn -c "CREATE DATABASE data_bridge_test;"
    echo "✓ Test database ready: postgresql://rstn:rstn@localhost:5432/data_bridge_test"

# Run PostgreSQL integration tests
test-postgres:
    #!/usr/bin/env bash
    echo "Running PostgreSQL integration tests..."
    just test-postgres-setup
    POSTGRES_URI="postgresql://rstn:rstn@localhost:5432/data_bridge_test" \
    uv run pytest tests/postgres/integration/ -v -m integration

# Run PostgreSQL migration example
test-postgres-migrations:
    #!/usr/bin/env bash
    echo "Running PostgreSQL migration example..."
    just test-postgres-setup
    POSTGRES_URI="postgresql://rstn:rstn@localhost:5432/data_bridge_test" \
    uv run python examples/postgres_migrations_example.py

# Run with verbose output
test-verbose:
    uv run dbtest --verbose

# Run with pattern filter
test-pattern PATTERN:
    uv run dbtest --pattern "{{PATTERN}}"

# Run with fail-fast mode
test-fail-fast:
    uv run dbtest --fail-fast

# Run tests with custom format (console/json/markdown)
test-format FORMAT:
    uv run dbtest --format {{FORMAT}}

# ============================================================================
# RUST TEST COMMANDS
# ============================================================================

# Run all Rust tests
test-rust:
    cargo test

# Run Rust tests for specific crate
test-rust-crate CRATE:
    cargo test -p {{CRATE}}

# Run all tests (Rust + Python via dbtest)
test-all: test-rust test

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
# DEVELOPMENT WORKFLOWS
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

# Full CI workflow (build + lint + test + bench)
ci: build-release check test-all bench
    @echo "✓ CI checks complete"

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
# PROJECT SETUP & INFO
# ============================================================================

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
    @echo "  3. Run tests: just test"

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
    @echo "  Test files: $(find tests -name "test_*.py" -o -name "bench_*.py" | wc -l | tr -d ' ') files"
    @echo "  Rust tests: $(grep -r "#\[test\]" crates | wc -l | tr -d ' ') tests"
