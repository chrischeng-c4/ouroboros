//! Argus: Unified multi-language code analysis tool
//!
//! Provides LSP + Linting for Python, TypeScript, and Rust.

pub mod core;
pub mod diagnostic;
pub mod lint;
pub mod lsp;
pub mod mcp;
pub mod output;
pub mod semantic;
pub mod server;
pub mod syntax;
pub mod types;
pub mod watch;

pub use core::{ArgusConfig, LanguageConfig};
pub use diagnostic::{Diagnostic, DiagnosticCategory, DiagnosticSeverity, Position, Range};
pub use lint::{Checker, CheckerRegistry};
pub use output::{OutputFormat, Reporter};
pub use syntax::{Language, MultiParser, ParsedFile};
pub use mcp::{McpServer, ArgusTools};
pub use server::{ArgusDaemon, DaemonClient, DaemonConfig, RequestHandler};
pub use watch::{FileWatcher, WatchConfig, WatchEvent};

use std::path::Path;

/// Check files and return diagnostics
pub fn check_paths(paths: &[&Path], config: &LintConfig) -> Vec<FileResult> {
    let registry = CheckerRegistry::new();
    let mut parser = MultiParser::new().expect("Failed to initialize parser");
    let mut results = Vec::new();

    for path in paths {
        if path.is_file() {
            if let Some(result) = check_file(&mut parser, &registry, path, config) {
                results.push(result);
            }
        } else if path.is_dir() {
            results.extend(check_directory(&mut parser, &registry, path, config));
        }
    }

    results
}

/// Check a single file
fn check_file(
    parser: &mut MultiParser,
    registry: &CheckerRegistry,
    path: &Path,
    config: &LintConfig,
) -> Option<FileResult> {
    let language = MultiParser::detect_language(path)?;

    if !config.is_language_enabled(language) {
        return None;
    }

    let source = std::fs::read_to_string(path).ok()?;
    let parsed = parser.parse(&source, language)?;

    let checker = registry.get(language)?;
    let diagnostics = checker.check(&parsed, config);

    Some(FileResult {
        path: path.to_path_buf(),
        language,
        diagnostics,
    })
}

/// Check all files in a directory
fn check_directory(
    parser: &mut MultiParser,
    registry: &CheckerRegistry,
    dir: &Path,
    config: &LintConfig,
) -> Vec<FileResult> {
    use jwalk::WalkDir;

    let mut results = Vec::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip excluded patterns
        if config.is_excluded(&path) {
            continue;
        }

        if let Some(result) = check_file(parser, registry, &path, config) {
            results.push(result);
        }
    }

    results
}

/// Result of checking a single file
#[derive(Debug)]
pub struct FileResult {
    pub path: std::path::PathBuf,
    pub language: Language,
    pub diagnostics: Vec<Diagnostic>,
}

impl FileResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count()
    }
}

/// Lint configuration
#[derive(Debug, Clone)]
pub struct LintConfig {
    pub languages: Vec<Language>,
    pub exclude_patterns: Vec<String>,
    pub min_severity: DiagnosticSeverity,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            languages: vec![Language::Python, Language::TypeScript, Language::Rust],
            exclude_patterns: vec![
                "__pycache__".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                ".venv".to_string(),
            ],
            min_severity: DiagnosticSeverity::Warning,
        }
    }
}

impl LintConfig {
    pub fn is_language_enabled(&self, lang: Language) -> bool {
        self.languages.contains(&lang)
    }

    pub fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.exclude_patterns.iter().any(|p| path_str.contains(p))
    }
}
