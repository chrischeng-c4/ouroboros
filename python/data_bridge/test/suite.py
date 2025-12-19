"""
TestSuite base class for data_bridge.test

Provides a base class for organizing tests into suites.
"""

from __future__ import annotations

import asyncio
import time
import traceback
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Type

# Import from Rust bindings
from data_bridge import data_bridge as _rust_module
_test = _rust_module.test
TestRunner = _test.TestRunner
TestResult = _test.TestResult
TestReport = _test.TestReport
Reporter = _test.Reporter
ReportFormat = _test.ReportFormat
FileCoverage = _test.FileCoverage
CoverageInfo = _test.CoverageInfo

from .decorators import TestDescriptor


class TestSuite:
    """
    Base class for test suites.

    Subclass this to create a test suite with setup/teardown hooks
    and test discovery.

    Example:
        from data_bridge.test import TestSuite, test, expect
        from data_bridge.http import HttpClient

        class UserAPITests(TestSuite):
            async def setup_suite(self):
                self.client = HttpClient(base_url="http://localhost:8000")

            async def teardown_suite(self):
                pass  # cleanup

            async def setup(self):
                pass  # before each test

            async def teardown(self):
                pass  # after each test

            @test(timeout=5.0, tags=["unit"])
            async def login_returns_token(self):
                response = await self.client.post("/auth/login", json={
                    "email": "test@example.com",
                    "password": "secret"
                })
                expect(response.status_code).to_equal(200)
    """

    def __init__(self) -> None:
        self._tests: List[TestDescriptor] = []
        self._discover_tests()

    def _discover_tests(self) -> None:
        """Discover all test methods in this suite"""
        self._tests = []

        for name in dir(self):
            attr = getattr(self.__class__, name, None)
            if isinstance(attr, TestDescriptor):
                self._tests.append(attr)

    @property
    def test_count(self) -> int:
        """Number of tests in this suite"""
        return len(self._tests)

    @property
    def suite_name(self) -> str:
        """Name of this test suite"""
        return self.__class__.__name__

    # Lifecycle hooks (override in subclasses)

    async def setup_suite(self) -> None:
        """Called once before all tests in the suite"""
        pass

    async def teardown_suite(self) -> None:
        """Called once after all tests in the suite"""
        pass

    async def setup(self) -> None:
        """Called before each test"""
        pass

    async def teardown(self) -> None:
        """Called after each test"""
        pass

    # Test execution

    async def run(
        self,
        runner: Optional[TestRunner] = None,
        verbose: bool = False,
        parallel: bool = False,
        max_workers: int = 4,
    ) -> TestReport:
        """
        Run all tests in this suite.

        Args:
            runner: Optional test runner with filters. If None, runs all tests.
            verbose: Whether to print verbose output
            parallel: Enable parallel test execution (default: False)
            max_workers: Maximum number of concurrent tests when parallel=True (default: 4)

        Returns:
            TestReport with all results
        """
        if runner is None:
            # Create runner with appropriate config for parallel/sequential execution
            runner = TestRunner(parallel=parallel, max_workers=max_workers)
        elif parallel:
            # If runner provided but parallel requested, create a new runner with parallel config
            # This ensures max_workers is respected
            runner = TestRunner(parallel=parallel, max_workers=max_workers)

        # If parallel execution is requested, use Rust parallel runner
        if parallel:
            return await self._run_parallel(runner, verbose, max_workers)

        # Otherwise, use sequential execution (existing behavior)

        runner.start()

        # Suite setup
        try:
            await self.setup_suite()
        except Exception as e:
            # If setup fails, mark all tests as error
            for test_desc in self._tests:
                meta = test_desc.get_meta()
                result = TestResult.error(meta, 0, f"Suite setup failed: {e}")
                result.set_stack_trace(traceback.format_exc())
                runner.record(result)
            return TestReport(self.suite_name, runner.results())

        # Run each test
        for test_desc in self._tests:
            meta = test_desc.get_meta()

            # Check if test should run based on filters
            if not runner.should_run(meta):
                continue

            # Check if skipped
            if meta.is_skipped():
                result = TestResult.skipped(meta, meta.skip_reason or "Skipped")
                runner.record(result)
                if verbose:
                    print(f"  SKIPPED: {meta.name}")
                continue

            # Run test setup
            try:
                await self.setup()
            except Exception as e:
                result = TestResult.error(meta, 0, f"Test setup failed: {e}")
                result.set_stack_trace(traceback.format_exc())
                runner.record(result)
                if verbose:
                    print(f"  ERROR: {meta.name} (setup failed)")
                continue

            # Run the test
            start_time = time.perf_counter()
            try:
                if test_desc.is_async:
                    await test_desc(self)
                else:
                    test_desc(self)

                duration_ms = int((time.perf_counter() - start_time) * 1000)
                result = TestResult.passed(meta, duration_ms)

                if verbose:
                    print(f"  PASSED: {meta.name} ({duration_ms}ms)")

            except AssertionError as e:
                duration_ms = int((time.perf_counter() - start_time) * 1000)
                result = TestResult.failed(meta, duration_ms, str(e))
                result.set_stack_trace(traceback.format_exc())

                if verbose:
                    print(f"  FAILED: {meta.name} ({duration_ms}ms)")
                    print(f"    Error: {e}")

            except Exception as e:
                duration_ms = int((time.perf_counter() - start_time) * 1000)
                result = TestResult.error(meta, duration_ms, str(e))
                result.set_stack_trace(traceback.format_exc())

                if verbose:
                    print(f"  ERROR: {meta.name} ({duration_ms}ms)")
                    print(f"    Error: {e}")

            runner.record(result)

            # Run test teardown
            try:
                await self.teardown()
            except Exception as e:
                # Log teardown error but don't override test result
                if verbose:
                    print(f"  WARNING: Teardown failed for {meta.name}: {e}")

        # Suite teardown
        try:
            await self.teardown_suite()
        except Exception as e:
            if verbose:
                print(f"WARNING: Suite teardown failed: {e}")

        return TestReport(self.suite_name, runner.results())

    async def _run_parallel(
        self,
        runner: TestRunner,
        verbose: bool,
        max_workers: int,
    ) -> TestReport:
        """
        Run tests in parallel using Rust Tokio runtime.

        Args:
            runner: Test runner with configuration
            verbose: Whether to print verbose output
            max_workers: Maximum concurrent tests

        Returns:
            TestReport with all results
        """
        runner.start()

        # Filter tests based on runner configuration
        tests_to_run = []
        skipped_results = []

        for test_desc in self._tests:
            meta = test_desc.get_meta()

            # Check if test should run based on filters
            if not runner.should_run(meta):
                continue

            # Check if skipped
            if meta.is_skipped():
                result = TestResult.skipped(meta, meta.skip_reason or "Skipped")
                skipped_results.append(result)
                if verbose:
                    print(f"  SKIPPED: {meta.name}")
                continue

            tests_to_run.append(test_desc)

        if verbose and tests_to_run:
            print(f"  Running {len(tests_to_run)} tests in parallel (max_workers={max_workers})...")

        # Run tests in parallel using Rust runner
        if tests_to_run:
            try:
                # Call Rust parallel runner
                # Note: max_workers is used via the runner's config
                results = await runner.run_parallel_async(
                    suite_instance=self,
                    test_descriptors=tests_to_run,
                )

                # Record all results
                for result in results:
                    runner.record(result)
                    if verbose:
                        status_str = str(result.status).upper()
                        duration = result.duration_ms
                        print(f"  {status_str}: {result.meta.name} ({duration}ms)")
                        if result.error_message and result.status != "SKIPPED":
                            print(f"    Error: {result.error_message}")

            except Exception as e:
                # If parallel execution fails, mark all tests as error
                if verbose:
                    print(f"  ERROR: Parallel execution failed: {e}")
                    traceback.print_exc()

                for test_desc in tests_to_run:
                    meta = test_desc.get_meta()
                    result = TestResult.error(meta, 0, f"Parallel execution failed: {e}")
                    result.set_stack_trace(traceback.format_exc())
                    runner.record(result)

        # Record skipped tests
        for result in skipped_results:
            runner.record(result)

        return TestReport(self.suite_name, runner.results())


