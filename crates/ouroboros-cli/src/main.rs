//! Ouroboros CLI - Unified command-line interface
//!
//! Usage:
//!   ob qc run [path]            Run tests
//!   ob qc run --bench           Run benchmarks
//!   ob qc run --security        Run security tests
//!   ob qc run --coverage        Run tests with coverage
//!   ob qc run --coverage --html Generate HTML coverage report
//!   ob qc collect [path]        Collect tests without running
//!   ob qc run -k <pattern>      Filter tests by pattern

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use pyo3::prelude::*;
use std::path::PathBuf;

use ouroboros_qc::{DiscoveryConfig, FileType, walk_files, CoverageInfo, FileCoverage, Reporter, ReportFormat, TestReport, TestSummary};

#[derive(Parser)]
#[command(name = "ob")]
#[command(about = "Ouroboros unified CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Quality Control - run tests, benchmarks, and security checks
    Qc {
        #[command(subcommand)]
        action: QcAction,
    },
    /// Argus - unified code analysis (LSP + Linting)
    Argus {
        #[command(subcommand)]
        action: ArgusAction,
    },
    /// Talos - Rust-based build tool for web applications
    Talos {
        #[command(subcommand)]
        action: TalosAction,
    },
    /// Lint - alias for argus check (deprecated, use 'ob argus')
    #[command(hide = true)]
    Lint {
        #[command(subcommand)]
        action: ArgusAction,
    },
}

#[derive(Subcommand)]
enum QcAction {
    /// Run tests
    Run {
        /// Path to test file or directory
        #[arg(default_value = "python/tests")]
        path: String,

        /// Run benchmarks instead of tests
        #[arg(long)]
        bench: bool,

        /// Run security tests
        #[arg(long)]
        security: bool,

        /// Collect code coverage
        #[arg(long)]
        coverage: bool,

        /// Output HTML report (use with --coverage)
        #[arg(long)]
        html: bool,

        /// Output file for coverage report
        #[arg(short, long)]
        output: Option<String>,

        /// Fail if coverage is below threshold (0-100)
        #[arg(long, value_name = "MIN")]
        cov_fail_under: Option<f64>,

        /// Output coverage as JSON (for CI tools like Codecov)
        #[arg(long)]
        cov_json: bool,

        /// CI mode: minimal output, exit codes for automation
        #[arg(long)]
        ci: bool,

        /// Filter tests by pattern (case-insensitive)
        #[arg(short = 'k', long)]
        pattern: Option<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Stop on first failure
        #[arg(long)]
        fail_fast: bool,
    },

    /// Collect tests without running
    Collect {
        /// Path to test file or directory
        #[arg(default_value = "python/tests")]
        path: String,

        /// Filter tests by pattern
        #[arg(short = 'k', long)]
        pattern: Option<String>,
    },

