#!/usr/bin/env python3
"""
Migration Validation Tool

Validates that pytest-to-data-bridge-test migrations preserve test behavior.
Runs both pytest and data-bridge-test on the same tests and compares results.

Usage:
    python tools/validate_migration.py tests/unit/test_example.py
    python tools/validate_migration.py tests/ --recursive --report=report.json
"""

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Dict, List, Optional


@dataclass
class TestStats:
    """Statistics from a test run."""
    total: int = 0
    passed: int = 0
    failed: int = 0
    skipped: int = 0
    errors: int = 0
    duration_ms: float = 0.0


@dataclass
class ValidationResult:
    """Result of validating a single file."""
    file_path: str
    pytest_available: bool
    pytest_stats: Optional[TestStats]
    data_bridge_stats: Optional[TestStats]
    tests_match: bool
    issues: List[str]


class ValidationReport:
    """Aggregated validation report."""

    def __init__(self):
        self.results: List[ValidationResult] = []

    def add_result(self, result: ValidationResult):
        """Add a validation result."""
        self.results.append(result)

    @property
    def total_files(self) -> int:
        """Total number of files validated."""
        return len(self.results)

    @property
    def matching_files(self) -> int:
        """Number of files where tests match."""
        return sum(1 for r in self.results if r.tests_match)

    @property
    def mismatched_files(self) -> int:
        """Number of files with mismatches."""
        return sum(1 for r in self.results if not r.tests_match)

    @property
    def all_issues(self) -> List[str]:
        """All issues across all files."""
        issues = []
        for result in self.results:
            issues.extend(result.issues)
        return issues

    def to_dict(self) -> Dict:
        """Convert report to dictionary."""
        return {
            'summary': {
                'total_files': self.total_files,
                'matching_files': self.matching_files,
                'mismatched_files': self.mismatched_files,
                'total_issues': len(self.all_issues),
            },
            'results': [asdict(r) for r in self.results],
        }

    def save_json(self, path: Path):
        """Save report as JSON."""
        with open(path, 'w', encoding='utf-8') as f:
            json.dump(self.to_dict(), f, indent=2)

    def print_summary(self):
        """Print summary to console."""
        print("\n" + "=" * 60)
        print("Validation Report")
        print("=" * 60)
        print(f"Total files:      {self.total_files}")
        print(f"Matching:         {self.matching_files}")
        print(f"Mismatched:       {self.mismatched_files}")
        print(f"Total issues:     {len(self.all_issues)}")
        print()

        if self.results:
            print("Details:")
            for result in self.results:
                status = "✓" if result.tests_match else "✗"
                print(f"  {status} {result.file_path}")

                if result.pytest_stats and result.data_bridge_stats:
                    print(f"     pytest:       {result.pytest_stats.passed}/{result.pytest_stats.total} passed")
                    print(f"     data-bridge:  {result.data_bridge_stats.passed}/{result.data_bridge_stats.total} passed")

                if result.issues:
                    for issue in result.issues[:3]:  # Show first 3 issues
                        print(f"     - {issue}")
                    if len(result.issues) > 3:
                        print(f"     ... and {len(result.issues) - 3} more issues")

        print()