def run_suite(
    suite_class: Type[TestSuite],
    output_format: ReportFormat = ReportFormat.Markdown,
    output_file: Optional[str] = None,
    verbose: bool = True,
    parallel: bool = False,
    max_workers: int = 4,
    **runner_kwargs: Any,
) -> TestReport:
    """
    Convenience function to run a test suite.

    Args:
        suite_class: The TestSuite subclass to run
        output_format: Report output format (default: Markdown)
        output_file: Optional file path to write report
        verbose: Whether to print verbose output
        parallel: Enable parallel test execution (default: False)
        max_workers: Maximum number of concurrent tests when parallel=True (default: 4)
        **runner_kwargs: Additional arguments for TestRunner

    Returns:
        TestReport with all results

    Example:
        from data_bridge.test import run_suite, ReportFormat

        # Sequential execution (default)
        report = run_suite(MyTests, output_format=ReportFormat.Html, output_file="report.html")

        # Parallel execution
        report = run_suite(MyTests, parallel=True, max_workers=8)
    """
    suite = suite_class()
    runner = TestRunner(**runner_kwargs)

    if verbose:
        print(f"\nRunning: {suite.suite_name}")
        if parallel:
            print(f"Mode: Parallel (max_workers={max_workers})")
        else:
            print("Mode: Sequential")
        print("=" * 50)

    report = asyncio.run(suite.run(runner=runner, verbose=verbose, parallel=parallel, max_workers=max_workers))

    if verbose:
        print("=" * 50)
        summary = report.summary
        print(f"Results: {summary.passed}/{summary.total} passed")
        if summary.failed > 0:
            print(f"  Failed: {summary.failed}")
        if summary.errors > 0:
            print(f"  Errors: {summary.errors}")
        if summary.skipped > 0:
            print(f"  Skipped: {summary.skipped}")
        print(f"Duration: {summary.total_duration_ms}ms")

    # Generate and optionally save report
    if output_file:
        reporter = Reporter(output_format)
        report_content = reporter.generate(report)

        with open(output_file, "w") as f:
            f.write(report_content)

        if verbose:
            print(f"\nReport written to: {output_file}")

    return report


