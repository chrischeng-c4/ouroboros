//! Test framework PyO3 bindings
//!
//! Provides Python bindings for the ouroboros-qc crate.

use pyo3::prelude::*;

// Sub-modules
pub mod enums;
pub mod core;
pub mod runner;
pub mod assertions;
pub mod reporter;
pub mod coverage;
pub mod benchmark;
pub mod server;
pub mod discovery;
pub mod profiler;
pub mod parametrize;
pub mod fixtures;
pub mod hooks;


// =====================
// Module registration
// =====================

/// Register test module classes and functions
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Enums
    m.add_class::<enums::PyTestType>()?;
    m.add_class::<enums::PyTestStatus>()?;
    m.add_class::<enums::PyReportFormat>()?;
    m.add_class::<enums::PyFileType>()?;
    m.add_class::<enums::PyProfilePhase>()?;
    m.add_class::<enums::PyFixtureScope>()?;
    m.add_class::<enums::PyHookType>()?;

    // Core types
    m.add_class::<core::PyTestMeta>()?;
    m.add_class::<core::PyTestResult>()?;
    m.add_class::<core::PyTestSummary>()?;
    m.add_class::<runner::PyTestRunner>()?;

    // Assertions
    m.add_class::<assertions::PyExpectation>()?;
    m.add_function(wrap_pyfunction!(assertions::expect, m)?)?;

    // Reporter
    m.add_class::<reporter::PyReporter>()?;
    m.add_class::<reporter::PyTestReport>()?;

    // Coverage
    m.add_class::<coverage::PyFileCoverage>()?;
    m.add_class::<coverage::PyCoverageInfo>()?;

    // Benchmark
    m.add_class::<benchmark::PyBenchmarkStats>()?;
    m.add_class::<benchmark::PyBenchmarkResult>()?;
    m.add_class::<benchmark::PyBenchmarkConfig>()?;
    m.add_function(wrap_pyfunction!(benchmark::compare_benchmarks, m)?)?;
    m.add_function(wrap_pyfunction!(benchmark::print_comparison_table, m)?)?;

    // Benchmark Report
    m.add_class::<benchmark::PyBenchmarkEnvironment>()?;
    m.add_class::<benchmark::PyBenchmarkReportGroup>()?;
    m.add_class::<benchmark::PyBenchmarkReport>()?;

    // Test Server
    m.add_class::<server::PyTestServer>()?;
    m.add_class::<server::PyTestServerHandle>()?;

    // Discovery
    m.add_class::<discovery::PyFileInfo>()?;
    m.add_class::<discovery::PyDiscoveryConfig>()?;
    m.add_class::<discovery::PyTestRegistry>()?;
    m.add_class::<discovery::PyBenchmarkRegistry>()?;
    m.add_class::<discovery::PyDiscoveryStats>()?;
    m.add_function(wrap_pyfunction!(discovery::discover_files, m)?)?;
    m.add_function(wrap_pyfunction!(discovery::filter_files_by_pattern, m)?)?;

    // Profiler
    m.add_class::<profiler::PyPhaseTiming>()?;
    m.add_class::<profiler::PyPhaseBreakdown>()?;
    m.add_class::<profiler::PyGilTestConfig>()?;
    m.add_class::<profiler::PyGilContentionResult>()?;
    m.add_class::<profiler::PyMemorySnapshot>()?;
    m.add_class::<profiler::PyMemoryProfile>()?;
    m.add_class::<profiler::PyFlamegraphData>()?;
    m.add_class::<profiler::PyProfileResult>()?;
    m.add_class::<profiler::PyProfileConfig>()?;
    m.add_function(wrap_pyfunction!(profiler::generate_flamegraph, m)?)?;

    // Fixtures
    m.add_class::<fixtures::PyFixtureMeta>()?;
    m.add_class::<fixtures::PyFixtureRegistry>()?;

    // Parametrize
    m.add_class::<parametrize::PyParameterValue>()?;
    m.add_class::<parametrize::PyParameterSet>()?;
    m.add_class::<parametrize::PyParameter>()?;
    m.add_class::<parametrize::PyParametrizedTest>()?;

    // Hooks
    m.add_class::<hooks::PyHookRegistry>()?;

    Ok(())
}