    /// Migrate pytest tests to TestSuite format
    Migrate {
        /// Path to test file or directory
        path: String,

        /// Create backup before migration (path_pytest_bak)
        #[arg(long)]
        backup: bool,

        /// Preview changes without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum TalosAction {
    /// Initialize a new project
    Init {
        /// Project name
        name: Option<String>,
    },

    /// Install dependencies from package.json
    Install {
        /// Specific packages to install
        packages: Vec<String>,
    },

    /// Add a new dependency
    Add {
        /// Package to add
        package: String,

        /// Add as dev dependency
        #[arg(short, long)]
        dev: bool,
    },

    /// Remove a dependency
    Remove {
        /// Package to remove
        package: String,
    },

    /// Update dependencies
    Update {
        /// Specific package to update
        package: Option<String>,
    },

    /// Start development server with HMR
    Dev {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Build for production
    Build {
        /// Watch mode
        #[arg(short, long)]
        watch: bool,

        /// Output directory
        #[arg(short, long, default_value = "dist")]
        output: String,
    },

    /// Type check TypeScript files
    Check,
}

#[derive(Subcommand)]
enum ArgusAction {
    /// Check files for issues (linting + analysis)
    Check {
        /// Paths to check (files or directories)
        #[arg(default_value = ".")]
        paths: Vec<String>,

        /// Languages to check (comma-separated: python,typescript,rust)
        #[arg(short, long)]
        lang: Option<String>,

        /// Output format (json, markdown, console)
        #[arg(short, long, default_value = "console")]
        format: String,

        /// Minimum severity to report (error, warning, info, hint)
        #[arg(long, default_value = "warning")]
        min_severity: String,

        /// Output to file instead of stdout
        #[arg(short, long)]
        output: Option<String>,
    },

    /// List available rules
    Rules {
        /// Language to list rules for
        #[arg(short, long)]
        lang: Option<String>,
    },

    /// Start LSP server for editor integration
    Serve {
        /// Port to listen on (stdio if not specified)
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Start daemon server for fast code analysis
    Server {
        /// Root directory to analyze (default: current directory)
        #[arg(default_value = ".")]
        root: String,

        /// Custom Unix socket path (default: /tmp/argus-<hash>.sock)
        #[arg(long)]
        socket: Option<String>,

        /// Disable file watching
        #[arg(long)]
        no_watch: bool,
    },

    /// Print MCP configuration for Claude Desktop
    Mcp,

    /// Start MCP server (stdio mode, for AI integration)
    McpServer,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Talos { action } => match action {
            TalosAction::Init { name } => {
                run_talos_init(name)?;
            }
            TalosAction::Install { packages } => {
                run_talos_install(packages)?;
            }
            TalosAction::Add { package, dev } => {
                run_talos_add(&package, dev)?;
            }
            TalosAction::Remove { package } => {
                run_talos_remove(&package)?;
            }
            TalosAction::Update { package } => {
                run_talos_update(package)?;
            }
            TalosAction::Dev { port, host } => {
                run_talos_dev(&host, port)?;
            }
            TalosAction::Build { watch, output } => {
                run_talos_build(&output, watch)?;
            }
            TalosAction::Check => {
                run_talos_check()?;
            }
        },
        Commands::Qc { action } => match action {
            QcAction::Run {
                path,
                bench,
                security,
                coverage,
                html,
                output,
                cov_fail_under,
                cov_json,
                ci,
                pattern,
                verbose,
                fail_fast,
            } => {
                let exit_code = run_tests(&path, bench, security, coverage, html, output, cov_fail_under, cov_json, ci, pattern, verbose, fail_fast)?;
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }

            QcAction::Collect { path, pattern } => {
                collect_tests(&path, pattern)?;
            }

            QcAction::Migrate { path, backup, dry_run, verbose } => {
                migrate_tests(&path, backup, dry_run, verbose)?;
            }
        },
        Commands::Argus { action } | Commands::Lint { action } => match action {
            ArgusAction::Check {
                paths,
                lang,
                format,
                min_severity,
                output,
            } => {
                let exit_code = run_argus_check(&paths, lang, &format, &min_severity, output)?;
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            ArgusAction::Rules { lang } => {
                list_argus_rules(lang)?;
            }
            ArgusAction::Serve { port } => {
                run_argus_lsp(port)?;
            }
            ArgusAction::Server { root, socket, no_watch } => {
                run_argus_server(&root, socket, no_watch)?;
            }
            ArgusAction::Mcp => {
                print_mcp_config();
            }
            ArgusAction::McpServer => {
                run_mcp_server()?;
            }
        },
    }

    Ok(())
}

/// Discover test files using Rust's fast walker
fn discover_test_files(path: &str, file_type: FileType) -> Result<Vec<ouroboros_qc::FileInfo>> {
    let config = DiscoveryConfig {
        root_path: PathBuf::from(path),
        patterns: match file_type {
            FileType::Test => vec!["test_*.py".to_string()],
            FileType::Benchmark => vec!["bench_*.py".to_string()],
        },
        exclusions: vec![
            "__pycache__".to_string(),
            ".git".to_string(),
            ".venv".to_string(),
            "node_modules".to_string(),
        ],
        max_depth: 10,
        num_threads: 4,
    };

    let files = walk_files(&config).map_err(|e| anyhow::anyhow!(e))?;
    let filtered: Vec<_> = files
        .into_iter()
        .filter(|f| f.file_type == file_type)
        .collect();

    Ok(filtered)
}

/// Run tests using embedded Python
/// Returns exit code: 0 = success, 1 = test failures, 2 = coverage below threshold
fn run_tests(
    path: &str,
    bench: bool,
    _security: bool,
    coverage: bool,
    html: bool,
    output: Option<String>,
    cov_fail_under: Option<f64>,
    cov_json: bool,
    ci: bool,
    pattern: Option<String>,
    verbose: bool,
    fail_fast: bool,
) -> Result<i32> {
    let file_type = if bench { FileType::Benchmark } else { FileType::Test };

    if !ci {
        println!("🔍 Discovering {} files in {}...",
            if bench { "benchmark" } else { "test" },
            path
        );
    }

    let files = discover_test_files(path, file_type)?;

    if files.is_empty() {
        if !ci {
            println!("❌ No {} files found", if bench { "benchmark" } else { "test" });
        }
        return Ok(1);
    }

    if !ci {
        println!("✅ Found {} file(s)", files.len());
        if coverage {
            println!("📊 Coverage collection enabled");
        }
    }

    // Initialize Python
    pyo3::prepare_freethreaded_python();

    let result = Python::with_gil(|py| -> Result<(u32, u32, u32, Option<CoverageInfo>)> {
        // Add directories to sys.path
        let sys = py.import("sys").context("Failed to import sys")?;
        let sys_path = sys.getattr("path").context("Failed to get sys.path")?;

        // Add current working directory
        sys_path.call_method1("insert", (0, ".")).context("Failed to add cwd to sys.path")?;

        // Add 'python' directory for the ouroboros module
        sys_path.call_method1("insert", (0, "python")).context("Failed to add python to sys.path")?;

        // Add the test directory's parent (for relative imports in tests)
        let test_dir = std::path::Path::new(path);
        if let Some(parent) = test_dir.parent() {
            let parent_str = parent.to_string_lossy();
            sys_path.call_method1("insert", (0, parent_str.as_ref())).ok();
        }
        // Add the test directory itself
        sys_path.call_method1("insert", (0, path)).ok();

        // Start coverage if enabled
        let cov_instance = if coverage {
            match start_coverage(py, path) {
                Ok(cov) => Some(cov),
                Err(e) => {
                    if !ci {
                        println!("⚠️  Failed to start coverage: {}", e);
                        println!("   Install coverage.py: pip install coverage");
                    }
                    None
                }
            }
        } else {
            None
        };

        let mut total_passed = 0u32;
        let mut total_failed = 0u32;
        let mut total_errors = 0u32;

        for file_info in &files {
            // Apply pattern filter
            if let Some(ref pat) = pattern {
                let pat_lower = pat.to_lowercase();
                if !file_info.module_name.to_lowercase().contains(&pat_lower) {
                    continue;
                }
            }

            if verbose && !ci {
                println!("\n📄 Loading: {}", file_info.module_name);
            }

            match run_test_file(py, &file_info.path, &file_info.module_name, verbose && !ci) {
                Ok((passed, failed, errors)) => {
                    total_passed += passed;
                    total_failed += failed;
                    total_errors += errors;

                    if fail_fast && (failed > 0 || errors > 0) {
                        if !ci {
                            println!("\n❌ Stopping due to --fail-fast");
                        }
                        break;
                    }
                }
                Err(e) => {
                    if !ci {
                        println!("❌ Error loading {}: {}", file_info.module_name, e);
                    }
                    total_errors += 1;
                    if fail_fast {
                        break;
                    }
                }
            }
        }

        // Stop coverage and collect data
        let coverage_info = if let Some(ref cov) = cov_instance {
            match stop_and_collect_coverage(py, cov, path) {
                Ok(info) => Some(info),
                Err(e) => {
                    if !ci {
                        println!("⚠️  Failed to collect coverage: {}", e);
                    }
                    None
                }
            }
        } else {
            None
        };

        Ok((total_passed, total_failed, total_errors, coverage_info))
    })?;

    let (total_passed, total_failed, total_errors, coverage_info) = result;

    // CI mode: JSON output
    if ci && cov_json {
        if let Some(ref cov_info) = coverage_info {
            let json_output = serde_json::json!({
                "tests": {
                    "passed": total_passed,
                    "failed": total_failed,
                    "errors": total_errors,
                    "total": total_passed + total_failed + total_errors
                },
                "coverage": {
                    "percent": cov_info.coverage_percent,
                    "covered": cov_info.covered_statements,
                    "total": cov_info.total_statements,
                    "files": cov_info.files.iter().map(|f| {
                        serde_json::json!({
                            "path": f.path,
                            "percent": f.coverage_percent,
                            "covered": f.covered,
                            "statements": f.statements,
                            "missing_lines": f.missing_lines
                        })
                    }).collect::<Vec<_>>()
                }
            });
            println!("{}", serde_json::to_string(&json_output).unwrap_or_default());
        } else {
            let json_output = serde_json::json!({
                "tests": {
                    "passed": total_passed,
                    "failed": total_failed,
                    "errors": total_errors,
                    "total": total_passed + total_failed + total_errors
                }
            });
            println!("{}", serde_json::to_string(&json_output).unwrap_or_default());
        }
    } else if ci {
        // CI mode: minimal line output
        if let Some(ref cov_info) = coverage_info {
            println!("PASSED={} FAILED={} ERRORS={} COVERAGE={:.1}%",
                total_passed, total_failed, total_errors, cov_info.coverage_percent);
        } else {
            println!("PASSED={} FAILED={} ERRORS={}",
                total_passed, total_failed, total_errors);
        }
    } else {
        // Normal mode: pretty output
        println!("\n{}", "=".repeat(60));
        println!("TEST SUMMARY");
        println!("{}", "=".repeat(60));
        println!("✅ Passed:  {}", total_passed);
        println!("❌ Failed:  {}", total_failed);
        println!("⚠️  Errors:  {}", total_errors);

        // Print coverage summary
        if let Some(ref cov_info) = coverage_info {
            println!();
            let cov_emoji = if cov_info.coverage_percent >= 80.0 {
                "🟢"
            } else if cov_info.coverage_percent >= 60.0 {
                "🟡"
            } else {
                "🔴"
            };
            println!("{} Coverage: {:.1}% ({}/{} statements)",
                cov_emoji,
                cov_info.coverage_percent,
                cov_info.covered_statements,
                cov_info.total_statements
            );
        }

        println!("{}", "=".repeat(60));
    }

    // Generate report if coverage was collected (non-CI or explicit output)
    if let Some(ref cov_info) = coverage_info {
        if !ci || output.is_some() {
            let report = TestReport::from_summary(
                "Test Results".to_string(),
                TestSummary {
                    total: (total_passed + total_failed + total_errors) as usize,
                    passed: total_passed as usize,
                    failed: total_failed as usize,
                    skipped: 0,
                    errors: total_errors as usize,
                    total_duration_ms: 0,
                },
            ).with_coverage(cov_info.clone());

            let format = if html { ReportFormat::Html } else { ReportFormat::Markdown };
            let output_path = output.clone().unwrap_or_else(|| {
                if html { "coverage_report.html".to_string() } else { "coverage_report.md".to_string() }
            });

            let reporter = Reporter::new(format);
            let report_content = reporter.generate(&report);

            std::fs::write(&output_path, &report_content)
                .context("Failed to write coverage report")?;

            if !ci {
                println!("\n📄 Coverage report written to: {}", output_path);
            }
        }
    }

    // Determine exit code
    let mut exit_code = 0;

    // Exit 1 if tests failed
    if total_failed > 0 || total_errors > 0 {
        exit_code = 1;
    }

    // Exit 2 if coverage below threshold
    if let Some(threshold) = cov_fail_under {
        if let Some(ref cov_info) = coverage_info {
            if cov_info.coverage_percent < threshold {
                if !ci {
                    println!("\n❌ Coverage {:.1}% is below threshold {:.1}%",
                        cov_info.coverage_percent, threshold);
                }
                exit_code = 2;
            }
        }
    }

    Ok(exit_code)
}

/// Start coverage.py collection
fn start_coverage<'py>(py: Python<'py>, source_path: &str) -> Result<Bound<'py, PyAny>> {
    let coverage_module = py.import("coverage")
        .context("coverage.py not installed")?;

    // Determine source directory (coverage needs directories, not files)
    let _source_dir = if std::path::Path::new(source_path).is_file() {
        std::path::Path::new(source_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string())
    } else {
        source_path.to_string()
    };

    // Create Coverage instance with source filter
    // Cover the ouroboros module which is in python/ouroboros
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("source", vec!["python/ouroboros"])?;
    kwargs.set_item("omit", vec!["*test_*", "*__pycache__*", "*/.venv/*", "*tests/*"])?;

    let cov = coverage_module.call_method("Coverage", (), Some(&kwargs))?;

    // Start collection
    cov.call_method0("start")?;

    // Store source_dir for later use (we'll use it in stop_and_collect)
    Ok(cov)
}

/// Stop coverage and collect data into Rust CoverageInfo
fn stop_and_collect_coverage(py: Python<'_>, cov: &Bound<'_, PyAny>, source_path: &str) -> Result<CoverageInfo> {
    // Stop and save
    cov.call_method0("stop")?;
    cov.call_method0("save")?;

    // Get coverage data
    let data = cov.call_method0("get_data")?;
    // measured_files() returns a set, need to convert to list first
    let measured_set = data.call_method0("measured_files")?;
    let builtins = py.import("builtins")?;
    let measured_list = builtins.call_method1("list", (&measured_set,))?;
    let measured_files: Vec<String> = measured_list.extract()?;

    let mut coverage_info = CoverageInfo::default();
    let mut files = Vec::new();

    for file_path in measured_files {
        // Skip test files and non-python files
        if file_path.contains("test_") || file_path.contains("__pycache__") {
            continue;
        }

        // Get analysis for this file
        let analysis_result = cov.call_method1("analysis", (&file_path,));
        if let Ok(analysis) = analysis_result {
            let tuple: (String, Vec<i32>, Vec<i32>, String) = analysis.extract()?;
            let (_filename, executable, missing, _excluded) = tuple;

            let total_statements = executable.len();
            let missing_count = missing.len();
            let covered = total_statements - missing_count;

            let coverage_percent = if total_statements > 0 {
                (covered as f64 / total_statements as f64) * 100.0
            } else {
                100.0
            };

            // Make path relative to source
            let relative_path = file_path
                .strip_prefix(source_path)
                .unwrap_or(&file_path)
                .trim_start_matches('/')
                .to_string();

            files.push(FileCoverage {
                path: relative_path,
                statements: total_statements,
                covered,
                missing_lines: missing.iter().map(|&x| x as usize).collect(),
                coverage_percent,
            });

            coverage_info.total_statements += total_statements;
            coverage_info.covered_statements += covered;
        }
    }

    // Calculate overall percentage
    coverage_info.coverage_percent = if coverage_info.total_statements > 0 {
        (coverage_info.covered_statements as f64 / coverage_info.total_statements as f64) * 100.0
    } else {
        100.0
    };

    coverage_info.files = files;

    Ok(coverage_info)
}

/// Run a single test file and return (passed, failed) counts
fn run_test_file(py: Python<'_>, file_path: &std::path::Path, module_name: &str, verbose: bool) -> Result<(u32, u32, u32)> {
    // Import module using file path
    let importlib_util = py.import("importlib.util")
        .context("Failed to import importlib.util")?;

    let file_path_str = file_path.to_string_lossy();

    // Create module spec from file location
    let spec = importlib_util
        .call_method1("spec_from_file_location", (module_name, file_path_str.as_ref()))?;

    if spec.is_none() {
        anyhow::bail!("Failed to create spec for module: {}", module_name);
    }

    // Create module from spec
    let module = importlib_util
        .call_method1("module_from_spec", (&spec,))?;

    // Execute the module
    let loader = spec.getattr("loader")?;
    loader.call_method1("exec_module", (&module,))?;

    // Import TestSuite base class for isinstance check
    let qc_module = py.import("ouroboros.qc")
        .context("Failed to import ouroboros.qc")?;
    let test_suite_class = qc_module.getattr("TestSuite")
        .context("Failed to get TestSuite class")?;

    // Find all TestSuite subclasses in the module
    let mut total_passed = 0u32;
    let mut total_failed = 0u32;
    let mut total_errors = 0u32;

    let dir_result = module.dir()?;
    for name in dir_result.iter() {
        let name_str: String = name.extract()?;

        // Skip private/dunder names
        if name_str.starts_with('_') {
            continue;
        }

        let attr = match module.getattr(&*name_str) {
            Ok(a) => a,
            Err(_) => continue,
        };

        // Check if it's a class that's a subclass of TestSuite
        let inspect = py.import("inspect")?;
        let is_class: bool = inspect
            .call_method1("isclass", (&attr,))?
            .extract()?;

        if !is_class {
            continue;
        }

        // Check if it's a subclass of TestSuite (but not TestSuite itself)
        let builtins = py.import("builtins")?;
        let is_subclass_result = builtins.call_method1("issubclass", (&attr, &test_suite_class));

        let is_subclass: bool = match is_subclass_result {
            Ok(result) => {
                let is_sub: bool = result.extract().unwrap_or(false);
                // Make sure it's not TestSuite itself
                let is_same: bool = attr.is(&test_suite_class);
                is_sub && !is_same
            }
            Err(_) => false,
        };

        if !is_subclass {
            continue;
        }

        if verbose {
            println!("  🧪 Running: {}", name_str);
        }

        // Create instance and run
        match run_test_suite(py, &attr, verbose) {
            Ok((p, f, e)) => {
                total_passed += p;
                total_failed += f;
                total_errors += e;
            }
            Err(e) => {
                println!("  ❌ Error running {}: {}", name_str, e);
                total_errors += 1;
            }
        }
    }

    Ok((total_passed, total_failed, total_errors))
}

/// Run a TestSuite class and return (passed, failed, errors) counts
fn run_test_suite(py: Python<'_>, suite_class: &Bound<'_, PyAny>, verbose: bool) -> Result<(u32, u32, u32)> {
    // Create instance
    let suite_instance = suite_class.call0()
        .context("Failed to instantiate test suite")?;

    // Get asyncio for running async code
    let asyncio = py.import("asyncio")?;

    // Build keyword arguments for run()
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("verbose", verbose)?;

    // Run the suite (it's async, so we need asyncio.run)
    let run_coro = suite_instance.call_method("run", (), Some(&kwargs))?;
    let report = asyncio.call_method1("run", (run_coro,))?;

    // Extract summary
    let summary = report.getattr("summary")?;
    let passed: u32 = summary.getattr("passed")?.extract()?;
    let failed: u32 = summary.getattr("failed")?.extract()?;
    let errors: u32 = summary.getattr("errors")?.extract()?;

    Ok((passed, failed, errors))
}

/// Collect tests without running them
fn collect_tests(path: &str, pattern: Option<String>) -> Result<()> {
    println!("🔍 Collecting tests in {}...", path);

    let files = discover_test_files(path, FileType::Test)?;

    if files.is_empty() {
        println!("❌ No test files found");
        return Ok(());
    }

    println!("\n📋 Test Files:");
    for file_info in &files {
        // Apply pattern filter
        if let Some(ref pat) = pattern {
            let pat_lower = pat.to_lowercase();
            if !file_info.module_name.to_lowercase().contains(&pat_lower) {
                continue;
            }
        }

        println!("  {} ({})", file_info.module_name, file_info.path.display());
    }

    println!("\n✅ Total: {} file(s)", files.len());
    Ok(())
}

/// Migrate pytest tests to TestSuite format
fn migrate_tests(path: &str, backup: bool, dry_run: bool, verbose: bool) -> Result<()> {
    println!("🔄 Migrating pytest tests to TestSuite format...");

    if dry_run {
        println!("   (dry-run mode - no files will be modified)");
    }

    // Initialize Python
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| -> Result<()> {
        // Add tools directory to sys.path
        let sys = py.import("sys")?;
        let sys_path = sys.getattr("path")?;
        sys_path.call_method1("insert", (0, "python/tools"))?;
        sys_path.call_method1("insert", (0, "python"))?;

        // Import the migration module
        let migrate_module = py.import("migrate_to_ouroboros_test")
            .context("Failed to import migrate_to_ouroboros_test module")?;

        // Get the migrate_directory function
        let migrate_fn = migrate_module.getattr("migrate_directory")?;

        // Convert path to Python Path object
        let pathlib = py.import("pathlib")?;
        let py_path = pathlib.call_method1("Path", (path,))?;

        // Build kwargs
        let kwargs = pyo3::types::PyDict::new(py);
        kwargs.set_item("backup", backup)?;
        kwargs.set_item("dry_run", dry_run)?;
        kwargs.set_item("verbose", verbose)?;

        // Call migrate_directory
        let stats = migrate_fn.call((py_path,), Some(&kwargs))?;

        // Extract and print stats
        let total: u32 = stats.getattr("total_files")?.extract()?;
        let migrated: u32 = stats.getattr("migrated")?.extract()?;
        let skipped: u32 = stats.getattr("skipped")?.extract()?;
        let failed: u32 = stats.getattr("failed")?.extract()?;
        let already: u32 = stats.getattr("already_testsuite")?.extract()?;

        println!("\n{}", "=".repeat(60));
        println!("MIGRATION SUMMARY{}", if dry_run { " (DRY RUN)" } else { "" });
        println!("{}", "=".repeat(60));
        println!("📁 Total files:       {}", total);
        println!("✅ Migrated:          {}", migrated);
        println!("⏭️  Already TestSuite: {}", already);
        println!("⏭️  Skipped:           {}", skipped);
        println!("❌ Failed:            {}", failed);
        println!("{}", "=".repeat(60));

        if failed > 0 {
            let errors: Vec<(String, String)> = stats.getattr("errors")?.extract()?;
            println!("\n❌ Errors:");
            for (path, error) in errors {
                println!("  {}: {}", path, error);
            }
        }

        Ok(())
    })?;

