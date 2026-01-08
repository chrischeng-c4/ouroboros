//! Test reporter - generates reports in various formats

use crate::runner::{TestResult, TestStatus, TestSummary, TestType};
use serde::{Deserialize, Serialize};
use std::fmt::Write as FmtWrite;

/// Report output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ReportFormat {
    /// Markdown format (human-readable)
    #[default]
    Markdown,
    /// HTML format (interactive report)
    Html,
    /// JSON format (machine-parseable)
    Json,
    /// YAML format (human-readable, machine-parseable)
    Yaml,
    /// JUnit XML format (CI integration)
    JUnit,
    /// Console format (colored terminal output)
    Console,
}


impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportFormat::Markdown => write!(f, "markdown"),
            ReportFormat::Html => write!(f, "html"),
            ReportFormat::Json => write!(f, "json"),
            ReportFormat::Yaml => write!(f, "yaml"),
            ReportFormat::JUnit => write!(f, "junit"),
            ReportFormat::Console => write!(f, "console"),
        }
    }
}

/// Full test report with all results and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    /// Suite name
    pub suite_name: String,
    /// Report generation timestamp
    pub generated_at: String,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Summary statistics
    pub summary: TestSummary,
    /// Individual test results
    pub results: Vec<TestResult>,
    /// Environment info
    pub environment: EnvironmentInfo,
    /// Code coverage info (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageInfo>,
}

/// Environment information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Python version
    pub python_version: Option<String>,
    /// Rust version
    pub rust_version: Option<String>,
    /// Platform (OS)
    pub platform: Option<String>,
    /// Hostname
    pub hostname: Option<String>,
}

/// Coverage information for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    /// File path (relative to project root)
    pub path: String,
    /// Total number of statements
    pub statements: usize,
    /// Number of covered statements
    pub covered: usize,
    /// Lines that are not covered (line numbers)
    pub missing_lines: Vec<usize>,
    /// Coverage percentage for this file
    pub coverage_percent: f64,
}

/// Overall coverage summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageInfo {
    /// Total statements across all files
    pub total_statements: usize,
    /// Total covered statements
    pub covered_statements: usize,
    /// Overall coverage percentage
    pub coverage_percent: f64,
    /// Per-file coverage data
    pub files: Vec<FileCoverage>,
    /// Files with 0% coverage (not imported/executed)
    pub uncovered_files: Vec<String>,
}

impl TestReport {
    /// Create a new test report
    pub fn new(suite_name: impl Into<String>, results: Vec<TestResult>) -> Self {
        let suite_name = suite_name.into();
        let summary = calculate_summary(&results);
        let duration_ms = summary.total_duration_ms;

        Self {
            suite_name,
            generated_at: chrono::Utc::now().to_rfc3339(),
            duration_ms,
            summary,
            results,
            environment: EnvironmentInfo::default(),
            coverage: None,
        }
    }

    /// Set environment info
    pub fn with_environment(mut self, env: EnvironmentInfo) -> Self {
        self.environment = env;
        self
    }

    /// Set coverage info
    pub fn with_coverage(mut self, coverage: CoverageInfo) -> Self {
        self.coverage = Some(coverage);
        self
    }

    /// Set coverage info (mutable)
    pub fn set_coverage(&mut self, coverage: CoverageInfo) {
        self.coverage = Some(coverage);
    }

    /// Get results by test type
    pub fn results_by_type(&self, test_type: TestType) -> Vec<&TestResult> {
        self.results
            .iter()
            .filter(|r| r.meta.test_type == test_type)
            .collect()
    }

    /// Get failed results
    pub fn failed_results(&self) -> Vec<&TestResult> {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Failed || r.status == TestStatus::Error)
            .collect()
    }
}

/// Calculate summary from results
fn calculate_summary(results: &[TestResult]) -> TestSummary {
    let mut summary = TestSummary::default();

    for result in results {
        match result.status {
            TestStatus::Passed => summary.passed += 1,
            TestStatus::Failed => summary.failed += 1,
            TestStatus::Skipped => summary.skipped += 1,
            TestStatus::Error => summary.errors += 1,
        }
        summary.total_duration_ms += result.duration_ms;
    }

    summary.total = results.len();
    summary
}

/// Test reporter - generates reports in various formats
#[derive(Debug)]
pub struct Reporter {
    format: ReportFormat,
}

impl Reporter {
    /// Create a new reporter with specified format
    pub fn new(format: ReportFormat) -> Self {
        Self { format }
    }

    /// Create markdown reporter
    pub fn markdown() -> Self {
        Self::new(ReportFormat::Markdown)
    }