def run_pytest(file_path: Path) -> Optional[TestStats]:
    """
    Run pytest on a file and extract statistics.

    Args:
        file_path: Path to test file

    Returns:
        TestStats or None if pytest not available or failed
    """
    try:
        # Run pytest with JSON report
        result = subprocess.run(
            ['pytest', str(file_path), '-v', '--tb=short', '--quiet'],
            capture_output=True,
            text=True,
            timeout=60
        )

        # Parse output to extract stats
        # Look for lines like: "5 passed in 0.23s"
        output = result.stdout + result.stderr

        stats = TestStats()

        # Simple parsing of pytest output
        for line in output.split('\n'):
            line_lower = line.lower()

            if 'passed' in line_lower:
                # Try to extract numbers
                parts = line.split()
                for i, part in enumerate(parts):
                    if 'passed' in part and i > 0:
                        try:
                            stats.passed = int(parts[i - 1])
                        except (ValueError, IndexError):
                            pass

            if 'failed' in line_lower:
                parts = line.split()
                for i, part in enumerate(parts):
                    if 'failed' in part and i > 0:
                        try:
                            stats.failed = int(parts[i - 1])
                        except (ValueError, IndexError):
                            pass

            if 'skipped' in line_lower:
                parts = line.split()
                for i, part in enumerate(parts):
                    if 'skipped' in part and i > 0:
                        try:
                            stats.skipped = int(parts[i - 1])
                        except (ValueError, IndexError):
                            pass

            if 'error' in line_lower:
                parts = line.split()
                for i, part in enumerate(parts):
                    if 'error' in part and i > 0:
                        try:
                            stats.errors = int(parts[i - 1])
                        except (ValueError, IndexError):
                            pass

        # Calculate total
        stats.total = stats.passed + stats.failed + stats.skipped + stats.errors

        # If we didn't get any stats, pytest likely failed
        if stats.total == 0:
            return None

        return stats

    except subprocess.TimeoutExpired:
        return None
    except FileNotFoundError:
        # pytest not installed
        return None
    except Exception:
        return None


def run_data_bridge_test(file_path: Path) -> Optional[TestStats]:
    """
    Run data-bridge-test on a file and extract statistics.

    Args:
        file_path: Path to test file

    Returns:
        TestStats or None if failed
    """
    try:
        # Run with uv
        result = subprocess.run(
            ['uv', 'run', 'python', str(file_path)],
            capture_output=True,
            text=True,
            timeout=60
        )

        # Parse output
        output = result.stdout + result.stderr

        stats = TestStats()

        # Look for summary lines like "Results: 5/10 passed"
        for line in output.split('\n'):
            if 'Results:' in line and 'passed' in line:
                # Extract: "Results: 5/10 passed"
                try:
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if '/' in part:
                            passed, total = part.split('/')
                            stats.passed = int(passed)
                            stats.total = int(total)
                            stats.failed = stats.total - stats.passed
                            break
                except (ValueError, IndexError):
                    pass

            if 'Failed:' in line:
                try:
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if part == 'Failed:' and i + 1 < len(parts):
                            stats.failed = int(parts[i + 1])
                            break
                except (ValueError, IndexError):
                    pass

            if 'Skipped:' in line:
                try:
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if part == 'Skipped:' and i + 1 < len(parts):
                            stats.skipped = int(parts[i + 1])
                            break
                except (ValueError, IndexError):
                    pass

            if 'Errors:' in line:
                try:
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if part == 'Errors:' and i + 1 < len(parts):
                            stats.errors = int(parts[i + 1])
                            break
                except (ValueError, IndexError):
                    pass

            if 'Duration:' in line:
                try:
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if part == 'Duration:' and i + 1 < len(parts):
                            duration_str = parts[i + 1].rstrip('ms')
                            stats.duration_ms = float(duration_str)
                            break
                except (ValueError, IndexError):
                    pass

        # If we didn't get stats, test may have failed
        if stats.total == 0:
            return None

        return stats

    except subprocess.TimeoutExpired:
        return None
    except Exception:
        return None