    Ok(())
}

// =============================================================================
// Argus Commands
// =============================================================================

/// Run Argus checks on files
fn run_argus_check(
    paths: &[String],
    lang: Option<String>,
    format: &str,
    _min_severity: &str,
    output: Option<String>,
) -> Result<i32> {
    use argus::{check_paths, Language, LintConfig, OutputFormat, Reporter};
    use argus::server::DaemonClient;

    // Try daemon first for faster results
    let cwd = std::env::current_dir().unwrap_or_default();
    let client = DaemonClient::for_workspace(&cwd);

    // Create tokio runtime for async client operations
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    // Check if daemon is running and try to use it
    let daemon_result = rt.block_on(async {
        if !client.is_daemon_running().await {
            return None;
        }

        // Use daemon for check
        for path in paths {
            match client.check(path).await {
                Ok(result) => {
                    // Print diagnostics from daemon
                    if let Some(check_result) = result.as_object() {
                        if let Some(diagnostics) = check_result.get("diagnostics").and_then(|d| d.as_array()) {
                            for diag in diagnostics {
                                if let Some(obj) = diag.as_object() {
                                    let file = obj.get("file").and_then(|f| f.as_str()).unwrap_or("");
                                    let line = obj.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
                                    let col = obj.get("column").and_then(|c| c.as_u64()).unwrap_or(0);
                                    let severity = obj.get("severity").and_then(|s| s.as_str()).unwrap_or("error");
                                    let code = obj.get("code").and_then(|c| c.as_str()).unwrap_or("");
                                    let message = obj.get("message").and_then(|m| m.as_str()).unwrap_or("");

                                    let emoji = match severity {
                                        "error" => "❌",
                                        "warning" => "⚠️",
                                        _ => "ℹ️",
                                    };

                                    eprintln!("{} {}:{}:{}: [{}] {}", emoji, file, line, col, code, message);
                                }
                            }
                        }

                        let errors = check_result.get("errors").and_then(|e| e.as_u64()).unwrap_or(0);
                        let warnings = check_result.get("warnings").and_then(|w| w.as_u64()).unwrap_or(0);

                        if errors > 0 {
                            return Some(1i32);
                        }
                        if warnings > 0 {
                            return Some(0);
                        }
                        return Some(0);
                    }
                }
                Err(_) => return None,
            }
        }
        Some(0)
    });

    // If daemon succeeded, return its result
    if let Some(exit_code) = daemon_result {
        return Ok(exit_code);
    }

    // Fallback to direct analysis
    // Parse languages
    let languages = if let Some(lang_str) = lang {
        lang_str
            .split(',')
            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                "python" | "py" => Some(Language::Python),
                "typescript" | "ts" => Some(Language::TypeScript),
                "rust" | "rs" => Some(Language::Rust),
                _ => None,
            })
            .collect()
    } else {
        vec![Language::Python, Language::TypeScript, Language::Rust]
    };

    // Create config
    let config = LintConfig {
        languages,
        ..LintConfig::default()
    };

    // Convert paths
    let path_refs: Vec<&std::path::Path> = paths
        .iter()
        .map(|p| std::path::Path::new(p))
        .collect();

    // Run checks
    let results = check_paths(&path_refs, &config);

    // Format output
    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Console);
    let reporter = Reporter::new(output_format);
    let report = reporter.generate(&results);

    // Output
    if let Some(output_path) = output {
        std::fs::write(&output_path, &report)
            .context("Failed to write lint report")?;
        println!("📄 Lint report written to: {}", output_path);
    } else {
        print!("{}", report);
    }

    // Count errors
    let error_count: usize = results.iter().map(|r| r.error_count()).sum();
    let warning_count: usize = results.iter().map(|r| r.warning_count()).sum();

    if error_count > 0 {
        return Ok(1);
    }
    if warning_count > 0 {
        return Ok(0); // Warnings don't fail
    }

    Ok(0)
}

