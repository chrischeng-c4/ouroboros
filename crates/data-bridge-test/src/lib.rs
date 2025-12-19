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
pub mod benchmark;
pub mod discovery;
pub mod http_server;
pub mod reporter;
pub mod runner;

// Re-export main types
pub use assertions::{Expectation, AssertionError, AssertionResult};
pub use benchmark::{
    BenchmarkConfig, BenchmarkResult, BenchmarkStats, Benchmarker, compare_results,
    print_comparison_table, BenchmarkReport, BenchmarkReportGroup, BenchmarkEnvironment,
};
pub use discovery::{
    DiscoveryConfig, FileType, FileInfo, TestRegistry, BenchmarkRegistry, DiscoveryStats,
    walk_files, filter_files,
};
pub use http_server::{TestServer, TestServerHandle, RouteConfig};
pub use reporter::{Reporter, ReportFormat, TestReport, CoverageInfo, FileCoverage, EnvironmentInfo};
pub use runner::{TestRunner, TestMeta, TestResult, TestStatus, TestType};