def validate_file(file_path: Path, skip_pytest: bool = False) -> ValidationResult:
    """
    Validate migration for a single file.

    Args:
        file_path: Path to test file
        skip_pytest: Skip running pytest (for already migrated files)

    Returns:
        ValidationResult
    """
    result = ValidationResult(
        file_path=str(file_path),
        pytest_available=False,
        pytest_stats=None,
        data_bridge_stats=None,
        tests_match=False,
        issues=[]
    )

    print(f"Validating: {file_path}")

    # Run pytest (unless skipped)
    if not skip_pytest:
        print("  Running pytest...")
        pytest_stats = run_pytest(file_path)

        if pytest_stats is None:
            result.issues.append("pytest failed or not available")
        else:
            result.pytest_available = True
            result.pytest_stats = pytest_stats
            print(f"    {pytest_stats.passed}/{pytest_stats.total} tests passed")

    # Run data-bridge-test
    print("  Running data-bridge-test...")
    data_bridge_stats = run_data_bridge_test(file_path)

    if data_bridge_stats is None:
        result.issues.append("data-bridge-test failed")
        print("    Failed to run")
    else:
        result.data_bridge_stats = data_bridge_stats
        print(f"    {data_bridge_stats.passed}/{data_bridge_stats.total} tests passed")

    # Compare results
    if result.pytest_stats and result.data_bridge_stats:
        pytest_s = result.pytest_stats
        dbt_s = result.data_bridge_stats

        # Check if test counts match
        if pytest_s.total != dbt_s.total:
            result.issues.append(
                f"Test count mismatch: pytest={pytest_s.total}, data-bridge-test={dbt_s.total}"
            )

        # Check if passed counts match
        if pytest_s.passed != dbt_s.passed:
            result.issues.append(
                f"Passed count mismatch: pytest={pytest_s.passed}, data-bridge-test={dbt_s.passed}"
            )

        # Check if failed counts match
        if pytest_s.failed != dbt_s.failed:
            result.issues.append(
                f"Failed count mismatch: pytest={pytest_s.failed}, data-bridge-test={dbt_s.failed}"
            )

        # Tests match if no issues
        result.tests_match = len(result.issues) == 0

    elif result.data_bridge_stats and skip_pytest:
        # If we skipped pytest, just check data-bridge-test ran
        result.tests_match = True

    print(f"  Result: {'✓ PASS' if result.tests_match else '✗ FAIL'}")
    print()

    return result


def find_test_files(path: Path, recursive: bool = False) -> List[Path]:
    """
    Find all test files in the given path.

    Args:
        path: Directory or file path
        recursive: If True, search recursively

    Returns:
        List of test file paths
    """
    if path.is_file():
        if path.name.startswith('test_') and path.suffix == '.py':
            return [path]
        return []

    if recursive:
        return list(path.rglob('test_*.py'))
    else:
        return list(path.glob('test_*.py'))


def main():
    parser = argparse.ArgumentParser(
        description='Validate pytest-to-data-bridge-test migration',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Validate single file
  python tools/validate_migration.py tests/unit/test_example.py

  # Validate all tests recursively
  python tools/validate_migration.py tests/ --recursive

  # Skip pytest (for already migrated files)
  python tools/validate_migration.py tests/ --recursive --skip-pytest

  # Save report to JSON
  python tools/validate_migration.py tests/ --recursive --report=validation.json
        """
    )

    parser.add_argument(
        'paths',
        nargs='+',
        type=Path,
        help='Test files or directories to validate'
    )
    parser.add_argument(
        '--recursive',
        action='store_true',
        help='Recursively search for test files in directories'
    )
    parser.add_argument(
        '--skip-pytest',
        action='store_true',
        help='Skip running pytest (for already migrated files)'
    )
    parser.add_argument(
        '--report',
        type=Path,
        help='Save validation report to JSON file'
    )

    args = parser.parse_args()

    # Collect all test files
    all_files = []
    for path in args.paths:
        if not path.exists():
            print(f"Error: Path does not exist: {path}", file=sys.stderr)
            sys.exit(1)

        files = find_test_files(path, args.recursive)
        all_files.extend(files)

    if not all_files:
        print("No test files found", file=sys.stderr)
        sys.exit(1)

    print(f"Found {len(all_files)} test file(s)")
    print()

    # Validate each file
    report = ValidationReport()

    for file_path in all_files:
        result = validate_file(file_path, args.skip_pytest)
        report.add_result(result)

    # Print summary
    report.print_summary()

    # Save report if requested
    if args.report:
        report.save_json(args.report)
        print(f"Report saved to: {args.report}")

    # Exit with error code if there were mismatches
    sys.exit(0 if report.mismatched_files == 0 else 1)


if __name__ == '__main__':
    main()