/// Print MCP configuration for Claude Desktop
fn print_mcp_config() {
    argus::mcp::server::print_mcp_config();
}

/// Run MCP server (stdio mode)
fn run_mcp_server() -> Result<()> {
    use argus::mcp::McpServer;

    let cwd = std::env::current_dir()
        .context("Failed to get current directory")?;

    let server = McpServer::new(cwd);

    // Create tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    rt.block_on(async {
        server.run_async().await
            .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))
    })
}

/// Run Argus daemon server
fn run_argus_server(root: &str, socket: Option<String>, no_watch: bool) -> Result<()> {
    use argus::server::{ArgusDaemon, DaemonConfig};

    let root_path = std::path::PathBuf::from(root)
        .canonicalize()
        .context("Failed to resolve root path")?;

    let mut config = DaemonConfig::new(root_path.clone());

    if let Some(socket_path) = socket {
        config = config.with_socket(std::path::PathBuf::from(socket_path));
    }

    config = config.with_watch(!no_watch);

    eprintln!("🚀 Starting Argus daemon server");
    eprintln!("   Root: {}", root_path.display());
    eprintln!("   Socket: {:?}", config.socket_path);
    eprintln!("   Watch: {}", if no_watch { "disabled" } else { "enabled" });

    let daemon = ArgusDaemon::new(config)
        .map_err(|e| anyhow::anyhow!("Failed to create daemon: {}", e))?;

    // Create tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    rt.block_on(async {
        daemon.run().await
            .map_err(|e| anyhow::anyhow!("Daemon error: {}", e))
    })
}

