//! Ouroboros CLI - Unified command-line interface
//!
//! Usage:
//!   ob qc run [path]       Run tests
//!   ob qc run --bench      Run benchmarks
//!   ob qc run --security   Run security tests
//!   ob qc collect [path]   Collect tests without running
//!   ob qc run -k <pattern> Filter tests by pattern

use clap::{Parser, Subcommand};
use anyhow::Result;

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
        #[arg(default_value = ".")]
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
        #[arg(default_value = ".")]
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
                if bench {
                    println!("Running benchmarks in: {}", path);
                } else if security {
                    println!("Running security tests in: {}", path);
                } else {
                    println!("Running tests in: {}", path);
                }

                if let Some(p) = &pattern {
                    println!("  Filter: {}", p);
                }
                if verbose {
                    println!("  Verbose: enabled");
                }
                if fail_fast {
                    println!("  Fail-fast: enabled");
                }

                // TODO: Wire to actual test runner
                println!("\n[TODO] Test runner not yet implemented in CLI");
            }

            QcAction::Collect { path, pattern } => {
                println!("Collecting tests in: {}", path);
                if let Some(p) = &pattern {
                    println!("  Filter: {}", p);
                }

                // TODO: Wire to discovery engine
                println!("\n[TODO] Test collection not yet implemented in CLI");
            }
        },
    }

    Ok(())
}
