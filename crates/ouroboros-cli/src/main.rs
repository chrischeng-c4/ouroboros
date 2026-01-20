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
//!   ob pg init                  Initialize migrations directory
//!   ob pg revision -m "msg"     Create a new migration
//!   ob pg upgrade               Apply pending migrations
//!   ob pg downgrade             Revert last migration
//!   ob pg status                Show migration status
//!
//! API Server (uvicorn-compatible CLI):
//!   ob api serve                          Start server (production mode)
//!   ob api serve --reload                 Start with hot reload (dev mode)
//!   ob api serve --reload --reload-dir ./src
//!   ob api serve --host 0.0.0.0 --port 3000
//!   ob api serve --log-level debug

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use pyo3::prelude::*;
use std::path::PathBuf;

use ouroboros_qc::{DiscoveryConfig, FileType, walk_files, CoverageInfo, FileCoverage, Reporter, ReportFormat, TestReport, TestSummary};
use ouroboros_postgres::{MigrationCli, MigrationCliConfig};

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
    /// Lint - alias for argus check (deprecated, use 'ob argus')
    #[command(hide = true)]
    Lint {
        #[command(subcommand)]
        action: ArgusAction,
    },
    /// PostgreSQL migrations (like Alembic)
    Pg {
        #[command(subcommand)]
        action: PgAction,
    },
    /// API server commands
    Api {
        #[command(subcommand)]
        action: ApiAction,
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
enum PgAction {
    /// Initialize migrations directory
    Init {
        /// Directory path for migrations
        #[arg(short, long, default_value = "./migrations")]
        directory: String,
    },

    /// Create a new migration
    Revision {
        /// Migration description
        #[arg(short, long)]
        message: String,

        /// Auto-generate migration from model changes
        #[arg(long)]
        autogenerate: bool,
    },

    /// Apply pending migrations
    #[command(alias = "up")]
    Upgrade {
        /// Number of migrations to apply (default: all)
        #[arg(short, long)]
        steps: Option<usize>,

        /// Show SQL without executing
        #[arg(long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Revert migrations
    #[command(alias = "down")]
    Downgrade {
        /// Number of migrations to revert (default: 1)
        #[arg(short = 'n', long, default_value = "1")]
        steps: usize,

        /// Show SQL without executing
        #[arg(long)]
        dry_run: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show migration status
    Status,

    /// Show migration history
    History,

    /// Show current migration version
    Current,

    /// Validate migration checksums
    Validate,
}

#[derive(Subcommand)]
enum ApiAction {
    /// Initialize a new API project
    ///
    /// Examples:
    ///   ob api init                    # Interactive mode
    ///   ob api init --minimal          # Minimal single-file app
    ///   ob api init --name my-api      # Specify project name
    Init {
        /// Project name (defaults to current directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Create minimal single-file app (non-interactive)
        #[arg(long)]
        minimal: bool,

        /// Create full project structure (non-interactive)
        #[arg(long)]
        full: bool,

        /// Target directory (defaults to current directory)
        #[arg(default_value = ".")]
        directory: String,
    },

    /// Start API server (supports both dev and production modes)
    ///
    /// Examples:
    ///   ob api serve                           # Production mode on :8000
    ///   ob api serve --reload                  # Dev mode with hot reload
    ///   ob api serve --reload --reload-dir ./src
    ///   ob api serve python.app:create_app    # Specify app factory
    Serve {
        /// Application import path (e.g., "python.app:create_app")
        #[arg(default_value = "")]
        app: String,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(long, default_value = "8000")]
        port: u16,

        /// Enable auto-reload (dev mode)
        #[arg(long)]
        reload: bool,

        /// Directories to watch for reload (implies --reload)
        #[arg(long, value_name = "PATH")]
        reload_dir: Vec<String>,

        /// Glob patterns to include for reload (e.g., "*.py")
        #[arg(long, value_name = "PATTERN", default_value = "*.py,*.rs")]
        reload_include: String,

        /// Glob patterns to exclude from reload
        #[arg(long, value_name = "PATTERN", default_value = "__pycache__,target,.git,node_modules")]
        reload_exclude: String,

        /// Delay between reload checks in seconds
        #[arg(long, value_name = "FLOAT", default_value = "0.5")]
        reload_delay: f64,

        /// Log level (trace, debug, info, warn, error)
        #[arg(long, default_value = "info")]
        log_level: String,

        /// Access log on/off
        #[arg(long)]
        access_log: bool,
    },
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
        Commands::Pg { action } => {
            run_pg_command(action)?;
        },
        Commands::Api { action } => match action {
            ApiAction::Init {
                name,
                minimal,
                full,
                directory,
            } => {
                run_api_init(name, minimal, full, &directory)?;
            }
            ApiAction::Serve {
                app,
                host,
                port,
                reload,
                reload_dir,
                reload_include,
                reload_exclude,
                reload_delay,
                log_level,
                access_log,
            } => {
                run_api_server(
                    app,
                    host,
                    port,
                    reload,
                    reload_dir,
                    reload_include,
                    reload_exclude,
                    reload_delay,
                    log_level,
                    access_log,
                )?;
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
// PostgreSQL Migration Commands
// =============================================================================

/// Run PostgreSQL migration commands
fn run_pg_command(action: PgAction) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    match action {
        PgAction::Init { directory } => {
            run_pg_init(&directory)?;
        }
        PgAction::Revision { message, autogenerate } => {
            if autogenerate {
                rt.block_on(run_pg_revision_auto(&message))?;
            } else {
                run_pg_revision(&message)?;
            }
        }
        PgAction::Upgrade { steps, dry_run, verbose } => {
            rt.block_on(run_pg_upgrade(steps, dry_run, verbose))?;
        }
        PgAction::Downgrade { steps, dry_run, verbose } => {
            rt.block_on(run_pg_downgrade(steps, dry_run, verbose))?;
        }
        PgAction::Status => {
            rt.block_on(run_pg_status())?;
        }
        PgAction::History => {
            rt.block_on(run_pg_history())?;
        }
        PgAction::Current => {
            rt.block_on(run_pg_current())?;
        }
        PgAction::Validate => {
            rt.block_on(run_pg_validate())?;
        }
    }

    Ok(())
}

/// Initialize migrations directory
fn run_pg_init(directory: &str) -> Result<()> {
    use std::fs;
    use std::path::Path;

    let migrations_dir = Path::new(directory);

    if migrations_dir.exists() {
        println!("Migrations directory already exists: {}", directory);
        return Ok(());
    }

    // Create migrations directory
    fs::create_dir_all(migrations_dir)
        .context("Failed to create migrations directory")?;

    // Create .env.example file
    let env_example = migrations_dir.join(".env.example");
    fs::write(&env_example, r#"# Database connection string
DATABASE_URL=postgresql://user:password@localhost:5432/dbname

# Migrations directory (optional, defaults to ./migrations)
# MIGRATIONS_DIR=./migrations

# Migrations table name (optional, defaults to _migrations)
# MIGRATIONS_TABLE=_migrations
"#).context("Failed to create .env.example")?;

    // Create README
    let readme = migrations_dir.join("README.md");
    fs::write(&readme, r#"# Database Migrations

This directory contains database migrations managed by `ob pg`.

## Commands

```bash
# Create a new migration
ob pg revision -m "add users table"

# Auto-generate migration from model changes
ob pg revision -m "sync models" --autogenerate

# Apply pending migrations
ob pg upgrade

# Apply specific number of migrations
ob pg upgrade --steps 1

# Revert last migration
ob pg downgrade

# Revert multiple migrations
ob pg downgrade -n 2

# Show status
ob pg status

# Show history
ob pg history

# Validate checksums
ob pg validate
```

## Migration File Format

Migration files use the following format:

```sql
-- Description: Create users table

-- UP
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL
);

-- DOWN
DROP TABLE users;
```
"#).context("Failed to create README.md")?;

    println!("Initialized migrations directory: {}", directory);
    println!("  Created: {}", env_example.display());
    println!("  Created: {}", readme.display());
    println!("\nNext steps:");
    println!("  1. Set DATABASE_URL environment variable");
    println!("  2. Create a migration: ob pg revision -m \"initial\"");

    Ok(())
}

/// Create a new empty migration
fn run_pg_revision(message: &str) -> Result<()> {
    use chrono::Utc;
    use std::fs;

    let migrations_dir = get_migrations_dir()?;

    // Generate version from current timestamp
    let version = Utc::now().format("%Y%m%d_%H%M%S").to_string();

    // Sanitize message for filename
    let sanitized_message = message
        .to_lowercase()
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    let filename = format!("{}_{}.sql", version, sanitized_message);
    let filepath = migrations_dir.join(&filename);

    let content = format!(r#"-- Description: {}

-- UP
-- TODO: Add your upgrade SQL here


-- DOWN
-- TODO: Add your downgrade SQL here

"#, message);

    fs::write(&filepath, content)
        .context("Failed to write migration file")?;

    println!("Created migration: {}", filepath.display());
    println!("  Version: {}", version);
    println!("  Description: {}", message);

    Ok(())
}

/// Create auto-generated migration from model changes
async fn run_pg_revision_auto(message: &str) -> Result<()> {
    use chrono::Utc;
    use std::fs;
    use ouroboros_postgres::{Connection, PoolConfig, AutoDetector};

    let migrations_dir = get_migrations_dir()?;
    let database_url = get_database_url()?;

    println!("Connecting to database...");
    let conn = Connection::new(&database_url, PoolConfig::default()).await
        .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;

    println!("Detecting schema changes...");
    let detector = AutoDetector::new(conn);

    // For now, detect changes against empty models (show current schema as "to be created")
    // In a full implementation, this would load models from Python files
    let result = detector.detect(&[]).await
        .map_err(|e| anyhow::anyhow!("Failed to detect changes: {}", e))?;

    if !result.has_changes() {
        println!("No changes detected.");
        return Ok(());
    }

    // Generate version from current timestamp
    let version = Utc::now().format("%Y%m%d_%H%M%S").to_string();

    // Sanitize message for filename
    let sanitized_message = message
        .to_lowercase()
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    let filename = format!("{}_{}.sql", version, sanitized_message);
    let filepath = migrations_dir.join(&filename);

    let content = format!(r#"-- Description: {}
-- Auto-generated migration

-- UP
{}

-- DOWN
{}
"#, message, result.up_sql, result.down_sql);

    fs::write(&filepath, content)
        .context("Failed to write migration file")?;

    println!("Created auto-generated migration: {}", filepath.display());
    println!("  Version: {}", version);
    println!("  Changes detected:");
    for summary in &result.summary {
        println!("    - {}", summary);
    }

    Ok(())
}

/// Apply pending migrations
async fn run_pg_upgrade(steps: Option<usize>, dry_run: bool, verbose: bool) -> Result<()> {
    let config = get_cli_config()?
        .dry_run(dry_run)
        .verbose(verbose);

    let cli = MigrationCli::new(config);
    let result = cli.up(steps).await
        .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;

    for msg in &result.messages {
        println!("{}", msg);
    }

    if !result.success {
        std::process::exit(1);
    }

    Ok(())
}

/// Revert migrations
async fn run_pg_downgrade(steps: usize, dry_run: bool, verbose: bool) -> Result<()> {
    let config = get_cli_config()?
        .dry_run(dry_run)
        .verbose(verbose);

    let cli = MigrationCli::new(config);
    let result = cli.down(Some(steps)).await
        .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;

    for msg in &result.messages {
        println!("{}", msg);
    }

    if !result.success {
        std::process::exit(1);
    }

    Ok(())
}

/// Show migration status
async fn run_pg_status() -> Result<()> {
    let config = get_cli_config()?;
    let cli = MigrationCli::new(config);
    let result = cli.status().await
        .map_err(|e| anyhow::anyhow!("Failed to get status: {}", e))?;

    for msg in &result.messages {
        println!("{}", msg);
    }

    Ok(())
}

/// Show migration history
async fn run_pg_history() -> Result<()> {
    let config = get_cli_config()?;
    let cli = MigrationCli::new(config);
    let result = cli.history().await
        .map_err(|e| anyhow::anyhow!("Failed to get history: {}", e))?;

    for msg in &result.messages {
        println!("{}", msg);
    }

    Ok(())
}

/// Show current migration version
async fn run_pg_current() -> Result<()> {
    let config = get_cli_config()?;
    let cli = MigrationCli::new(config);
    let result = cli.current().await
        .map_err(|e| anyhow::anyhow!("Failed to get current version: {}", e))?;

    for msg in &result.messages {
        println!("{}", msg);
    }

    Ok(())
}

/// Validate migration checksums
async fn run_pg_validate() -> Result<()> {
    let config = get_cli_config()?;
    let cli = MigrationCli::new(config);
    let result = cli.validate().await
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    for msg in &result.messages {
        println!("{}", msg);
    }

    if !result.success {
        std::process::exit(1);
    }

    Ok(())
}

/// Get migrations directory from environment or default
fn get_migrations_dir() -> Result<PathBuf> {
    let dir = std::env::var("MIGRATIONS_DIR")
        .unwrap_or_else(|_| "./migrations".to_string());
    let path = PathBuf::from(&dir);

    if !path.exists() {
        anyhow::bail!(
            "Migrations directory not found: {}\n\
             Run 'ob pg init' to create it, or set MIGRATIONS_DIR environment variable.",
            dir
        );
    }

    Ok(path)
}

/// Get database URL from environment
fn get_database_url() -> Result<String> {
    std::env::var("DATABASE_URL").context(
        "DATABASE_URL environment variable not set.\n\
         Set it to your PostgreSQL connection string, e.g.:\n\
         export DATABASE_URL=postgresql://user:password@localhost:5432/dbname"
    )
}

/// Get CLI config from environment
fn get_cli_config() -> Result<MigrationCliConfig> {
    MigrationCliConfig::from_env()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// =============================================================================
// API Init
// =============================================================================

/// Project template type
#[derive(Debug, Clone, Copy, PartialEq)]
enum ProjectTemplate {
    Minimal,
    Basic,
    Full,
}

impl std::fmt::Display for ProjectTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectTemplate::Minimal => write!(f, "Minimal (single app.py)"),
            ProjectTemplate::Basic => write!(f, "Basic (app + routes + config)"),
            ProjectTemplate::Full => write!(f, "Full (with tests, models, middleware)"),
        }
    }
}

/// Initialize a new API project
fn run_api_init(
    name: Option<String>,
    minimal: bool,
    full: bool,
    directory: &str,
) -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Input, Select, Confirm};
    use std::path::Path;

    let theme = ColorfulTheme::default();
    let target_dir = Path::new(directory);

    // Determine project name
    let project_name = if let Some(n) = name {
        n
    } else if directory != "." {
        directory.to_string()
    } else {
        // Interactive: ask for project name or use current directory name
        let default_name = std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-api".to_string());

        Input::with_theme(&theme)
            .with_prompt("Project name")
            .default(default_name)
            .interact_text()
            .context("Failed to read project name")?
    };

    // Determine template type
    let template = if minimal {
        ProjectTemplate::Minimal
    } else if full {
        ProjectTemplate::Full
    } else {
        // Interactive: ask for template type
        let templates = vec![
            ProjectTemplate::Minimal,
            ProjectTemplate::Basic,
            ProjectTemplate::Full,
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("Select project template")
            .items(&templates)
            .default(1) // Default to Basic
            .interact()
            .context("Failed to select template")?;

        templates[selection]
    };

    // Ask for OpenAPI option in interactive mode (only for Basic template)
    let include_openapi = if !minimal && !full && template == ProjectTemplate::Basic {
        Confirm::with_theme(&theme)
            .with_prompt("Include OpenAPI documentation?")
            .default(true)
            .interact()
            .context("Failed to read OpenAPI option")?
    } else {
        template == ProjectTemplate::Full || template == ProjectTemplate::Basic
    };

    println!("\n Creating {} project: {}", template, project_name);

    // Create directory structure based on template
    // Note: K8s probes (/health/*) and metrics (/metrics) are always included for Basic/Full
    match template {
        ProjectTemplate::Minimal => {
            create_minimal_project(target_dir, &project_name)?;
        }
        ProjectTemplate::Basic => {
            create_basic_project(target_dir, &project_name, include_openapi)?;
        }
        ProjectTemplate::Full => {
            create_full_project(target_dir, &project_name)?;
        }
    }

    println!("\n Project created successfully!");
    println!("\nNext steps:");
    println!("  cd {}", if directory == "." { &project_name } else { directory });
    println!("  ob api serve app:app --reload");

    Ok(())
}

/// Create minimal single-file project
fn create_minimal_project(target_dir: &std::path::Path, project_name: &str) -> Result<()> {
    use std::fs;

    let app_content = format!(r#""""
{project_name} - Minimal API
"""
from ouroboros.api import App, Response

app = App()


@app.get("/")
async def root():
    return {{"message": "Hello from {project_name}!"}}


@app.get("/health")
async def health():
    return {{"status": "ok"}}
"#, project_name = project_name);

    fs::write(target_dir.join("app.py"), app_content)
        .context("Failed to write app.py")?;

    println!("   Created app.py");

    Ok(())
}

/// Create basic project with routes and config
/// K8s probes and metrics are always included
fn create_basic_project(
    target_dir: &std::path::Path,
    project_name: &str,
    include_openapi: bool,
) -> Result<()> {
    use std::fs;

    // Create directories
    let routes_dir = target_dir.join("routes");
    fs::create_dir_all(&routes_dir).context("Failed to create routes directory")?;

    // Build app.py content dynamically
    let mut app_lines = vec![
        format!(r#"""""#),
        format!("{} - API Application", project_name),
        format!(r#"""""#),
        "from ouroboros.api import App".to_string(),
        String::new(),
        "from routes.api import router as api_router".to_string(),
        "from routes.probes import router as probes_router".to_string(),
        "from routes.metrics import router as metrics_router".to_string(),
    ];

    app_lines.push(String::new());
    app_lines.push("app = App()".to_string());

    if include_openapi {
        app_lines.push(String::new());
        app_lines.push("# OpenAPI documentation".to_string());
        app_lines.push(format!(r#"app.title = "{}""#, project_name));
        app_lines.push(r#"app.description = "API powered by Ouroboros""#.to_string());
        app_lines.push(r#"app.version = "0.1.0""#.to_string());
    }

    app_lines.push(String::new());
    app_lines.push("# Include routers".to_string());
    app_lines.push(r#"app.include_router(api_router, prefix="/api")"#.to_string());
    app_lines.push(String::new());
    app_lines.push("# K8s probes and metrics (Pure Rust - no GIL)".to_string());
    app_lines.push(r#"app.include_router(probes_router)"#.to_string());
    app_lines.push(r#"app.include_router(metrics_router)"#.to_string());

    app_lines.push(String::new());
    app_lines.push(String::new());
    app_lines.push(r#"@app.get("/")"#.to_string());
    app_lines.push("async def root():".to_string());
    app_lines.push(format!(r#"    return {{"message": "Welcome to {}", "docs": "/docs"}}"#, project_name));
    app_lines.push(String::new());

    let app_content = app_lines.join("\n");

    fs::write(target_dir.join("app.py"), app_content)
        .context("Failed to write app.py")?;
    println!("   Created app.py");

    // config.py
    let config_lines = vec![
        r#"""""#.to_string(),
        format!("Configuration for {}", project_name),
        r#"""""#.to_string(),
        "import os".to_string(),
        String::new(),
        String::new(),
        "class Config:".to_string(),
        r#"    """Application configuration.""""#.to_string(),
        String::new(),
        "    # Server settings".to_string(),
        r#"    HOST: str = os.getenv("HOST", "127.0.0.1")"#.to_string(),
        r#"    PORT: int = int(os.getenv("PORT", "8000"))"#.to_string(),
        r#"    DEBUG: bool = os.getenv("DEBUG", "false").lower() == "true""#.to_string(),
        String::new(),
        "    # API settings".to_string(),
        r#"    API_PREFIX: str = "/api""#.to_string(),
        r#"    API_VERSION: str = "v1""#.to_string(),
        String::new(),
        String::new(),
        "config = Config()".to_string(),
        String::new(),
    ];

    let config_content = config_lines.join("\n");
    fs::write(target_dir.join("config.py"), config_content)
        .context("Failed to write config.py")?;
    println!("   Created config.py");

    // routes/__init__.py
    fs::write(routes_dir.join("__init__.py"), "")
        .context("Failed to write routes/__init__.py")?;

    // routes/api.py
    let api_routes_content = r#""""
API routes
"""
from ouroboros.api import Router, Response

router = Router()


@router.get("/hello")
async def hello():
    return {"message": "Hello, World!"}


@router.get("/items/{item_id}")
async def get_item(item_id: int):
    return {"item_id": item_id, "name": f"Item {item_id}"}


@router.post("/items")
async def create_item(name: str, price: float):
    return {"name": name, "price": price, "status": "created"}
"#;

    fs::write(routes_dir.join("api.py"), api_routes_content)
        .context("Failed to write routes/api.py")?;
    println!("   Created routes/api.py");

    // routes/probes.py - K8s probes (Pure Rust capable)
    let probes_content = r##""""
Kubernetes probes (Pure Rust execution - no GIL overhead)

These endpoints can be implemented as pure Rust handlers for maximum performance:
- /health    - Basic health check
- /ready     - Readiness probe (can check DB, cache, etc.)
- /live      - Liveness probe (always returns ok if process is alive)
- /startup   - Startup probe (for slow-starting containers)

Usage in K8s deployment:
  livenessProbe:
    httpGet:
      path: /live
      port: 8000
    initialDelaySeconds: 3
    periodSeconds: 10
  readinessProbe:
    httpGet:
      path: /ready
      port: 8000
    initialDelaySeconds: 5
    periodSeconds: 5
  startupProbe:
    httpGet:
      path: /startup
      port: 8000
    failureThreshold: 30
    periodSeconds: 10
"""
from ouroboros.api import Router

router = Router()


@router.get("/health")
async def health():
    """Basic health check endpoint."""
    return {"status": "ok"}


@router.get("/ready")
async def readiness():
    """Readiness probe for Kubernetes.

    Add your dependency checks here:
    - Database connection
    - Cache connection
    - External service availability
    """
    return {"status": "ready"}


@router.get("/live")
async def liveness():
    """Liveness probe for Kubernetes.

    Should only check if the process itself is healthy.
    Do NOT check external dependencies here.
    """
    return {"status": "alive"}


@router.get("/startup")
async def startup():
    """Startup probe for Kubernetes.

    Used for containers that need time to start.
    Returns ok once the application is fully initialized.
    """
    return {"status": "started"}
"##;

    fs::write(routes_dir.join("probes.py"), probes_content)
        .context("Failed to write routes/probes.py")?;
    println!("   Created routes/probes.py");

    // routes/metrics.py - Prometheus metrics (Pure Rust capable)
    let metrics_content = r##""""
Prometheus metrics endpoint (Pure Rust execution - no GIL overhead)

Exposes metrics in Prometheus format at /metrics

Usage in K8s:
  Add this annotation to your pod:
    prometheus.io/scrape: "true"
    prometheus.io/port: "8000"
    prometheus.io/path: "/metrics"
"""
from ouroboros.api import Router, Response
import time

router = Router()

# Simple in-memory metrics (replace with proper metrics library in production)
_start_time = time.time()
_request_count = 0


@router.get("/metrics")
async def metrics():
    """Prometheus metrics endpoint.

    Returns metrics in Prometheus exposition format.
    Consider using prometheus_client library for production.
    """
    uptime = time.time() - _start_time

    # Prometheus exposition format
    lines = [
        "# HELP app_uptime_seconds Application uptime in seconds",
        "# TYPE app_uptime_seconds gauge",
        f"app_uptime_seconds {uptime:.2f}",
        "",
        "# HELP app_info Application information",
        "# TYPE app_info gauge",
        'app_info{version="0.1.0"} 1',
        "",
    ]

    return Response(
        body="\n".join(lines),
        headers={"Content-Type": "text/plain; version=0.0.4; charset=utf-8"}
    )
"##;

    fs::write(routes_dir.join("metrics.py"), metrics_content)
        .context("Failed to write routes/metrics.py")?;
    println!("   Created routes/metrics.py");

    Ok(())
}

/// Create full project with tests, models, and middleware
fn create_full_project(target_dir: &std::path::Path, project_name: &str) -> Result<()> {
    use std::fs;

    // First create basic structure (with OpenAPI enabled)
    create_basic_project(target_dir, project_name, true)?;

    // Create additional directories
    let dirs = ["models", "middleware", "tests", "utils"];
    for dir in dirs {
        fs::create_dir_all(target_dir.join(dir))
            .with_context(|| format!("Failed to create {} directory", dir))?;
        fs::write(target_dir.join(dir).join("__init__.py"), "")
            .with_context(|| format!("Failed to create {}/__init__.py", dir))?;
    }
    println!("   Created models/, middleware/, tests/, utils/");

    // models/base.py
    let models_content = r#""""
Base model definitions
"""
from dataclasses import dataclass
from typing import Optional
from datetime import datetime


@dataclass
class BaseModel:
    """Base model with common fields."""
    id: Optional[int] = None
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None


@dataclass
class User(BaseModel):
    """User model."""
    name: str = ""
    email: str = ""
    is_active: bool = True
"#;

    fs::write(target_dir.join("models").join("base.py"), models_content)
        .context("Failed to write models/base.py")?;
    println!("   Created models/base.py");

    // middleware/logging.py
    let middleware_content = r#""""
Logging middleware
"""
import time
from ouroboros.api import Request, Response


async def logging_middleware(request: Request, call_next):
    """Log request timing and details."""
    start_time = time.perf_counter()

    response = await call_next(request)

    process_time = (time.perf_counter() - start_time) * 1000
    print(f"{request.method} {request.path} - {response.status_code} ({process_time:.2f}ms)")

    return response
"#;

    fs::write(target_dir.join("middleware").join("logging.py"), middleware_content)
        .context("Failed to write middleware/logging.py")?;
    println!("   Created middleware/logging.py");

    // tests/test_api.py
    let test_content = format!(r#""""
API tests for {project_name}
"""
from ouroboros.qc import TestSuite, test


class TestAPI(TestSuite):
    """Test API endpoints."""

    @test
    async def test_root_endpoint(self):
        """Test root endpoint returns welcome message."""
        # TODO: Add actual HTTP client test
        assert True

    @test
    async def test_health_endpoint(self):
        """Test health endpoint returns ok status."""
        # TODO: Add actual HTTP client test
        assert True

    @test
    async def test_hello_endpoint(self):
        """Test hello endpoint."""
        # TODO: Add actual HTTP client test
        assert True
"#, project_name = project_name);

    fs::write(target_dir.join("tests").join("test_api.py"), test_content)
        .context("Failed to write tests/test_api.py")?;
    println!("   Created tests/test_api.py");

    // utils/helpers.py
    let utils_content = r#""""
Utility functions
"""
from typing import Any, Dict


def success_response(data: Any, message: str = "Success") -> Dict:
    """Create a standardized success response."""
    return {
        "success": True,
        "message": message,
        "data": data,
    }


def error_response(message: str, code: str = "ERROR") -> Dict:
    """Create a standardized error response."""
    return {
        "success": False,
        "error": {
            "code": code,
            "message": message,
        },
    }
"#;

    fs::write(target_dir.join("utils").join("helpers.py"), utils_content)
        .context("Failed to write utils/helpers.py")?;
    println!("   Created utils/helpers.py");

    // .env.example
    let env_content = format!(r#"# {project_name} Configuration

# Server
HOST=127.0.0.1
PORT=8000
DEBUG=true

# Database (optional)
# DATABASE_URL=postgresql://user:password@localhost:5432/dbname

# Logging
LOG_LEVEL=info
"#, project_name = project_name);

    fs::write(target_dir.join(".env.example"), env_content)
        .context("Failed to write .env.example")?;
    println!("   Created .env.example");

    Ok(())
}

// =============================================================================
// API Server
// =============================================================================

/// Parse app path in format "module:attribute" (e.g., "python.app:create_app")
fn parse_app_path(app_path: &str) -> Result<(String, String)> {
    if app_path.is_empty() {
        anyhow::bail!("App path is required (e.g., 'python.app:app' or 'myapp:create_app')");
    }

    let parts: Vec<&str> = app_path.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid app path format: '{}'\n\
             Expected format: 'module:attribute' (e.g., 'python.app:app')",
            app_path
        );
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Load Python app and extract routes
fn load_python_app(module_path: &str, attr_name: &str) -> Result<Vec<PythonRouteInfo>> {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        // Add current directory and python directory to sys.path
        let sys = py.import("sys").context("Failed to import sys")?;
        let sys_path = sys.getattr("path").context("Failed to get sys.path")?;
        sys_path.call_method1("insert", (0, ".")).ok();
        sys_path.call_method1("insert", (0, "python")).ok();

        // Import the module
        let module = py.import(module_path.as_ref() as &str)
            .with_context(|| format!("Failed to import module: {}", module_path))?;

        // Get the attribute (app or factory function)
        let attr = module.getattr(attr_name.as_ref() as &str)
            .with_context(|| format!("Module '{}' has no attribute '{}'", module_path, attr_name))?;

        // Check if it's callable (factory function) or an App instance
        let app = if attr.is_callable() {
            // Check if it's a class/type (App class) vs a factory function
            let inspect = py.import("inspect")?;
            let is_class: bool = inspect.call_method1("isclass", (&attr,))?.extract()?;

            if is_class {
                // It's the App class itself, not a factory - this is the app instance
                attr
            } else {
                // It's a factory function, call it to get the app
                attr.call0()
                    .with_context(|| format!("Failed to call factory function: {}", attr_name))?
            }
        } else {
            // It's already an instance
            attr
        };

        // Extract routes from the app
        extract_routes_from_app(py, &app)
    })
}

/// Route information extracted from Python app
#[derive(Debug)]
struct PythonRouteInfo {
    method: String,
    path: String,
    handler_name: String,
    handler: pyo3::PyObject,
}

/// Extract routes from a Python App instance
fn extract_routes_from_app(_py: Python<'_>, app: &pyo3::Bound<'_, pyo3::PyAny>) -> Result<Vec<PythonRouteInfo>> {
    // Check if app has _routes attribute (ouroboros.api.App)
    let routes_attr = app.getattr("_routes")
        .or_else(|_| app.getattr("routes"))
        .context("App object has no 'routes' or '_routes' attribute. Is this an ouroboros.api.App?")?;

    let routes_list: Vec<pyo3::Bound<'_, pyo3::PyAny>> = routes_attr.extract()
        .context("Failed to extract routes list")?;

    let mut routes = Vec::new();

    for route in routes_list {
        // Extract route info (RouteInfo dataclass)
        let method: String = route.getattr("method")?.extract()?;
        let path: String = route.getattr("path")?.extract()?;
        let name: String = route.getattr("name")?.extract()?;
        let handler = route.getattr("handler")?.into();

        routes.push(PythonRouteInfo {
            method,
            path,
            handler_name: name,
            handler,
        });
    }

    Ok(routes)
}

/// Run API server (supports both dev and production modes)
#[allow(clippy::too_many_arguments)]
fn run_api_server(
    app_path: String,
    host: String,
    port: u16,
    reload: bool,
    reload_dir: Vec<String>,
    reload_include: String,
    reload_exclude: String,
    reload_delay: f64,
    log_level: String,
    access_log: bool,
) -> Result<()> {
    use ouroboros_api::{Router, Server, ServerConfig};
    use ouroboros_api::handler::HandlerMeta;
    use ouroboros_api::validation::RequestValidator;
    use ouroboros_api::python_handler::PythonHandler;
    use ouroboros_pyloop::PyLoop;
    use std::sync::Arc;

    // Initialize logging
    init_logging(&log_level)?;

    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    let bind_addr = format!("{}:{}", host, port);

    // If --reload-dir is specified, enable reload mode automatically
    let reload_enabled = reload || !reload_dir.is_empty();

    // Load Python app if specified
    let python_routes = if !app_path.is_empty() {
        let (module_path, attr_name) = parse_app_path(&app_path)?;
        println!("Loading Python app: {}:{}", module_path, attr_name);
        let routes = load_python_app(&module_path, &attr_name)?;
        println!("  Found {} routes", routes.len());
        for route in &routes {
            println!("    {} {} -> {}", route.method, route.path, route.handler_name);
        }
        Some(routes)
    } else {
        None
    };

    if reload_enabled {
        // Dev mode with hot reload
        #[cfg(feature = "dev")]
        {
            use ouroboros_api::dev_server::{DevServer, DevServerConfig};

            // Parse include patterns (e.g., "*.py,*.rs" -> ["py", "rs"])
            let watch_extensions: Vec<String> = reload_include
                .split(',')
                .map(|s| s.trim().trim_start_matches("*.").to_string())
                .filter(|s| !s.is_empty())
                .collect();

            // Parse exclude patterns
            let exclude_patterns: Vec<String> = reload_exclude
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            // Use reload_dir if specified, otherwise default to current directory
            let watch_paths: Vec<PathBuf> = if reload_dir.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                reload_dir.iter().map(PathBuf::from).collect()
            };

            let dev_config = DevServerConfig {
                bind_addr: bind_addr.clone(),
                watch_dirs: watch_paths.clone(),
                watch_extensions: watch_extensions.clone(),
                debounce: std::time::Duration::from_secs_f64(reload_delay),
                exclude_patterns,
                hot_reload: true,
            };

            println!("Starting API server (dev mode)...");
            println!("  Uvicorn-compatible CLI");
            println!("  Address: http://{}", bind_addr);
            if !app_path.is_empty() {
                println!("  App: {}", app_path);
            }
            println!("  Reload: enabled");
            println!("  Reload dirs: {:?}", watch_paths);
            println!("  Reload include: {:?}", watch_extensions);
            println!("  Log level: {}", log_level);
            if access_log {
                println!("  Access log: enabled");
            }

            // TODO: Pass python_routes to DevServer for hot reload
            let _ = python_routes;

            rt.block_on(async {
                let dev_server = DevServer::new(dev_config);
                dev_server.run().await
                    .map_err(|e| anyhow::anyhow!("Server error: {}", e))
            })
        }

        #[cfg(not(feature = "dev"))]
        {
            let _ = (reload_dir, reload_include, reload_exclude, reload_delay, python_routes);
            anyhow::bail!(
                "Hot reload requires the 'dev' feature.\n\
                 Rebuild with: cargo build -p ouroboros-cli --features dev\n\
                 Or run without --reload for production mode."
            )
        }
    } else {
        // Production mode (no reload)
        println!("Starting API server...");
        println!("  Address: http://{}", bind_addr);
        if !app_path.is_empty() {
            println!("  App: {}", app_path);
        }
        println!("  Log level: {}", log_level);
        if access_log {
            println!("  Access log: enabled");
        }

        rt.block_on(async {
            let mut router = Router::new();

            // Register Python routes if loaded
            if let Some(routes) = python_routes {
                // Initialize PyLoop for Python handler execution
                let pyloop = Python::with_gil(|_py| {
                    PyLoop::new()
                        .map(Arc::new)
                        .map_err(|e| anyhow::anyhow!("Failed to create PyLoop: {}", e))
                })?;

                for route_info in routes {
                    let method = match route_info.method.to_uppercase().as_str() {
                        "GET" => ouroboros_api::HttpMethod::Get,
                        "POST" => ouroboros_api::HttpMethod::Post,
                        "PUT" => ouroboros_api::HttpMethod::Put,
                        "DELETE" => ouroboros_api::HttpMethod::Delete,
                        "PATCH" => ouroboros_api::HttpMethod::Patch,
                        "HEAD" => ouroboros_api::HttpMethod::Head,
                        "OPTIONS" => ouroboros_api::HttpMethod::Options,
                        _ => {
                            println!("  Skipping unsupported method: {}", route_info.method);
                            continue;
                        }
                    };

                    // Create PythonHandler for this route
                    let handler = PythonHandler::new(route_info.handler, pyloop.clone());
                    let handler_fn = handler.into_handler_fn();

                    // Register route
                    router.route(
                        method,
                        &route_info.path,
                        handler_fn,
                        RequestValidator::new(),
                        HandlerMeta::new(route_info.handler_name),
                    ).map_err(|e| anyhow::anyhow!("Failed to register route: {}", e))?;
                }
            }

            let server_config = ServerConfig::new(&bind_addr);
            let server = Server::new(router, server_config);

            server.run().await
                .map_err(|e| anyhow::anyhow!("Server error: {}", e))
        })
    }
}

/// Initialize logging based on log level
fn init_logging(level: &str) -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .try_init()
        .ok(); // Ignore error if already initialized

    Ok(())
}