    /// Create JSON reporter
    pub fn json() -> Self {
        Self::new(ReportFormat::Json)
    }

    /// Create HTML reporter
    pub fn html() -> Self {
        Self::new(ReportFormat::Html)
    }

    /// Create JUnit reporter
    pub fn junit() -> Self {
        Self::new(ReportFormat::JUnit)
    }

    /// Create YAML reporter
    pub fn yaml() -> Self {
        Self::new(ReportFormat::Yaml)
    }

    /// Create Console reporter
    pub fn console() -> Self {
        Self::new(ReportFormat::Console)
    }

    /// Generate report string
    pub fn generate(&self, report: &TestReport) -> String {
        match self.format {
            ReportFormat::Markdown => self.generate_markdown(report),
            ReportFormat::Html => self.generate_html(report),
            ReportFormat::Json => self.generate_json(report),
            ReportFormat::Yaml => self.generate_yaml(report),
            ReportFormat::JUnit => self.generate_junit(report),
            ReportFormat::Console => self.generate_console(report),
        }
    }

    /// Generate Markdown report
    fn generate_markdown(&self, report: &TestReport) -> String {
        let mut output = String::new();

        // Header
        writeln!(output, "# Test Report: {}", report.suite_name).unwrap();
        writeln!(output).unwrap();

        // Metadata
        let status_emoji = if report.summary.all_passed() {
            "✅ PASSED"
        } else {
            "❌ FAILED"
        };
        writeln!(
            output,
            "**Date**: {} | **Duration**: {:.2}s | **Status**: {} ({}/{})",
            &report.generated_at[..10],
            report.duration_ms as f64 / 1000.0,
            status_emoji,
            report.summary.passed,
            report.summary.total
        )
        .unwrap();
        writeln!(output).unwrap();

        // Summary table
        writeln!(output, "## Summary").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "| Type | Passed | Failed | Skipped | Duration |").unwrap();
        writeln!(output, "|------|--------|--------|---------|----------|").unwrap();

        for test_type in [TestType::Unit, TestType::Profile, TestType::Stress, TestType::Security] {
            let results = report.results_by_type(test_type);
            if !results.is_empty() {
                let passed = results.iter().filter(|r| r.status == TestStatus::Passed).count();
                let failed = results
                    .iter()
                    .filter(|r| r.status == TestStatus::Failed || r.status == TestStatus::Error)
                    .count();
                let skipped = results.iter().filter(|r| r.status == TestStatus::Skipped).count();
                let duration: u64 = results.iter().map(|r| r.duration_ms).sum();

                writeln!(
                    output,
                    "| {} | {} | {} | {} | {:.2}s |",
                    test_type,
                    passed,
                    failed,
                    skipped,
                    duration as f64 / 1000.0
                )
                .unwrap();
            }
        }
        writeln!(output).unwrap();

