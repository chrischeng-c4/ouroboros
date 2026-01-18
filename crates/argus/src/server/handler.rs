//! Request handlers for Argus daemon

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::diagnostic::Diagnostic;
use crate::lint::CheckerRegistry;
use crate::semantic::{SymbolTable, SymbolTableBuilder};
use crate::syntax::{Language, MultiParser, ParsedFile};
use crate::types::StubLoader;
use crate::LintConfig;

use super::protocol::*;

/// Cached analysis for a file
struct FileAnalysis {
    parsed: ParsedFile,
    symbol_table: SymbolTable,
    diagnostics: Vec<Diagnostic>,
}

/// Request handler with caching
pub struct RequestHandler {
    /// Root directory being analyzed
    root: PathBuf,
    /// File cache: path -> analysis
    cache: Arc<RwLock<HashMap<PathBuf, FileAnalysis>>>,
    /// Checker registry
    registry: Arc<CheckerRegistry>,
    /// Lint configuration
    config: Arc<LintConfig>,
    /// Type stubs
    stubs: Arc<RwLock<StubLoader>>,
    /// Parser (not thread-safe, needs mutex)
    parser: Arc<tokio::sync::Mutex<MultiParser>>,
}

impl RequestHandler {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        let parser = MultiParser::new().map_err(|e| format!("Failed to create parser: {}", e))?;

        let mut stubs = StubLoader::new();
        stubs.load_builtins();