def run_suites(
    suite_classes: List[Type[TestSuite]],
    output_format: ReportFormat = ReportFormat.Markdown,
    output_file: Optional[str] = None,
    verbose: bool = True,
    **runner_kwargs: Any,
) -> List[TestReport]:
    """
    Run multiple test suites.

    Args:
        suite_classes: List of TestSuite subclasses to run
        output_format: Report output format
        output_file: Optional file path for combined report
        verbose: Whether to print verbose output
        **runner_kwargs: Additional arguments for TestRunner

    Returns:
        List of TestReports, one per suite
    """
    reports = []

    for suite_class in suite_classes:
        report = run_suite(
            suite_class,
            output_format=output_format,
            verbose=verbose,
            **runner_kwargs,
        )
        reports.append(report)

    # Optionally combine reports into one file
    if output_file and reports:
        reporter = Reporter(output_format)
        combined = "\n\n---\n\n".join(reporter.generate(r) for r in reports)

        with open(output_file, "w") as f:
            f.write(combined)

        if verbose:
            print(f"\nCombined report written to: {output_file}")

    return reports


def _collect_coverage_from_coveragepy(
    source_dirs: List[str],
    omit_patterns: Optional[List[str]] = None,
) -> Optional[CoverageInfo]:
    """
    Collect coverage data from coverage.py.

    Must be called after coverage.stop() and coverage.save().

    Args:
        source_dirs: Directories to collect coverage from
        omit_patterns: Patterns to omit from coverage

    Returns:
        CoverageInfo object or None if coverage module not available
    """
    try:
        import coverage
    except ImportError:
        return None

    # Load existing coverage data
    cov = coverage.Coverage()
    try:
        cov.load()
    except coverage.misc.CoverageException:
        return None

    # Get analysis data
    coverage_info = CoverageInfo()

    for source_dir in source_dirs:
        source_path = Path(source_dir)
        if not source_path.exists():
            continue

        # Find all Python files
        for py_file in source_path.rglob("*.py"):
            # Skip test files and __pycache__
            if "__pycache__" in str(py_file):
                continue
            if omit_patterns:
                skip = False
                for pattern in omit_patterns:
                    if pattern in str(py_file):
                        skip = True
                        break
                if skip:
                    continue

            try:
                analysis = cov.analysis2(str(py_file))
                # analysis returns: (filename, executable, excluded, missing, formatted)
                filename, executable, excluded, missing, _ = analysis

                statements = len(executable)
                covered = statements - len(missing)

                if statements > 0:
                    file_cov = FileCoverage(
                        path=str(py_file.relative_to(source_path.parent)),
                        statements=statements,
                        covered=covered,
                        missing_lines=list(missing),
                    )
                    coverage_info.add_file(file_cov)
            except Exception:
                # File might not have been imported/executed
                coverage_info.add_uncovered_file(str(py_file.relative_to(source_path.parent)))

    return coverage_info