        // Coverage section
        if let Some(ref coverage) = report.coverage {
            writeln!(output, "## Coverage").unwrap();
            writeln!(output).unwrap();

            // Coverage badge/summary
            let coverage_emoji = if coverage.coverage_percent >= 80.0 {
                "🟢"
            } else if coverage.coverage_percent >= 60.0 {
                "🟡"
            } else {
                "🔴"
            };
            writeln!(
                output,
                "{} **{:.1}%** coverage ({}/{} statements)",
                coverage_emoji,
                coverage.coverage_percent,
                coverage.covered_statements,
                coverage.total_statements
            )
            .unwrap();
            writeln!(output).unwrap();

            // Per-file coverage table (only show files with < 100% coverage)
            let incomplete_files: Vec<_> = coverage
                .files
                .iter()
                .filter(|f| f.coverage_percent < 100.0)
                .collect();

            if !incomplete_files.is_empty() {
                writeln!(output, "### Files with Incomplete Coverage").unwrap();
                writeln!(output).unwrap();
                writeln!(output, "| File | Coverage | Statements | Missing Lines |").unwrap();
                writeln!(output, "|------|----------|------------|---------------|").unwrap();

                for file in incomplete_files {
                    let missing = if file.missing_lines.len() > 5 {
                        format!(
                            "{}, ... ({} more)",
                            file.missing_lines[..5]
                                .iter()
                                .map(|n| n.to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                            file.missing_lines.len() - 5
                        )
                    } else {
                        file.missing_lines
                            .iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    writeln!(
                        output,
                        "| {} | {:.1}% | {}/{} | {} |",
                        file.path, file.coverage_percent, file.covered, file.statements, missing
                    )
                    .unwrap();
                }
                writeln!(output).unwrap();
            }

            // Uncovered files
            if !coverage.uncovered_files.is_empty() {
                writeln!(output, "### Uncovered Files (0%)").unwrap();
                writeln!(output).unwrap();
                for file in &coverage.uncovered_files {
                    writeln!(output, "- {}", file).unwrap();
                }
                writeln!(output).unwrap();
            }
        }

        // Failed tests section
        let failed = report.failed_results();
        if !failed.is_empty() {
            writeln!(output, "## Failed Tests").unwrap();
            writeln!(output).unwrap();

            for result in failed {
                writeln!(output, "### {} ❌", result.meta.name).unwrap();
                writeln!(output).unwrap();
                if let Some(ref error) = result.error {
                    writeln!(output, "**Error**: {}", error).unwrap();
                }
                if let Some(ref trace) = result.stack_trace {
                    writeln!(output, "```").unwrap();
                    writeln!(output, "{}", trace).unwrap();
                    writeln!(output, "```").unwrap();
                }
                writeln!(output).unwrap();
            }
        }

        // Profile metrics section
        let profile_results: Vec<_> = report
            .results
            .iter()
            .filter(|r| r.profile_metrics.is_some())
            .collect();

        if !profile_results.is_empty() {
            writeln!(output, "## Profile Metrics").unwrap();
            writeln!(output).unwrap();
            writeln!(
                output,
                "| Test | Iterations | Avg CPU (ms) | Peak Memory | Boundary Overhead |"
            )
            .unwrap();
            writeln!(output, "|------|------------|--------------|-------------|-------------------|").unwrap();

            for result in profile_results {
                if let Some(ref metrics) = result.profile_metrics {
                    writeln!(
                        output,
                        "| {} | {} | {:.2} | {} | {:.2}ms |",
                        result.meta.name,
                        metrics.iterations,
                        metrics.avg_cpu_time_ms,
                        format_bytes(metrics.peak_memory_bytes),
                        metrics.boundary_overhead_ms
                    )
                    .unwrap();
                }
            }
            writeln!(output).unwrap();
        }

        // Stress metrics section
        let stress_results: Vec<_> = report
            .results
            .iter()
            .filter(|r| r.stress_metrics.is_some())
            .collect();

        if !stress_results.is_empty() {
            writeln!(output, "## Stress Test Results").unwrap();
            writeln!(output).unwrap();
            writeln!(
                output,
                "| Test | RPS | P50 (ms) | P95 (ms) | P99 (ms) | Error Rate |"
            )
            .unwrap();
            writeln!(output, "|------|-----|----------|----------|----------|------------|").unwrap();

            for result in stress_results {
                if let Some(ref metrics) = result.stress_metrics {
                    writeln!(
                        output,
                        "| {} | {:.1} | {} | {} | {} | {:.2}% |",
                        result.meta.name,
                        metrics.rps,
                        metrics.latency_p50_ms,
                        metrics.latency_p95_ms,
                        metrics.latency_p99_ms,
                        metrics.error_rate * 100.0
                    )
                    .unwrap();
                }
            }
            writeln!(output).unwrap();
        }

        // All tests details
        writeln!(output, "## All Tests").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "| Status | Test | Duration | Type |").unwrap();
        writeln!(output, "|--------|------|----------|------|").unwrap();

        for result in &report.results {
            let status_icon = match result.status {
                TestStatus::Passed => "✅",
                TestStatus::Failed => "❌",
                TestStatus::Skipped => "⏭️",
                TestStatus::Error => "💥",
            };
            writeln!(
                output,
                "| {} | {} | {}ms | {} |",
                status_icon, result.meta.name, result.duration_ms, result.meta.test_type
            )
            .unwrap();
        }
        writeln!(output).unwrap();

        // Environment info
        if report.environment.python_version.is_some() || report.environment.platform.is_some() {
            writeln!(output, "## Environment").unwrap();
            writeln!(output).unwrap();
            if let Some(ref py) = report.environment.python_version {
                writeln!(output, "- **Python**: {}", py).unwrap();
            }
            if let Some(ref rust) = report.environment.rust_version {
                writeln!(output, "- **Rust**: {}", rust).unwrap();
            }
            if let Some(ref platform) = report.environment.platform {
                writeln!(output, "- **Platform**: {}", platform).unwrap();
            }
            if let Some(ref host) = report.environment.hostname {
                writeln!(output, "- **Hostname**: {}", host).unwrap();
            }
        }

        output
    }

