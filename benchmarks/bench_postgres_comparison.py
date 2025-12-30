#!/usr/bin/env python3
"""
PostgreSQL benchmark comparison: data-bridge vs asyncpg vs psycopg2 vs SQLAlchemy

Follows the same pattern as MongoDB benchmarks using the data_bridge.test framework.

Usage:
    python benchmarks/bench_postgres_comparison.py

Environment:
    POSTGRES_URI - default: postgresql://postgres:postgres@localhost:5432/data_bridge_benchmark
"""

import asyncio
import os
import sys
from pathlib import Path

# Add project root to Python path to enable importing from tests module
project_root = Path(__file__).parent.parent
if str(project_root) not in sys.path:
    sys.path.insert(0, str(project_root))

try:
    import uvloop
    uvloop.install()
except ImportError:
    pass

from data_bridge.postgres import init, close, is_connected
from data_bridge.test import discover_benchmarks, run_benchmarks

POSTGRES_URI = os.environ.get(
    "POSTGRES_URI",
    "postgresql://postgres:postgres@localhost:5432/data_bridge_benchmark"
)
BENCHMARK_DIR = Path(__file__).parent.parent / "tests" / "postgres" / "benchmarks"
RESULTS_DIR = BENCHMARK_DIR / "results"


async def setup_connections():
    """Initialize PostgreSQL connections for all frameworks."""
    print("Initializing PostgreSQL connections...")

    # data-bridge
    if is_connected():
        await close()
    await init(POSTGRES_URI)

    # Other frameworks (asyncpg, psycopg2, SQLAlchemy) are initialized by benchmark_setup
    from tests.postgres.benchmarks import benchmark_setup
    await benchmark_setup.async_ensure_setup()

    print("  All frameworks initialized")


async def teardown_connections():
    """Close all database connections."""
    from tests.postgres.benchmarks import benchmark_setup

    await benchmark_setup.cleanup()
    await close()


async def main():
    print("=" * 70)
    print("PostgreSQL Framework Comparison Benchmark")
    print("=" * 70)
    print(f"\nPostgreSQL URI: {POSTGRES_URI}")
    print("Comparing: data-bridge vs asyncpg vs psycopg2 vs SQLAlchemy")
    print()

    await setup_connections()

    try:
        # Discover benchmarks
        info = discover_benchmarks(BENCHMARK_DIR)
        print(f"Discovered {len(info['files'])} benchmark files")
        print(f"Found {info['groups']} benchmark groups")
        print()

        # Run benchmarks with SQLAlchemy as baseline
        report = await run_benchmarks(
            baseline_name="SQLAlchemy",
            title="PostgreSQL Framework Comparison",
            description="Rust-based data-bridge vs asyncpg vs psycopg2 vs SQLAlchemy ORM",
        )

        # Print console report
        print(report.to_console())

        # Save results
        RESULTS_DIR.mkdir(exist_ok=True, parents=True)

        report.save(str(RESULTS_DIR / "benchmark_report"), "markdown")
        report.save(str(RESULTS_DIR / "full_results"), "json")

        print(f"\nResults saved to: {RESULTS_DIR}")
        print(f"  - Markdown: {RESULTS_DIR / 'benchmark_report.md'}")
        print(f"  - JSON: {RESULTS_DIR / 'full_results.json'}")

    finally:
        await teardown_connections()


if __name__ == "__main__":
    asyncio.run(main())