/// Run Argus LSP server
fn run_argus_lsp(port: Option<u16>) -> Result<()> {
    use argus::lsp;

    // Create tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    rt.block_on(async {
        if let Some(p) = port {
            eprintln!("🔮 Argus LSP server listening on port {}", p);
            lsp::run_server_tcp(p).await
                .map_err(|e| anyhow::anyhow!("LSP server error: {}", e))?;
        } else {
            // stdio mode - no output to stderr (it would interfere with LSP)
            lsp::run_server().await;
        }
        Ok(())
    })
}

/// List available Argus rules
fn list_argus_rules(lang: Option<String>) -> Result<()> {
    use argus::{CheckerRegistry, Language};

    let registry = CheckerRegistry::new();

    let languages = if let Some(lang_str) = lang {
        match lang_str.to_lowercase().as_str() {
            "python" | "py" => vec![Language::Python],
            "typescript" | "ts" => vec![Language::TypeScript],
            "rust" | "rs" => vec![Language::Rust],
            _ => {
                println!("Unknown language: {}", lang_str);
                return Ok(());
            }
        }
    } else {
        vec![Language::Python, Language::TypeScript, Language::Rust]
    };

    for lang in languages {
        if let Some(checker) = registry.get(lang) {
            println!("\n{} Rules:", lang.as_str().to_uppercase());
            println!("{}", "-".repeat(40));
            for rule in checker.available_rules() {
                println!("  {}", rule);
            }
        }
    }

    Ok(())
}

