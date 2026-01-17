//! Output formatters for lint results

use crate::diagnostic::DiagnosticSeverity;
use crate::FileResult;
use serde::Serialize;

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Markdown,
    Console,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(OutputFormat::Json),
            "markdown" | "md" => Some(OutputFormat::Markdown),
            "console" | "text" => Some(OutputFormat::Console),
            _ => None,
        }
    }
}

/// Reporter for generating output
pub struct Reporter {
    format: OutputFormat,
}

impl Reporter {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn generate(&self, results: &[FileResult]) -> String {
        match self.format {
            OutputFormat::Json => self.generate_json(results),
            OutputFormat::Markdown => self.generate_markdown(results),
            OutputFormat::Console => self.generate_console(results),
        }
    }

    fn generate_json(&self, results: &[FileResult]) -> String {
        #[derive(Serialize)]
        struct JsonOutput<'a> {
            files: Vec<JsonFile<'a>>,
            summary: JsonSummary,
        }

        #[derive(Serialize)]
        struct JsonFile<'a> {
            path: String,
            language: &'a str,
            diagnostics: &'a [crate::Diagnostic],
        }

        #[derive(Serialize)]
        struct JsonSummary {
            files_checked: usize,
            files_with_issues: usize,
            total_errors: usize,
            total_warnings: usize,
        }

        let mut total_errors = 0;
        let mut total_warnings = 0;
        let mut files_with_issues = 0;

        let files: Vec<JsonFile> = results
            .iter()
            .map(|r| {
                if !r.diagnostics.is_empty() {
                    files_with_issues += 1;
                }
                for d in &r.diagnostics {
                    match d.severity {
                        DiagnosticSeverity::Error => total_errors += 1,
                        DiagnosticSeverity::Warning => total_warnings += 1,
                        _ => {}
                    }
                }
                JsonFile {
                    path: r.path.to_string_lossy().to_string(),
                    language: r.language.as_str(),
                    diagnostics: &r.diagnostics,
                }
            })
            .collect();

        let output = JsonOutput {
            files,
            summary: JsonSummary {
                files_checked: results.len(),
                files_with_issues,
                total_errors,
                total_warnings,
            },
        };

        serde_json::to_string_pretty(&output).unwrap_or_default()
    }

    fn generate_markdown(&self, results: &[FileResult]) -> String {
        let mut output = String::new();
        output.push_str("# Lint Report\n\n");

        let mut total_errors = 0;
        let mut total_warnings = 0;

        for result in results {
            if result.diagnostics.is_empty() {
                continue;
            }

            output.push_str(&format!("## {}\n\n", result.path.display()));

            for diag in &result.diagnostics {
                let emoji = match diag.severity {
                    DiagnosticSeverity::Error => {
                        total_errors += 1;
                        "❌"
                    }
                    DiagnosticSeverity::Warning => {
                        total_warnings += 1;
                        "⚠️"
                    }
                    DiagnosticSeverity::Information => "ℹ️",
                    DiagnosticSeverity::Hint => "💡",
                };

                output.push_str(&format!(
                    "- {} **{}** (line {}): {}\n",
                    emoji,
                    diag.code,
                    diag.range.start.line + 1,
                    diag.message
                ));
            }

            output.push('\n');
        }

        output.push_str("## Summary\n\n");
        output.push_str(&format!("- Files checked: {}\n", results.len()));
        output.push_str(&format!("- Errors: {}\n", total_errors));
        output.push_str(&format!("- Warnings: {}\n", total_warnings));

        output
    }

    fn generate_console(&self, results: &[FileResult]) -> String {
        let mut output = String::new();
        let mut total_errors = 0;
        let mut total_warnings = 0;

        for result in results {
            if result.diagnostics.is_empty() {
                continue;
            }

            for diag in &result.diagnostics {
                let severity_str = match diag.severity {
                    DiagnosticSeverity::Error => {
                        total_errors += 1;
                        "error"
                    }
                    DiagnosticSeverity::Warning => {
                        total_warnings += 1;
                        "warning"
                    }
                    DiagnosticSeverity::Information => "info",
                    DiagnosticSeverity::Hint => "hint",
                };

                output.push_str(&format!(
                    "{}:{}:{}: {} [{}]: {}\n",
                    result.path.display(),
                    diag.range.start.line + 1,
                    diag.range.start.character + 1,
                    severity_str,
                    diag.code,
                    diag.message
                ));
            }
        }

        if total_errors > 0 || total_warnings > 0 {
            output.push_str(&format!(
                "\n{} error(s), {} warning(s)\n",
                total_errors, total_warnings
            ));
        }

        output
    }
}
