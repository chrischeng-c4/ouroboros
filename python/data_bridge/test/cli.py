"""
dbtest CLI - Unified test and benchmark runner with auto-discovery.

This module provides a command-line interface for running tests and benchmarks
with automatic file discovery powered by the Rust engine.
"""

from __future__ import annotations

import argparse
import asyncio
import sys
from pathlib import Path
from typing import List, Optional

from .lazy_loader import lazy_load_test_suite, lazy_load_benchmark
from . import (
    DiscoveryConfig,
    discover_files,
    FileType,
    TestRunner,
    Reporter,
    ReportFormat,
    run_benchmarks,
    clear_registry,
)


class CLIConfig:
    """Configuration for CLI execution."""

    def __init__(self):
        self.root_path: str = "tests/"
        self.patterns: List[str] = ["test_*.py", "bench_*.py"]
        self.exclusions: List[str] = ["__pycache__", ".git", ".venv", "node_modules"]
        self.max_depth: int = 10
        self.test_type: Optional[str] = None  # unit, integration, all
        self.run_benchmarks_flag: bool = False
        self.run_tests_flag: bool = True
        self.pattern_filter: Optional[str] = None
        self.tags: List[str] = []
        self.verbose: bool = False
        self.fail_fast: bool = False
        self.format: str = "console"  # console, json, markdown
        self.output_file: Optional[str] = None


def create_discovery_config(cli_config: CLIConfig) -> DiscoveryConfig:
    """
    Create a DiscoveryConfig from CLIConfig.

    Args:
        cli_config: CLI configuration

    Returns:
        DiscoveryConfig for Rust discovery engine
    """
    return DiscoveryConfig(
        root_path=cli_config.root_path,
        patterns=cli_config.patterns,
        exclusions=cli_config.exclusions,
        max_depth=cli_config.max_depth,
    )


