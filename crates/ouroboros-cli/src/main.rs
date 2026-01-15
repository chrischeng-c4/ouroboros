//! Ouroboros CLI - Unified command-line interface
//!
//! Usage:
//!   ob qc run [path]       Run tests
//!   ob qc run --bench      Run benchmarks
//!   ob qc run --security   Run security tests
//!   ob qc collect [path]   Collect tests without running
//!   ob qc run -k <pattern> Filter tests by pattern

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use pyo3::prelude::*;
use std::path::PathBuf;

use ouroboros_qc::{DiscoveryConfig, FileType, walk_files};

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Qc { action } => match action {
            QcAction::Run {
                path,
                bench,
                security,
                pattern,
                verbose,
                fail_fast,
            } => {
                run_tests(&path, bench, security, pattern, verbose, fail_fast)?;
            }

            QcAction::Collect { path, pattern } => {
                collect_tests(&path, pattern)?;
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
fn run_tests(
    path: &str,
    bench: bool,
    _security: bool,
    pattern: Option<String>,
    verbose: bool,
    fail_fast: bool,
) -> Result<()> {
    let file_type = if bench { FileType::Benchmark } else { FileType::Test };

    println!("🔍 Discovering {} files in {}...",
        if bench { "benchmark" } else { "test" },
        path
    );

    let files = discover_test_files(path, file_type)?;

    if files.is_empty() {
        println!("❌ No {} files found", if bench { "benchmark" } else { "test" });
        return Ok(());
    }

    println!("✅ Found {} file(s)", files.len());

    // Initialize Python
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| -> Result<()> {
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

            if verbose {
                println!("\n📄 Loading: {}", file_info.module_name);
            }

            match run_test_file(py, &file_info.path, &file_info.module_name, verbose) {
                Ok((passed, failed)) => {
                    total_passed += passed;
                    total_failed += failed;

                    if fail_fast && failed > 0 {
                        println!("\n❌ Stopping due to --fail-fast");
                        break;
                    }
                }
                Err(e) => {
                    println!("❌ Error loading {}: {}", file_info.module_name, e);
                    total_errors += 1;
                    if fail_fast {
                        break;
                    }
                }
            }
        }

        // Print summary
        println!("\n{}", "=".repeat(60));
        println!("TEST SUMMARY");
        println!("{}", "=".repeat(60));
        println!("✅ Passed:  {}", total_passed);
        println!("❌ Failed:  {}", total_failed);
        println!("⚠️  Errors:  {}", total_errors);
        println!("{}", "=".repeat(60));

        Ok(())
    })?;

    Ok(())
}

/// Run a single test file and return (passed, failed) counts
fn run_test_file(py: Python<'_>, file_path: &std::path::Path, module_name: &str, verbose: bool) -> Result<(u32, u32)> {
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
            Ok((p, f)) => {
                total_passed += p;
                total_failed += f;
            }
            Err(e) => {
                println!("  ❌ Error running {}: {}", name_str, e);
                total_failed += 1;
            }
        }
    }

    Ok((total_passed, total_failed))
}

/// Run a TestSuite class and return (passed, failed) counts
fn run_test_suite(py: Python<'_>, suite_class: &Bound<'_, PyAny>, verbose: bool) -> Result<(u32, u32)> {
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

    Ok((passed, failed))
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