        Ok(Self {
            root,
            cache: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(CheckerRegistry::new()),
            config: Arc::new(LintConfig::default()),
            stubs: Arc::new(RwLock::new(stubs)),
            parser: Arc::new(tokio::sync::Mutex::new(parser)),
        })
    }

    /// Handle a JSON-RPC request
    pub async fn handle(&self, request: Request) -> Response {
        let result = match request.method.as_str() {
            "check" => self.handle_check(request.params).await,
            "type_at" => self.handle_type_at(request.params).await,
            "symbols" => self.handle_symbols(request.params).await,
            "diagnostics" => self.handle_diagnostics(request.params).await,
            "hover" => self.handle_hover(request.params).await,
            "definition" => self.handle_definition(request.params).await,
            "references" => self.handle_references(request.params).await,
            "index_status" => self.handle_index_status().await,
            "invalidate" => self.handle_invalidate(request.params).await,
            "shutdown" => self.handle_shutdown().await,
            _ => Err(RpcError::method_not_found(&request.method)),
        };

        match result {
            Ok(value) => Response::success(request.id, value),
            Err(error) => Response::error(request.id, error),
        }
    }

    /// Check files/directories for issues
    async fn handle_check(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params: CheckParams = params
            .ok_or_else(|| RpcError::invalid_params("Missing params"))?
            .try_into()
            .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;

        let path = self.resolve_path(&params.path);

        let mut all_diagnostics = Vec::new();
        let mut files_checked = 0;

        if path.is_file() {
            if let Some(diags) = self.check_file(&path).await {
                files_checked = 1;
                all_diagnostics.extend(diags);
            }
        } else if path.is_dir() {
            let files = self.collect_files(&path);
            for file in files {
                if let Some(diags) = self.check_file(&file).await {
                    files_checked += 1;
                    all_diagnostics.extend(diags);
                }
            }
        } else {
            return Err(RpcError::invalid_params(format!("Path not found: {}", params.path)));
        }

        let errors = all_diagnostics.iter().filter(|d| d.severity == "error").count();
        let warnings = all_diagnostics.iter().filter(|d| d.severity == "warning").count();

        let result = CheckResult {
            diagnostics: all_diagnostics,
            files_checked,
            errors,
            warnings,
        };

        serde_json::to_value(result).map_err(|e| RpcError::internal_error(e.to_string()))
    }

    /// Get type at position
    async fn handle_type_at(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params: TypeAtParams = params
            .ok_or_else(|| RpcError::invalid_params("Missing params"))?
            .try_into()
            .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;

        let path = self.resolve_path(&params.file);
        self.ensure_analyzed(&path).await?;

        let cache = self.cache.read().await;
        let analysis = cache.get(&path)
            .ok_or_else(|| RpcError::invalid_params("File not found in cache"))?;

        // Find symbol at position
        let symbol = analysis.symbol_table.find_at_position(params.line, params.column);

        match symbol {
            Some(sym) => {
                let type_str = sym.type_info.as_ref()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_else(|| "Unknown".to_string());
                serde_json::to_value(type_str).map_err(|e| RpcError::internal_error(e.to_string()))
            }
            None => Ok(serde_json::Value::Null),
        }
    }

    /// List symbols in a file
    async fn handle_symbols(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params: SymbolsParams = params
            .ok_or_else(|| RpcError::invalid_params("Missing params"))?
            .try_into()
            .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;

        let path = self.resolve_path(&params.file);
        self.ensure_analyzed(&path).await?;

        let cache = self.cache.read().await;
        let analysis = cache.get(&path)
            .ok_or_else(|| RpcError::invalid_params("File not found in cache"))?;

        let symbols: Vec<SymbolInfo> = analysis.symbol_table.all_symbols()
            .iter()
            .map(|sym| SymbolInfo {
                name: sym.name.clone(),
                kind: format!("{:?}", sym.kind),
                line: sym.location.start.line,
                column: sym.location.start.character,
                type_info: sym.type_info.as_ref().map(|t| format!("{:?}", t)),
            })
            .collect();

        serde_json::to_value(symbols).map_err(|e| RpcError::internal_error(e.to_string()))
    }

    /// Get diagnostics
    async fn handle_diagnostics(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params: DiagnosticsParams = params
            .map(|p| serde_json::from_value(p).ok())
            .flatten()
            .unwrap_or(DiagnosticsParams { file: None });

        let cache = self.cache.read().await;

        let diagnostics: Vec<DiagnosticInfo> = if let Some(file) = params.file {
            let path = self.resolve_path(&file);
            cache.get(&path)
                .map(|a| self.convert_diagnostics(&path, &a.diagnostics))
                .unwrap_or_default()
        } else {
            cache.iter()
                .flat_map(|(path, analysis)| self.convert_diagnostics(path, &analysis.diagnostics))
                .collect()
        };

        serde_json::to_value(diagnostics).map_err(|e| RpcError::internal_error(e.to_string()))
    }

    /// Get hover information
    async fn handle_hover(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params: HoverParams = params
            .ok_or_else(|| RpcError::invalid_params("Missing params"))?
            .try_into()
            .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;

        let path = self.resolve_path(&params.file);
        self.ensure_analyzed(&path).await?;

        let cache = self.cache.read().await;
        let analysis = cache.get(&path)
            .ok_or_else(|| RpcError::invalid_params("File not found in cache"))?;

        let symbol = analysis.symbol_table.find_at_position(params.line, params.column);

        match symbol {
            Some(sym) => {
                let content = sym.hover_content(Language::Python);
                serde_json::to_value(content).map_err(|e| RpcError::internal_error(e.to_string()))
            }
            None => Ok(serde_json::Value::Null),
        }
    }

    /// Go to definition
    async fn handle_definition(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params: DefinitionParams = params
            .ok_or_else(|| RpcError::invalid_params("Missing params"))?
            .try_into()
            .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;

        let path = self.resolve_path(&params.file);
        self.ensure_analyzed(&path).await?;

        let cache = self.cache.read().await;
        let analysis = cache.get(&path)
            .ok_or_else(|| RpcError::invalid_params("File not found in cache"))?;

        let symbol = analysis.symbol_table.find_definition_at(params.line, params.column);

        match symbol {
            Some(sym) => {
                let loc = LocationInfo {
                    file: path.to_string_lossy().to_string(),
                    line: sym.location.start.line,
                    column: sym.location.start.character,
                    end_line: sym.location.end.line,
                    end_column: sym.location.end.character,
                };
                serde_json::to_value(loc).map_err(|e| RpcError::internal_error(e.to_string()))
            }
            None => Ok(serde_json::Value::Null),
        }
    }

    /// Find references
    async fn handle_references(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        let params: ReferencesParams = params
            .ok_or_else(|| RpcError::invalid_params("Missing params"))?
            .try_into()
            .map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;

        let path = self.resolve_path(&params.file);
        self.ensure_analyzed(&path).await?;

        let cache = self.cache.read().await;
        let analysis = cache.get(&path)
            .ok_or_else(|| RpcError::invalid_params("File not found in cache"))?;

        let refs = analysis.symbol_table.find_references_at(
            params.line,
            params.column,
            params.include_declaration,
        );

        let locations: Vec<LocationInfo> = refs
            .into_iter()
            .map(|r| LocationInfo {
                file: path.to_string_lossy().to_string(),
                line: r.start.line,
                column: r.start.character,
                end_line: r.end.line,
                end_column: r.end.character,
            })
            .collect();

        serde_json::to_value(locations).map_err(|e| RpcError::internal_error(e.to_string()))
    }

    /// Get index status
    async fn handle_index_status(&self) -> Result<serde_json::Value, RpcError> {
        let cache = self.cache.read().await;

        let total_symbols: usize = cache.values()
            .map(|a| a.symbol_table.all_symbols().len())
            .sum();

        let status = IndexStatus {
            indexed_files: cache.len(),
            total_symbols,
            last_updated: None, // TODO: track last update time
            is_ready: true,
        };

        serde_json::to_value(status).map_err(|e| RpcError::internal_error(e.to_string()))
    }

    /// Invalidate cache for files
    async fn handle_invalidate(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, RpcError> {
        #[derive(serde::Deserialize)]
        struct InvalidateParams {
            files: Vec<String>,
        }

        let params: InvalidateParams = serde_json::from_value(
            params.ok_or_else(|| RpcError::invalid_params("Missing params"))?
        ).map_err(|e| RpcError::invalid_params(format!("Invalid params: {}", e)))?;

        let mut cache = self.cache.write().await;
        let mut invalidated = 0;

        for file in params.files {
            let path = self.resolve_path(&file);
            if cache.remove(&path).is_some() {
                invalidated += 1;
            }
        }

        serde_json::to_value(serde_json::json!({ "invalidated": invalidated }))
            .map_err(|e| RpcError::internal_error(e.to_string()))
    }

    /// Shutdown the daemon
    async fn handle_shutdown(&self) -> Result<serde_json::Value, RpcError> {
        // The actual shutdown is handled by the daemon
        Ok(serde_json::json!({ "status": "shutting_down" }))
    }

    // =========================================================================
    // Helper methods
    // =========================================================================

    /// Resolve a path relative to root
    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else {
            self.root.join(p)
        }
    }

    /// Collect all analyzable files in a directory
    fn collect_files(&self, dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Skip excluded patterns
                if self.config.is_excluded(&path) {
                    continue;
                }

                if path.is_dir() {
                    files.extend(self.collect_files(&path));
                } else if path.is_file() {
                    if MultiParser::detect_language(&path).is_some() {
                        files.push(path);
                    }
                }
            }
        }

        files
    }

    /// Ensure a file is analyzed and cached
    async fn ensure_analyzed(&self, path: &Path) -> Result<(), RpcError> {
        {
            let cache = self.cache.read().await;
            if cache.contains_key(path) {
                return Ok(());
            }
        }

        self.check_file(path).await;
        Ok(())
    }

    /// Check a single file and cache the results
    async fn check_file(&self, path: &Path) -> Option<Vec<DiagnosticInfo>> {
        let language = MultiParser::detect_language(path)?;

        if !self.config.is_language_enabled(language) {
            return None;
        }

        let source = std::fs::read_to_string(path).ok()?;

        // Parse
        let parsed = {
            let mut parser = self.parser.lock().await;
            parser.parse(&source, language)?
        };

        // Run linting
        let checker = self.registry.get(language)?;
        let diagnostics = checker.check(&parsed, &self.config);

        // Build symbol table
        let symbol_table = if language == Language::Python {
            SymbolTableBuilder::new().build_python(&parsed)
        } else {
            SymbolTable::default()
        };

        let diag_infos = self.convert_diagnostics(path, &diagnostics);

        // Cache the analysis
        {
            let mut cache = self.cache.write().await;
            cache.insert(path.to_path_buf(), FileAnalysis {
                parsed,
                symbol_table,
                diagnostics,
            });
        }

        Some(diag_infos)
    }

    /// Convert diagnostics to protocol format
    fn convert_diagnostics(&self, path: &Path, diagnostics: &[Diagnostic]) -> Vec<DiagnosticInfo> {
        diagnostics.iter().map(|d| DiagnosticInfo {
            file: path.to_string_lossy().to_string(),
            line: d.range.start.line,
            column: d.range.start.character,
            end_line: d.range.end.line,
            end_column: d.range.end.character,
            severity: format!("{:?}", d.severity).to_lowercase(),
            code: d.code.clone(),
            message: d.message.clone(),
        }).collect()
    }
}

// Implement TryFrom for param types
impl TryFrom<serde_json::Value> for CheckParams {
    type Error = serde_json::Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<serde_json::Value> for TypeAtParams {
    type Error = serde_json::Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<serde_json::Value> for SymbolsParams {
    type Error = serde_json::Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<serde_json::Value> for HoverParams {
    type Error = serde_json::Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<serde_json::Value> for DefinitionParams {
    type Error = serde_json::Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl TryFrom<serde_json::Value> for ReferencesParams {
    type Error = serde_json::Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}