def run_suite_with_coverage(
    suite_class: Type[TestSuite],
    source_dirs: List[str],
    output_format: ReportFormat = ReportFormat.Markdown,
    output_file: Optional[str] = None,
    verbose: bool = True,
    omit_patterns: Optional[List[str]] = None,
    **runner_kwargs: Any,
) -> TestReport:
    """
    Run a test suite with coverage collection.

    Requires coverage.py to be installed.

    Args:
        suite_class: The TestSuite subclass to run
        source_dirs: Directories to measure coverage for
        output_format: Report output format (default: Markdown)
        output_file: Optional file path to write report
        verbose: Whether to print verbose output
        omit_patterns: Patterns to omit from coverage (e.g., ["test_", "__pycache__"])
        **runner_kwargs: Additional arguments for TestRunner

    Returns:
        TestReport with coverage data included

    Example:
        from data_bridge.test import run_suite_with_coverage, ReportFormat

        report = run_suite_with_coverage(
            MyTests,
            source_dirs=["python/data_bridge"],
            output_format=ReportFormat.Html,
            output_file="coverage_report.html"
        )
    """
    try:
        import coverage
    except ImportError:
        raise ImportError("coverage.py is required for coverage collection. Install with: pip install coverage")

    # Start coverage
    cov = coverage.Coverage(
        source=source_dirs,
        omit=omit_patterns or ["*test*", "*__pycache__*"],
    )
    cov.start()

    try:
        # Run the test suite
        report = run_suite(
            suite_class,
            output_format=output_format,
            verbose=verbose,
            **runner_kwargs,
        )
    finally:
        # Stop and save coverage
        cov.stop()
        cov.save()

    # Collect coverage data
    coverage_info = _collect_coverage_from_coveragepy(
        source_dirs,
        omit_patterns=omit_patterns or ["test_", "__pycache__"],
    )

    if coverage_info:
        report.set_coverage(coverage_info)

        if verbose:
            print(f"\nCoverage: {coverage_info.coverage_percent:.1f}% "
                  f"({coverage_info.covered_statements}/{coverage_info.total_statements} statements)")

    # Generate and optionally save report
    if output_file:
        reporter = Reporter(output_format)
        report_content = reporter.generate(report)

        with open(output_file, "w") as f:
            f.write(report_content)

        if verbose:
            print(f"Report written to: {output_file}")

    return report


def run_suites_with_coverage(
    suite_classes: List[Type[TestSuite]],
    source_dirs: List[str],
    output_format: ReportFormat = ReportFormat.Markdown,
    output_file: Optional[str] = None,
    verbose: bool = True,
    omit_patterns: Optional[List[str]] = None,
    **runner_kwargs: Any,
) -> List[TestReport]:
    """
    Run multiple test suites with combined coverage collection.

    Args:
        suite_classes: List of TestSuite subclasses to run
        source_dirs: Directories to measure coverage for
        output_format: Report output format
        output_file: Optional file path for combined report
        verbose: Whether to print verbose output
        omit_patterns: Patterns to omit from coverage
        **runner_kwargs: Additional arguments for TestRunner

    Returns:
        List of TestReports with coverage data
    """
    try:
        import coverage
    except ImportError:
        raise ImportError("coverage.py is required for coverage collection. Install with: pip install coverage")

    # Start coverage
    cov = coverage.Coverage(
        source=source_dirs,
        omit=omit_patterns or ["*test*", "*__pycache__*"],
    )
    cov.start()

    reports = []
    try:
        for suite_class in suite_classes:
            report = run_suite(
                suite_class,
                output_format=output_format,
                verbose=verbose,
                **runner_kwargs,
            )
            reports.append(report)
    finally:
        # Stop and save coverage
        cov.stop()
        cov.save()

    # Collect coverage data
    coverage_info = _collect_coverage_from_coveragepy(
        source_dirs,
        omit_patterns=omit_patterns or ["test_", "__pycache__"],
    )

    # Add coverage to all reports (shared coverage data)
    if coverage_info:
        for report in reports:
            report.set_coverage(coverage_info)

        if verbose:
            print(f"\nCoverage: {coverage_info.coverage_percent:.1f}% "
                  f"({coverage_info.covered_statements}/{coverage_info.total_statements} statements)")

    # Optionally combine reports into one file
    if output_file and reports:
        reporter = Reporter(output_format)
        combined = "\n\n---\n\n".join(reporter.generate(r) for r in reports)

        with open(output_file, "w") as f:
            f.write(combined)

        if verbose:
            print(f"\nCombined report written to: {output_file}")

    return reports