// =============================================================================
// Talos Commands
// =============================================================================

/// Initialize a new Talos project
fn run_talos_init(name: Option<String>) -> Result<()> {
    let project_name = name.unwrap_or_else(|| "my-app".to_string());
    println!("🚀 Initializing Talos project: {}", project_name);
    println!("⚠️  This feature is under development");
    Ok(())
}

/// Install dependencies
fn run_talos_install(packages: Vec<String>) -> Result<()> {
    if packages.is_empty() {
        println!("📦 Installing dependencies from package.json...");
    } else {
        println!("📦 Installing: {}", packages.join(", "));
    }
    println!("⚠️  This feature is under development");
    Ok(())
}

/// Add a dependency
fn run_talos_add(package: &str, dev: bool) -> Result<()> {
    println!("➕ Adding package: {}", package);
    if dev {
        println!("   (as dev dependency)");
    }
    println!("⚠️  This feature is under development");
    Ok(())
}

/// Remove a dependency
fn run_talos_remove(package: &str) -> Result<()> {
    println!("➖ Removing package: {}", package);
    println!("⚠️  This feature is under development");
    Ok(())
}

/// Update dependencies
fn run_talos_update(package: Option<String>) -> Result<()> {
    if let Some(pkg) = package {
        println!("🔄 Updating package: {}", pkg);
    } else {
        println!("🔄 Updating all dependencies...");
    }
    println!("⚠️  This feature is under development");
    Ok(())
}

/// Start development server
fn run_talos_dev(host: &str, port: u16) -> Result<()> {
    println!("🚀 Starting Talos development server...");
    println!("   Local: http://{}:{}", host, port);
    println!("⚠️  This feature is under development");
    Ok(())
}

/// Build for production
fn run_talos_build(output: &str, watch: bool) -> Result<()> {
    println!("🔨 Building for production...");
    println!("   Output: {}", output);
    if watch {
        println!("   Watch mode enabled");
    }
    println!("⚠️  This feature is under development");
    Ok(())
}

/// Type check
fn run_talos_check() -> Result<()> {
    println!("🔍 Type checking TypeScript files...");
    println!("⚠️  This feature is under development");
    Ok(())
}