    /// Generate JSON report
    fn generate_json(&self, report: &TestReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate HTML report
    fn generate_html(&self, report: &TestReport) -> String {
        let mut output = String::new();

        writeln!(output, "<!DOCTYPE html>").unwrap();
        writeln!(output, "<html lang=\"en\">").unwrap();
        writeln!(output, "<head>").unwrap();
        writeln!(output, "  <meta charset=\"UTF-8\">").unwrap();
        writeln!(output, "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">").unwrap();
        writeln!(output, "  <title>Test Report: {}</title>", report.suite_name).unwrap();
        writeln!(output, "  <style>").unwrap();
        writeln!(output, "    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #f5f5f5; }}").unwrap();
        writeln!(output, "    .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}").unwrap();
        writeln!(output, "    h1 {{ color: #333; border-bottom: 2px solid #4CAF50; padding-bottom: 10px; }}").unwrap();
        writeln!(output, "    h2 {{ color: #555; margin-top: 30px; }}").unwrap();
        writeln!(output, "    .status-passed {{ color: #4CAF50; }}").unwrap();
        writeln!(output, "    .status-failed {{ color: #f44336; }}").unwrap();
        writeln!(output, "    .status-skipped {{ color: #ff9800; }}").unwrap();
        writeln!(output, "    .status-error {{ color: #9c27b0; }}").unwrap();
        writeln!(output, "    table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}").unwrap();
        writeln!(output, "    th, td {{ border: 1px solid #ddd; padding: 12px; text-align: left; }}").unwrap();
        writeln!(output, "    th {{ background: #f8f9fa; font-weight: 600; }}").unwrap();
        writeln!(output, "    tr:hover {{ background: #f5f5f5; }}").unwrap();
        writeln!(output, "    .summary {{ display: flex; gap: 20px; margin: 20px 0; }}").unwrap();
        writeln!(output, "    .stat {{ padding: 20px; border-radius: 8px; text-align: center; flex: 1; }}").unwrap();
        writeln!(output, "    .stat-passed {{ background: #e8f5e9; }}").unwrap();
        writeln!(output, "    .stat-failed {{ background: #ffebee; }}").unwrap();
        writeln!(output, "    .stat-skipped {{ background: #fff3e0; }}").unwrap();
        writeln!(output, "    .stat-value {{ font-size: 2em; font-weight: bold; }}").unwrap();
        writeln!(output, "    .error-box {{ background: #ffebee; padding: 15px; border-radius: 4px; margin: 10px 0; }}").unwrap();
        writeln!(output, "    pre {{ background: #263238; color: #aed581; padding: 15px; border-radius: 4px; overflow-x: auto; }}").unwrap();
        writeln!(output, "  </style>").unwrap();
        writeln!(output, "</head>").unwrap();
        writeln!(output, "<body>").unwrap();
        writeln!(output, "<div class=\"container\">").unwrap();

        // Header
        let status_class = if report.summary.all_passed() {
            "status-passed"
        } else {
            "status-failed"
        };
        writeln!(
            output,
            "  <h1>Test Report: {} <span class=\"{}\">({})</span></h1>",
            report.suite_name,
            status_class,
            if report.summary.all_passed() { "PASSED" } else { "FAILED" }
        )
        .unwrap();
        writeln!(
            output,
            "  <p>Generated: {} | Duration: {:.2}s</p>",
            &report.generated_at[..10],
            report.duration_ms as f64 / 1000.0
        )
        .unwrap();

        // Summary cards
        writeln!(output, "  <div class=\"summary\">").unwrap();
        writeln!(
            output,
            "    <div class=\"stat stat-passed\"><div class=\"stat-value\">{}</div><div>Passed</div></div>",
            report.summary.passed
        )
        .unwrap();
        writeln!(
            output,
            "    <div class=\"stat stat-failed\"><div class=\"stat-value\">{}</div><div>Failed</div></div>",
            report.summary.failed + report.summary.errors
        )
        .unwrap();
        writeln!(
            output,
            "    <div class=\"stat stat-skipped\"><div class=\"stat-value\">{}</div><div>Skipped</div></div>",
            report.summary.skipped
        )
        .unwrap();
        writeln!(output, "  </div>").unwrap();

        // Coverage section
        if let Some(ref coverage) = report.coverage {
            let coverage_color = if coverage.coverage_percent >= 80.0 {
                "#4CAF50"
            } else if coverage.coverage_percent >= 60.0 {
                "#ff9800"
            } else {
                "#f44336"
            };

            writeln!(output, "  <h2>Coverage</h2>").unwrap();
            writeln!(
                output,
                "  <div style=\"background: linear-gradient(90deg, {} {}%, #e0e0e0 {}%); padding: 15px; border-radius: 8px; margin: 20px 0;\">",
                coverage_color,
                coverage.coverage_percent,
                coverage.coverage_percent
            )
            .unwrap();
            writeln!(
                output,
                "    <span style=\"font-size: 1.5em; font-weight: bold; color: white; text-shadow: 1px 1px 2px rgba(0,0,0,0.5);\">{:.1}%</span>",
                coverage.coverage_percent
            )
            .unwrap();
            writeln!(
                output,
                "    <span style=\"color: white; text-shadow: 1px 1px 2px rgba(0,0,0,0.5);\"> ({}/{} statements)</span>",
                coverage.covered_statements,
                coverage.total_statements
            )
            .unwrap();
            writeln!(output, "  </div>").unwrap();

            // Per-file coverage table
            let incomplete_files: Vec<_> = coverage
                .files
                .iter()
                .filter(|f| f.coverage_percent < 100.0)
                .collect();

            if !incomplete_files.is_empty() {
                writeln!(output, "  <h3>Files with Incomplete Coverage</h3>").unwrap();
                writeln!(output, "  <table>").unwrap();
                writeln!(
                    output,
                    "    <tr><th>File</th><th>Coverage</th><th>Statements</th><th>Missing Lines</th></tr>"
                )
                .unwrap();

                for file in incomplete_files {
                    let file_color = if file.coverage_percent >= 80.0 {
                        "#4CAF50"
                    } else if file.coverage_percent >= 60.0 {
                        "#ff9800"
                    } else {
                        "#f44336"
                    };

                    let missing = if file.missing_lines.len() > 5 {
                        format!(
                            "{}, ... ({} more)",
                            file.missing_lines[..5]
                                .iter()
                                .map(|n| n.to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                            file.missing_lines.len() - 5
                        )
                    } else {
                        file.missing_lines
                            .iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    writeln!(
                        output,
                        "    <tr><td>{}</td><td style=\"color: {}\">{:.1}%</td><td>{}/{}</td><td><code>{}</code></td></tr>",
                        file.path, file_color, file.coverage_percent, file.covered, file.statements, missing
                    )
                    .unwrap();
                }

                writeln!(output, "  </table>").unwrap();
            }

            // Uncovered files
            if !coverage.uncovered_files.is_empty() {
                writeln!(output, "  <h3>Uncovered Files (0%)</h3>").unwrap();
                writeln!(output, "  <ul>").unwrap();
                for file in &coverage.uncovered_files {
                    writeln!(output, "    <li style=\"color: #f44336;\">{}</li>", file).unwrap();
                }
                writeln!(output, "  </ul>").unwrap();
            }
        }

        // Results table
        writeln!(output, "  <h2>Test Results</h2>").unwrap();
        writeln!(output, "  <table>").unwrap();
        writeln!(output, "    <tr><th>Status</th><th>Test</th><th>Type</th><th>Duration</th><th>Details</th></tr>").unwrap();

        for result in &report.results {
            let (status_icon, status_class) = match result.status {
                TestStatus::Passed => ("✅", "status-passed"),
                TestStatus::Failed => ("❌", "status-failed"),
                TestStatus::Skipped => ("⏭️", "status-skipped"),
                TestStatus::Error => ("💥", "status-error"),
            };
            let error_detail = result
                .error
                .as_ref()
                .map(|e| e.chars().take(50).collect::<String>())
                .unwrap_or_default();

            writeln!(
                output,
                "    <tr><td class=\"{}\">{}</td><td>{}</td><td>{}</td><td>{}ms</td><td>{}</td></tr>",
                status_class, status_icon, result.meta.name, result.meta.test_type, result.duration_ms, error_detail
            )
            .unwrap();
        }

        writeln!(output, "  </table>").unwrap();

        // Failed tests details
        let failed = report.failed_results();
        if !failed.is_empty() {
            writeln!(output, "  <h2>Failed Tests Details</h2>").unwrap();
            for result in failed {
                writeln!(output, "  <div class=\"error-box\">").unwrap();
                writeln!(output, "    <h3>{}</h3>", result.meta.name).unwrap();
                if let Some(ref error) = result.error {
                    writeln!(output, "    <p><strong>Error:</strong> {}</p>", error).unwrap();
                }
                if let Some(ref trace) = result.stack_trace {
                    writeln!(output, "    <pre>{}</pre>", trace).unwrap();
                }
                writeln!(output, "  </div>").unwrap();
            }
        }

        writeln!(output, "</div>").unwrap();
        writeln!(output, "</body>").unwrap();
        writeln!(output, "</html>").unwrap();

        output
    }

    /// Generate JUnit XML report (for CI integration)
    fn generate_junit(&self, report: &TestReport) -> String {
        let mut output = String::new();

        writeln!(output, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
        writeln!(
            output,
            "<testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\" time=\"{:.3}\" timestamp=\"{}\">",
            escape_xml(&report.suite_name),
            report.summary.total,
            report.summary.failed,
            report.summary.errors,
            report.summary.skipped,
            report.duration_ms as f64 / 1000.0,
            report.generated_at
        )
        .unwrap();

        for result in &report.results {
            write!(
                output,
                "  <testcase name=\"{}\" classname=\"{}\" time=\"{:.3}\"",
                escape_xml(&result.meta.name),
                escape_xml(&result.meta.full_name),
                result.duration_ms as f64 / 1000.0
            )
            .unwrap();

            // Add file and line if available
            if let Some(ref file) = result.meta.file_path {
                write!(output, " file=\"{}\"", escape_xml(file)).unwrap();
            }
            if let Some(line) = result.meta.line_number {
                write!(output, " line=\"{}\"", line).unwrap();
            }

            writeln!(output, ">").unwrap();

            match result.status {
                TestStatus::Failed => {
                    let message = result.error.as_deref().unwrap_or("Assertion failed");
                    writeln!(
                        output,
                        "    <failure message=\"{}\">{}</failure>",
                        escape_xml(message),
                        escape_xml(result.stack_trace.as_deref().unwrap_or(""))
                    )
                    .unwrap();
                }
                TestStatus::Error => {
                    let message = result.error.as_deref().unwrap_or("Test error");
                    writeln!(
                        output,
                        "    <error type=\"Error\" message=\"{}\">{}</error>",
                        escape_xml(message),
                        escape_xml(result.stack_trace.as_deref().unwrap_or(""))
                    )
                    .unwrap();
                }
                TestStatus::Skipped => {
                    let message = result.error.as_deref().unwrap_or("Skipped");
                    writeln!(output, "    <skipped message=\"{}\" />", escape_xml(message)).unwrap();
                }
                TestStatus::Passed => {}
            }

            writeln!(output, "  </testcase>").unwrap();
        }

        writeln!(output, "</testsuite>").unwrap();

        output
    }

    /// Generate YAML report
    fn generate_yaml(&self, report: &TestReport) -> String {
        let mut output = String::new();

        writeln!(output, "# Test Report").unwrap();
        writeln!(output, "suite_name: \"{}\"", report.suite_name).unwrap();
        writeln!(output, "generated_at: \"{}\"", report.generated_at).unwrap();
        writeln!(output, "duration_ms: {}", report.duration_ms).unwrap();
        writeln!(output).unwrap();

        // Summary
        writeln!(output, "summary:").unwrap();
        writeln!(output, "  total: {}", report.summary.total).unwrap();
        writeln!(output, "  passed: {}", report.summary.passed).unwrap();
        writeln!(output, "  failed: {}", report.summary.failed).unwrap();
        writeln!(output, "  skipped: {}", report.summary.skipped).unwrap();
        writeln!(output, "  errors: {}", report.summary.errors).unwrap();
        writeln!(output, "  total_duration_ms: {}", report.summary.total_duration_ms).unwrap();
        writeln!(output, "  pass_rate: {:.2}", report.summary.pass_rate()).unwrap();
        writeln!(output).unwrap();

        // Coverage (if present)
        if let Some(ref coverage) = report.coverage {
            writeln!(output, "coverage:").unwrap();
            writeln!(output, "  total_statements: {}", coverage.total_statements).unwrap();
            writeln!(output, "  covered_statements: {}", coverage.covered_statements).unwrap();
            writeln!(output, "  coverage_percent: {:.2}", coverage.coverage_percent).unwrap();
            if !coverage.files.is_empty() {
                writeln!(output, "  files:").unwrap();
                for file in &coverage.files {
                    writeln!(output, "    - path: \"{}\"", file.path).unwrap();
                    writeln!(output, "      statements: {}", file.statements).unwrap();
                    writeln!(output, "      covered: {}", file.covered).unwrap();
                    writeln!(output, "      coverage_percent: {:.2}", file.coverage_percent).unwrap();
                    if !file.missing_lines.is_empty() {
                        let missing: Vec<String> = file.missing_lines.iter().map(|n| n.to_string()).collect();
                        writeln!(output, "      missing_lines: [{}]", missing.join(", ")).unwrap();
                    }
                }
            }
            if !coverage.uncovered_files.is_empty() {
                writeln!(output, "  uncovered_files:").unwrap();
                for file in &coverage.uncovered_files {
                    writeln!(output, "    - \"{}\"", file).unwrap();
                }
            }
            writeln!(output).unwrap();
        }

        // Results
        writeln!(output, "results:").unwrap();
        for result in &report.results {
            writeln!(output, "  - name: \"{}\"", result.meta.name).unwrap();
            writeln!(output, "    status: \"{}\"", result.status).unwrap();
            writeln!(output, "    duration_ms: {}", result.duration_ms).unwrap();
            writeln!(output, "    test_type: \"{}\"", result.meta.test_type).unwrap();
            if let Some(ref error) = result.error {
                writeln!(output, "    error: \"{}\"", error.replace('"', "\\\"")).unwrap();
            }
        }

        // Environment
        if report.environment.python_version.is_some() || report.environment.platform.is_some() {
            writeln!(output).unwrap();
            writeln!(output, "environment:").unwrap();
            if let Some(ref py) = report.environment.python_version {
                writeln!(output, "  python_version: \"{}\"", py).unwrap();
            }
            if let Some(ref rust) = report.environment.rust_version {
                writeln!(output, "  rust_version: \"{}\"", rust).unwrap();
            }
            if let Some(ref platform) = report.environment.platform {
                writeln!(output, "  platform: \"{}\"", platform).unwrap();
            }
            if let Some(ref host) = report.environment.hostname {
                writeln!(output, "  hostname: \"{}\"", host).unwrap();
            }
        }

        output
    }

    /// Generate Console report (colored terminal output)
    fn generate_console(&self, report: &TestReport) -> String {
        let mut output = String::new();

        // ANSI color codes
        const RESET: &str = "\x1b[0m";
        const BOLD: &str = "\x1b[1m";
        const GREEN: &str = "\x1b[32m";
        const RED: &str = "\x1b[31m";
        const YELLOW: &str = "\x1b[33m";
        const CYAN: &str = "\x1b[36m";
        const DIM: &str = "\x1b[2m";

        // Header
        let status_color = if report.summary.all_passed() { GREEN } else { RED };
        let status_text = if report.summary.all_passed() { "PASSED" } else { "FAILED" };
        writeln!(
            output,
            "\n{}{}═══ Test Report: {} ═══{}",
            BOLD, status_color, report.suite_name, RESET
        ).unwrap();
        writeln!(
            output,
            "{}Status: {}{}{}  |  Duration: {:.2}s  |  Date: {}{}",
            DIM, status_color, status_text, DIM,
            report.duration_ms as f64 / 1000.0,
            &report.generated_at[..10],
            RESET
        ).unwrap();

        // Summary bar
        writeln!(output).unwrap();
        writeln!(output, "{}{}Summary:{}", BOLD, CYAN, RESET).unwrap();
        writeln!(
            output,
            "  {}✓ Passed: {}{}  {}✗ Failed: {}{}  {}⊘ Skipped: {}{}  {}! Errors: {}{}",
            GREEN, report.summary.passed, RESET,
            RED, report.summary.failed, RESET,
            YELLOW, report.summary.skipped, RESET,
            RED, report.summary.errors, RESET
        ).unwrap();

        // Coverage (if present)
        if let Some(ref coverage) = report.coverage {
            writeln!(output).unwrap();
            writeln!(output, "{}{}Coverage:{}", BOLD, CYAN, RESET).unwrap();

            let cov_color = if coverage.coverage_percent >= 80.0 {
                GREEN
            } else if coverage.coverage_percent >= 60.0 {
                YELLOW
            } else {
                RED
            };

            // Progress bar
            let bar_width = 40;
            let filled = (coverage.coverage_percent / 100.0 * bar_width as f64) as usize;
            let empty = bar_width - filled;
            writeln!(
                output,
                "  {}[{}{}{}] {}{:.1}%{} ({}/{} statements)",
                DIM,
                cov_color, "█".repeat(filled), "░".repeat(empty),
                cov_color, coverage.coverage_percent, RESET,
                coverage.covered_statements, coverage.total_statements
            ).unwrap();

            // Low coverage files
            let low_coverage: Vec<_> = coverage.files.iter()
                .filter(|f| f.coverage_percent < 50.0)
                .take(5)
                .collect();
            if !low_coverage.is_empty() {
                writeln!(output, "  {}Low coverage files:{}", DIM, RESET).unwrap();
                for file in low_coverage {
                    writeln!(
                        output,
                        "    {}{}{} {:.1}%{}",
                        RED, file.path, DIM, file.coverage_percent, RESET
                    ).unwrap();
                }
            }
        }

        // Test results
        writeln!(output).unwrap();
        writeln!(output, "{}{}Test Results:{}", BOLD, CYAN, RESET).unwrap();
        for result in &report.results {
            let (icon, color) = match result.status {
                TestStatus::Passed => ("✓", GREEN),
                TestStatus::Failed => ("✗", RED),
                TestStatus::Skipped => ("⊘", YELLOW),
                TestStatus::Error => ("!", RED),
            };
            writeln!(
                output,
                "  {}{} {}{}  {}{}ms{}",
                color, icon, result.meta.name, RESET,
                DIM, result.duration_ms, RESET
            ).unwrap();

            if let Some(ref error) = result.error {
                writeln!(output, "    {}→ {}{}",DIM, error, RESET).unwrap();
            }
        }

        // Footer
        writeln!(output).unwrap();
        writeln!(
            output,
            "{}────────────────────────────────────────{}",
            DIM, RESET
        ).unwrap();
        writeln!(
            output,
            "{}Total: {} tests  |  Pass rate: {:.1}%{}",
            DIM, report.summary.total, report.summary.pass_rate() * 100.0, RESET
        ).unwrap();

        output
    }
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::TestMeta;

    #[test]
    fn test_report_creation() {
        let results = vec![
            TestResult::passed(TestMeta::new("test_one"), 100),
            TestResult::passed(TestMeta::new("test_two"), 200),
            TestResult::failed(TestMeta::new("test_three"), 50, "assertion failed"),
        ];

        let report = TestReport::new("MyTestSuite", results);

        assert_eq!(report.suite_name, "MyTestSuite");
        assert_eq!(report.summary.total, 3);
        assert_eq!(report.summary.passed, 2);
        assert_eq!(report.summary.failed, 1);
        assert!(!report.summary.all_passed());
    }

    #[test]
    fn test_markdown_generation() {
        let results = vec![
            TestResult::passed(TestMeta::new("test_example"), 100),
        ];
        let report = TestReport::new("ExampleSuite", results);
        let reporter = Reporter::markdown();
        let markdown = reporter.generate(&report);

        assert!(markdown.contains("# Test Report: ExampleSuite"));
        assert!(markdown.contains("✅ PASSED"));
        assert!(markdown.contains("test_example"));
    }

    #[test]
    fn test_json_generation() {
        let results = vec![
            TestResult::passed(TestMeta::new("test_json"), 50),
        ];
        let report = TestReport::new("JsonSuite", results);
        let reporter = Reporter::json();
        let json = reporter.generate(&report);

        assert!(json.contains("\"suite_name\": \"JsonSuite\""));
        assert!(json.contains("\"test_json\""));
    }

    #[test]
    fn test_junit_generation() {
        let results = vec![
            TestResult::passed(TestMeta::new("test_pass"), 100),
            TestResult::failed(TestMeta::new("test_fail"), 50, "oops"),
        ];
        let report = TestReport::new("JUnitSuite", results);
        let reporter = Reporter::junit();
        let xml = reporter.generate(&report);

        assert!(xml.contains("<testsuite"));
        assert!(xml.contains("tests=\"2\""));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("<failure"));
    }

    #[test]
    fn test_junit_enhanced_attributes() {
        // Test with file_path and line_number
        let mut meta1 = TestMeta::new("test_with_location");
        meta1.file_path = Some("src/lib.rs".to_string());
        meta1.line_number = Some(42);
        meta1.full_name = "my_module::test_with_location".to_string();

        let mut meta2 = TestMeta::new("test_error");
        meta2.file_path = Some("src/error.rs".to_string());
        meta2.line_number = Some(100);
        meta2.full_name = "my_module::test_error".to_string();

        let results = vec![
            TestResult::passed(meta1, 150),
            TestResult {
                meta: meta2,
                status: TestStatus::Error,
                duration_ms: 25,
                error: Some("Runtime error".to_string()),
                stack_trace: Some("stack trace".to_string()),
                profile_metrics: None,
                stress_metrics: None,
                started_at: chrono::Utc::now().to_rfc3339(),
            },
        ];

        let report = TestReport::new("EnhancedSuite", results);
        let reporter = Reporter::junit();
        let xml = reporter.generate(&report);

        // Verify timestamp attribute on testsuite
        assert!(xml.contains("timestamp=\""));

        // Verify file and line attributes on testcase
        assert!(xml.contains("file=\"src/lib.rs\""));
        assert!(xml.contains("line=\"42\""));
        assert!(xml.contains("file=\"src/error.rs\""));
        assert!(xml.contains("line=\"100\""));

        // Verify type attribute on error element
        assert!(xml.contains("<error type=\"Error\""));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1024), "1.00KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00GB");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<test>"), "&lt;test&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