async def run_tests_only(cli_config: CLIConfig) -> int:
    """
    Run only tests (no benchmarks).

    Args:
        cli_config: CLI configuration

    Returns:
        Exit code (0 = success, 1 = failures, 2 = errors)
    """
    # Create discovery config
    patterns = ["test_*.py"]
    if cli_config.pattern_filter:
        patterns = [cli_config.pattern_filter]

    discovery_config = DiscoveryConfig(
        root_path=cli_config.root_path,
        patterns=patterns,
        exclusions=cli_config.exclusions,
        max_depth=cli_config.max_depth,
    )

    # Discover test files
    print(f"🔍 Discovering test files in {cli_config.root_path}...")
    files = discover_files(discovery_config)

    # Filter by file type
    test_files = [f for f in files if f.file_type == FileType.Test]

    if not test_files:
        print("❌ No test files found")
        return 1

    print(f"✅ Found {len(test_files)} test file(s)")

    # Load and run test suites
    total_passed = 0
    total_failed = 0
    total_errors = 0

    for file_info in test_files:
        file_path = Path(file_info.path)

        if cli_config.verbose:
            print(f"\n📄 Loading: {file_info.module_name}")

        try:
            # Lazy load test suites from file
            suites = lazy_load_test_suite(file_path)

            for suite_class in suites:
                if cli_config.verbose:
                    print(f"  🧪 Running: {suite_class.__name__}")

                # Create suite instance and run it
                suite = suite_class()
                runner = TestRunner()
                report = await suite.run(runner=runner, verbose=cli_config.verbose)

                # Aggregate results from summary
                summary = report.summary
                total_passed += summary.passed
                total_failed += summary.failed
                total_errors += summary.errors

                # Fail fast if requested
                if cli_config.fail_fast and (summary.failed > 0 or summary.errors > 0):
                    print(f"\n❌ Stopping due to --fail-fast")
                    break

        except Exception as e:
            print(f"❌ Error loading {file_path}: {e}")
            total_errors += 1
            if cli_config.fail_fast:
                break

        if cli_config.fail_fast and (total_failed > 0 or total_errors > 0):
            break

    # Print summary
    print("\n" + "=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)
    print(f"✅ Passed:  {total_passed}")
    print(f"❌ Failed:  {total_failed}")
    print(f"⚠️  Errors:  {total_errors}")
    print("=" * 60)

    # Return appropriate exit code
    if total_errors > 0:
        return 2
    elif total_failed > 0:
        return 1
    else:
        return 0


async def run_benchmarks_only(cli_config: CLIConfig) -> int:
    """
    Run only benchmarks (no tests).

    Args:
        cli_config: CLI configuration

    Returns:
        Exit code (0 = success, 1 = failures)
    """
    # Clear benchmark registry before discovery
    clear_registry()

    # Create discovery config
    patterns = ["bench_*.py"]
    if cli_config.pattern_filter:
        patterns = [cli_config.pattern_filter]

    discovery_config = DiscoveryConfig(
        root_path=cli_config.root_path,
        patterns=patterns,
        exclusions=cli_config.exclusions,
        max_depth=cli_config.max_depth,
    )

    # Discover benchmark files
    print(f"🔍 Discovering benchmark files in {cli_config.root_path}...")
    files = discover_files(discovery_config)

    # Filter by file type
    bench_files = [f for f in files if f.file_type == FileType.Benchmark]

    if not bench_files:
        print("❌ No benchmark files found")
        return 1

    print(f"✅ Found {len(bench_files)} benchmark file(s)")

    # Load benchmark groups
    all_groups = []
    for file_info in bench_files:
        file_path = Path(file_info.path)

        if cli_config.verbose:
            print(f"\n📄 Loading: {file_info.module_name}")

        try:
            # Lazy load benchmark groups from file
            groups = lazy_load_benchmark(file_path)
            all_groups.extend(groups)

            if cli_config.verbose:
                for group in groups:
                    print(f"  📊 Group: {group.name} ({len(group.benchmarks)} benchmarks)")

        except Exception as e:
            print(f"❌ Error loading {file_path}: {e}")

    if not all_groups:
        print("❌ No benchmark groups found")
        return 1

    # Run all benchmarks
    print(f"\n🏃 Running {len(all_groups)} benchmark group(s)...")

    try:
        # Use existing run_benchmarks function
        # This will run all registered groups
        report = await run_benchmarks()

        if cli_config.verbose:
            print(f"\n📊 Benchmark Report:")
            print(f"  Total groups: {len(all_groups)}")
            # Report contains results - could display more details here

        print("\n✅ Benchmarks completed")
        return 0

    except Exception as e:
        print(f"❌ Benchmark execution failed: {e}")
        if cli_config.verbose:
            import traceback
            traceback.print_exc()
        return 1


async def run_all(cli_config: CLIConfig) -> int:
    """
    Run both tests and benchmarks.

    Args:
        cli_config: CLI configuration

    Returns:
        Exit code (0 = success, 1 = failures)
    """
    print("=" * 60)
    print("RUNNING TESTS")
    print("=" * 60)

    test_exit_code = await run_tests_only(cli_config)

    print("\n" + "=" * 60)
    print("RUNNING BENCHMARKS")
    print("=" * 60)

    bench_exit_code = await run_benchmarks_only(cli_config)

    # Return worst exit code
    return max(test_exit_code, bench_exit_code)


def parse_args(args: Optional[List[str]] = None) -> CLIConfig:
    """
    Parse command-line arguments.

    Args:
        args: Command-line arguments (defaults to sys.argv[1:])

    Returns:
        CLIConfig with parsed settings
    """
    parser = argparse.ArgumentParser(
        prog="dbtest",
        description="Unified test and benchmark runner with auto-discovery",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  dbtest                    # Run all tests and benchmarks
  dbtest unit               # Run unit tests only
  dbtest integration        # Run integration tests only
  dbtest bench              # Run benchmarks only
  dbtest --pattern "*crud*" # Run tests matching pattern
  dbtest --verbose          # Verbose output
  dbtest --fail-fast        # Stop on first failure
        """,
    )

    # Subcommands
    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # dbtest unit
    unit_parser = subparsers.add_parser("unit", help="Run unit tests only")
    unit_parser.add_argument("--pattern", help="File pattern to match")
    unit_parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    unit_parser.add_argument("--fail-fast", action="store_true", help="Stop on first failure")

    # dbtest integration
    integration_parser = subparsers.add_parser("integration", help="Run integration tests only")
    integration_parser.add_argument("--pattern", help="File pattern to match")
    integration_parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    integration_parser.add_argument("--fail-fast", action="store_true", help="Stop on first failure")

    # dbtest bench
    bench_parser = subparsers.add_parser("bench", help="Run benchmarks only")
    bench_parser.add_argument("--pattern", help="File pattern to match")
    bench_parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")

    # Global options (for main command without subcommand)
    parser.add_argument("--root", default="tests/", help="Root directory to search (default: tests/)")
    parser.add_argument("--pattern", help="File pattern to match (e.g., test_*crud*.py)")
    parser.add_argument("--tags", nargs="+", help="Filter tests by tags")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    parser.add_argument("--fail-fast", action="store_true", help="Stop on first failure")
    parser.add_argument("--format", choices=["console", "json", "markdown"], default="console",
                        help="Output format (default: console)")
    parser.add_argument("--output", "-o", help="Output file (default: stdout)")

    parsed = parser.parse_args(args)

    # Build CLIConfig
    config = CLIConfig()
    config.root_path = parsed.root
    config.verbose = parsed.verbose
    config.fail_fast = parsed.fail_fast
    config.format = parsed.format
    config.output_file = parsed.output

    if hasattr(parsed, "tags") and parsed.tags:
        config.tags = parsed.tags

    if parsed.pattern:
        config.pattern_filter = parsed.pattern

    # Handle subcommands
    if parsed.command == "unit":
        config.test_type = "unit"
        config.run_tests_flag = True
        config.run_benchmarks_flag = False
        config.patterns = ["test_*.py"]

    elif parsed.command == "integration":
        config.test_type = "integration"
        config.run_tests_flag = True
        config.run_benchmarks_flag = False
        config.patterns = ["test_*.py"]

    elif parsed.command == "bench":
        config.run_tests_flag = False
        config.run_benchmarks_flag = True
        config.patterns = ["bench_*.py"]

    else:
        # No subcommand: run all
        config.run_tests_flag = True
        config.run_benchmarks_flag = True

    return config


def main(args: Optional[List[str]] = None) -> int:
    """
    Main entry point for dbtest CLI.

    Args:
        args: Command-line arguments (defaults to sys.argv[1:])

    Returns:
        Exit code (0 = success, 1 = failures, 2 = errors)
    """
    config = parse_args(args)

    # Print banner
    print("=" * 60)
    print("dbtest - Data Bridge Test & Benchmark Runner")
    print("=" * 60)
    print()

    # Run appropriate command
    try:
        if config.run_benchmarks_flag and not config.run_tests_flag:
            # Benchmarks only
            exit_code = asyncio.run(run_benchmarks_only(config))

        elif config.run_tests_flag and not config.run_benchmarks_flag:
            # Tests only
            exit_code = asyncio.run(run_tests_only(config))

        else:
            # Both tests and benchmarks
            exit_code = asyncio.run(run_all(config))

        return exit_code

    except KeyboardInterrupt:
        print("\n\n⚠️  Interrupted by user")
        return 130  # Standard exit code for SIGINT

    except Exception as e:
        print(f"\n❌ Fatal error: {e}")
        if config.verbose:
            import traceback
            traceback.print_exc()
        return 2


if __name__ == "__main__":
    sys.exit(main())
