//! data-bridge-test: Rust-powered test framework
//!
//! A custom test framework with Rust engine providing:
//! - Unit testing with decorator-based syntax
//! - Custom assertion API (expect-style)
//! - Benchmarking with timing statistics
//! - Profiling (CPU, memory, Rust-Python boundary)
//! - Stress testing (Tokio-powered concurrency)
//! - Security testing (fuzzing, injection detection)
//!
//! # Example
//!
//! ```python
//! from data_bridge.test import TestSuite, test, expect
//!
//! class MyTests(TestSuite):
//!     @test(timeout=5.0, tags=["unit"])
//!     async def test_example(self):
//!         expect(1 + 1).to_equal(2)
//! ```

pub mod assertions;
pub mod baseline;
pub mod benchmark;
pub mod discovery;
pub mod http_server;
pub mod performance;
pub mod reporter;
pub mod runner;
pub mod security;

// Re-export main types
pub use assertions::{expect, Expectation, AssertionError, AssertionResult};
pub use baseline::{
    BaselineMetadata, BaselineSnapshot, FileBaselineStore, GitMetadata,
    Improvement, Regression, RegressionDetector, RegressionReport,
    RegressionSeverity, RegressionSummary, RegressionThresholds,
};
pub use benchmark::{
    BenchmarkConfig, BenchmarkResult, BenchmarkStats, Benchmarker, compare_results,
    print_comparison_table, BenchmarkReport, BenchmarkReportGroup, BenchmarkEnvironment,
};
pub use discovery::{
    DiscoveryConfig, FileType, FileInfo, TestRegistry, BenchmarkRegistry, DiscoveryStats,
    walk_files, filter_files,
};
pub use http_server::{TestServer, TestServerHandle, RouteConfig};

// Re-export performance types (from performance module)
pub use performance::{
    // Boundary tracing
    BoundaryTracer, BoundaryTiming, BoundaryMetrics,
    // Profiling
    generate_flamegraph_svg, get_rss_bytes, FlamegraphData, GilContentionResult, GilTestConfig,
    MemoryProfile, MemorySnapshot, PhaseBreakdown, PhaseTiming, ProfileConfig, ProfilePhase,
    ProfileResult, Profiler,
};

pub use reporter::{Reporter, ReportFormat, TestReport, CoverageInfo, FileCoverage, EnvironmentInfo};
pub use runner::{TestRunner, TestMeta, TestResult, TestStatus, TestType};
pub use security::{
    AsyncFuzzConfig, AsyncFuzzer,
    FuzzConfig, FuzzCrash, FuzzResult, Fuzzer, MutationStrategy,
    PayloadCategory, PayloadDatabase,
    InjectionResult, InjectionTest, SqlInjectionTester,
};
