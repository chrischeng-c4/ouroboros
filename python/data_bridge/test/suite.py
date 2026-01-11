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
from data_bridge import data_bridge as _rust_module
_test = _rust_module.test
ParameterValue = _test.ParameterValue
Parameter = _test.Parameter
ParametrizedTest = _test.ParametrizedTest
HookType = _test.HookType
HookRegistry = _test.HookRegistry


class ParametrizedTestInstance:
    """Wrapper for a single instance of a parametrized test"""

    def __init__(self, test_desc: TestDescriptor, param_set: Any, instance_name: str):
        self.test_desc = test_desc
        self.param_set = param_set
        self.instance_name = instance_name

    def get_meta(self):
        """Get TestMeta with parametrized name"""
        # Get the base meta from the test descriptor
        base_meta = self.test_desc.get_meta()

        # Import TestMeta from Rust bindings
        from data_bridge import data_bridge as _rust_module
        _test = _rust_module.test
        TestMeta = _test.TestMeta

        # Create a new TestMeta with the parametrized name
        meta = TestMeta(
            name=self.instance_name,
            test_type=base_meta.test_type,
            timeout=base_meta.timeout,
            tags=base_meta.tags,
        )

        # Update full_name to include parameters
        base_full_name = base_meta.full_name
        if '.' in base_full_name:
            parts = base_full_name.rsplit('.', 1)
            meta.full_name = f"{parts[0]}.{self.instance_name}"
        else:
            meta.full_name = self.instance_name

        # Copy skip reason if present
        if base_meta.is_skipped():
            meta.skip(base_meta.skip_reason or "Skipped")

        return meta

    @property
    def is_async(self) -> bool:
        return self.test_desc.is_async

    def __call__(self, suite_instance: Any) -> Any:
        """Execute the test with parameter injection"""
        import inspect

        # Get the test function signature to extract parameter names
        sig = inspect.signature(self.test_desc.func)
        param_names = [p for p in sig.parameters.keys() if p != 'self']

        # Build kwargs from ParameterSet by converting to dict first
        param_dict = self.param_set.to_dict()

        # Extract values for the specific parameters needed by this test
        kwargs = {}
        for param_name in param_names:
            if param_name in param_dict:
                kwargs[param_name] = param_dict[param_name]

        # Call the original test function with injected parameters
        # Return whatever the function returns (coroutine for async, value for sync)
        return self.test_desc.func(suite_instance, **kwargs)


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
        self._hook_registry: HookRegistry = HookRegistry()
        self._discover_tests()
        self._discover_hooks()

    def _discover_tests(self) -> None:
        """Discover all test methods in this suite"""
        self._tests = []

        for name in dir(self):
            attr = getattr(self.__class__, name, None)
            if isinstance(attr, TestDescriptor):
                # Check if test has parametrize decorators
                if hasattr(attr.func, '_parametrize'):
                    # Expand parametrized test into multiple instances
                    expanded_tests = self._expand_parametrized_test(attr)
                    self._tests.extend(expanded_tests)
                else:
                    # Regular test (no parametrization)
                    self._tests.append(attr)

    def _discover_hooks(self) -> None:
        """Discover and register lifecycle hooks"""
        # Class-level hooks (run once per test class)
        if hasattr(self.__class__, 'setup_class') and callable(getattr(self.__class__, 'setup_class')):
            setup_class = getattr(self.__class__, 'setup_class')
            # Only register if it's not the base TestSuite method
            if setup_class.__qualname__ != 'TestSuite.setup_class':
                self._hook_registry.register_hook(HookType.SetupClass, setup_class)

        if hasattr(self.__class__, 'teardown_class') and callable(getattr(self.__class__, 'teardown_class')):
            teardown_class = getattr(self.__class__, 'teardown_class')
            if teardown_class.__qualname__ != 'TestSuite.teardown_class':
                self._hook_registry.register_hook(HookType.TeardownClass, teardown_class)

        # Method-level hooks (run before/after each test)
        if hasattr(self.__class__, 'setup_method') and callable(getattr(self.__class__, 'setup_method')):
            setup_method = getattr(self.__class__, 'setup_method')
            if setup_method.__qualname__ != 'TestSuite.setup_method':
                self._hook_registry.register_hook(HookType.SetupMethod, setup_method)

        if hasattr(self.__class__, 'teardown_method') and callable(getattr(self.__class__, 'teardown_method')):
            teardown_method = getattr(self.__class__, 'teardown_method')
            if teardown_method.__qualname__ != 'TestSuite.teardown_method':
                self._hook_registry.register_hook(HookType.TeardownMethod, teardown_method)

    def _expand_parametrized_test(self, test_desc: TestDescriptor) -> List:
        """Expand a parametrized test into multiple test instances"""
        # Get parametrize metadata from the function
        parametrize_data = getattr(test_desc.func, '_parametrize', [])
        if not parametrize_data:
            return [test_desc]

        # Create ParametrizedTest and add parameters
        param_test = ParametrizedTest(test_desc.func.__name__)

        for param_name, param_values in parametrize_data:
            # Convert Python values to ParameterValue objects
            converted_values = []
            for value in param_values:
                # Auto-convert based on Python type
                # IMPORTANT: Check bool BEFORE int since bool is a subclass of int in Python
                if isinstance(value, bool):
                    converted_values.append(ParameterValue.bool(value))
                elif isinstance(value, int):
                    converted_values.append(ParameterValue.int(value))
                elif isinstance(value, float):
                    converted_values.append(ParameterValue.float(value))
                elif isinstance(value, str):
                    converted_values.append(ParameterValue.string(value))
                elif value is None:
                    converted_values.append(ParameterValue.none())
                else:
                    raise TypeError(f"Unsupported parameter type for value {value}: {type(value)}")

            # Create Parameter and add to ParametrizedTest
            param = Parameter(param_name, converted_values)
            param_test.add_parameter(param)

        # Expand into test instances
        expanded = param_test.expand()

        # Create ParametrizedTestInstance wrappers
        instances = []
        for instance_name, param_set in expanded:
            instances.append(ParametrizedTestInstance(test_desc, param_set, instance_name))

        return instances

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
        """Called once before all tests in the suite (legacy, use setup_class)"""
        pass

    async def teardown_suite(self) -> None:
        """Called once after all tests in the suite (legacy, use teardown_class)"""
        pass

    async def setup(self) -> None:
        """Called before each test (legacy, use setup_method)"""
        pass

    async def teardown(self) -> None:
        """Called after each test (legacy, use teardown_method)"""
        pass

    # New hook methods (pytest-compatible)

    async def setup_class(self) -> None:
        """Called once before all tests in the class"""
        pass

    async def teardown_class(self) -> None:
        """Called once after all tests in the class"""
        pass

    async def setup_method(self) -> None:
        """Called before each test method"""
        pass

    async def teardown_method(self) -> None:
        """Called after each test method"""
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

        # Run setup_class hooks (and legacy setup_suite)
        try:
            await self.setup_suite()  # Legacy support
            # Run setup_class hooks
            error = await self._hook_registry.run_hooks(HookType.SetupClass, self)
            if error:
                raise RuntimeError(error)
        except Exception as e:
            # If setup fails, mark all tests as error
            for test_desc in self._tests:
                meta = test_desc.get_meta()
                result = TestResult.error(meta, 0, f"Class setup failed: {e}")
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

            # Run setup_method hooks (and legacy setup)
            try:
                await self.setup()  # Legacy support
                # Run setup_method hooks
                error = await self._hook_registry.run_hooks(HookType.SetupMethod, self)
                if error:
                    raise RuntimeError(error)
            except Exception as e:
                result = TestResult.error(meta, 0, f"Method setup failed: {e}")
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

            # Run teardown_method hooks (and legacy teardown)
            try:
                # Run teardown_method hooks first
                error = await self._hook_registry.run_hooks(HookType.TeardownMethod, self)
                if error and verbose:
                    print(f"  WARNING: Method teardown error for {meta.name}: {error}")
                await self.teardown()  # Legacy support
            except Exception as e:
                # Log teardown error but don't override test result
                if verbose:
                    print(f"  WARNING: Method teardown failed for {meta.name}: {e}")

        # Run teardown_class hooks (and legacy teardown_suite)
        try:
            # Run teardown_class hooks first
            error = await self._hook_registry.run_hooks(HookType.TeardownClass, self)
            if error and verbose:
                print(f"WARNING: Class teardown error: {error}")
            await self.teardown_suite()  # Legacy support
        except Exception as e:
            if verbose:
                print(f"WARNING: Class teardown failed: {e}")

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
