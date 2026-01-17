//! Argus LSP Server implementation

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::diagnostic::{Diagnostic as ArgusDiagnostic, DiagnosticSeverity as ArgusSeverity};
use crate::lint::CheckerRegistry;
use crate::semantic::{SymbolTable, SymbolTableBuilder};
use crate::syntax::{MultiParser, ParsedFile};
use crate::{LintConfig, Language};

/// Document state tracked by the server
#[derive(Debug)]
struct Document {
    content: String,
    language: Language,
    version: i32,
}

/// Cached analysis for a document
struct DocumentAnalysis {
    symbol_table: SymbolTable,
    /// Diagnostics with their quick fixes
    diagnostics: Vec<ArgusDiagnostic>,
}

/// Argus Language Server
pub struct ArgusServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, Document>>>,
    analyses: Arc<RwLock<HashMap<Url, DocumentAnalysis>>>,
    registry: Arc<CheckerRegistry>,
    config: Arc<LintConfig>,
}

impl ArgusServer {
    /// Create a new Argus server
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            analyses: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(CheckerRegistry::new()),
            config: Arc::new(LintConfig::default()),
        }
    }

    /// Analyze a document and publish diagnostics
    async fn analyze_document(&self, uri: &Url) {
        let documents = self.documents.read().await;
        let Some(doc) = documents.get(uri) else {
            return;
        };

        let content = doc.content.clone();
        let language = doc.language;
        drop(documents);

        // Parse
        let mut parser = match MultiParser::new() {
            Ok(p) => p,
            Err(_) => return,
        };

        let parsed = match parser.parse(&content, language) {
            Some(p) => p,
            None => return,
        };

        // Run linting
        let diagnostics = self.run_lint(&parsed, language);

        // Build symbol table (currently only for Python) and store diagnostics
        let symbol_table = if language == Language::Python {
            SymbolTableBuilder::new().build_python(&parsed)
        } else {
            SymbolTable::default()
        };

        // Store analysis with diagnostics for code actions
        {
            let mut analyses = self.analyses.write().await;
            analyses.insert(uri.clone(), DocumentAnalysis {
                symbol_table,
                diagnostics: diagnostics.clone(),
            });
        }

        // Convert to LSP diagnostics
        let lsp_diagnostics: Vec<tower_lsp::lsp_types::Diagnostic> = diagnostics
            .into_iter()
            .map(|d| self.to_lsp_diagnostic(&d))
            .collect();

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri.clone(), lsp_diagnostics, None)
            .await;
    }

    /// Run linting on parsed file
    fn run_lint(&self, parsed: &ParsedFile, language: Language) -> Vec<ArgusDiagnostic> {
        let checker = match self.registry.get(language) {
            Some(c) => c,
            None => return Vec::new(),
        };

        checker.check(parsed, &self.config)
    }

    /// Convert Argus diagnostic to LSP diagnostic
    fn to_lsp_diagnostic(&self, diag: &ArgusDiagnostic) -> tower_lsp::lsp_types::Diagnostic {
        tower_lsp::lsp_types::Diagnostic {
            range: Range {
                start: Position {
                    line: diag.range.start.line,
                    character: diag.range.start.character,
                },
                end: Position {
                    line: diag.range.end.line,
                    character: diag.range.end.character,
                },
            },
            severity: Some(self.to_lsp_severity(diag.severity)),
            code: Some(NumberOrString::String(diag.code.clone())),
            code_description: None,
            source: Some("argus".to_string()),
            message: diag.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Convert Argus severity to LSP severity
    fn to_lsp_severity(&self, severity: ArgusSeverity) -> DiagnosticSeverity {
        match severity {
            ArgusSeverity::Error => DiagnosticSeverity::ERROR,
            ArgusSeverity::Warning => DiagnosticSeverity::WARNING,
            ArgusSeverity::Information => DiagnosticSeverity::INFORMATION,
            ArgusSeverity::Hint => DiagnosticSeverity::HINT,
        }
    }

    /// Detect language from URI
    fn detect_language(uri: &Url) -> Option<Language> {
        let path = PathBuf::from(uri.path());
        MultiParser::detect_language(&path)
    }

    /// Convert Argus Range to LSP Range
    fn to_lsp_range(range: &crate::diagnostic::Range) -> Range {
        Range {
            start: Position {
                line: range.start.line,
                character: range.start.character,
            },
            end: Position {
                line: range.end.line,
                character: range.end.character,
            },
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ArgusServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                // Semantic capabilities
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                // Code actions (quick fixes)
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "argus".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Argus LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        let Some(language) = Self::detect_language(&uri) else {
            return;
        };

        // Store document
        {
            let mut documents = self.documents.write().await;
            documents.insert(
                uri.clone(),
                Document {
                    content,
                    language,
                    version,
                },
            );
        }

        // Analyze
        self.analyze_document(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // Get full content (we use FULL sync)
        let Some(change) = params.content_changes.into_iter().next() else {
            return;
        };

        // Update document
        {
            let mut documents = self.documents.write().await;
            if let Some(doc) = documents.get_mut(&uri) {
                doc.content = change.text;
                doc.version = version;
            }
        }

        // Analyze
        self.analyze_document(&uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        // Update content if provided
        if let Some(text) = params.text {
            let mut documents = self.documents.write().await;
            if let Some(doc) = documents.get_mut(&uri) {
                doc.content = text;
            }
        }

        // Re-analyze
        self.analyze_document(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        // Remove document and analysis
        {
            let mut documents = self.documents.write().await;
            documents.remove(&uri);
        }
        {
            let mut analyses = self.analyses.write().await;
            analyses.remove(&uri);
        }

        // Clear diagnostics
        self.client
            .publish_diagnostics(uri, Vec::new(), None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get document language
        let language = {
            let documents = self.documents.read().await;
            documents.get(uri).map(|d| d.language)
        };

        let Some(language) = language else {
            return Ok(None);
        };

        // Get analysis
        let analyses = self.analyses.read().await;
        let Some(analysis) = analyses.get(uri) else {
            return Ok(None);
        };

        // Find symbol at position
        let symbol = analysis.symbol_table.find_at_position(position.line, position.character);

        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // Generate hover content
        let content = symbol.hover_content(language);

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: Some(Self::to_lsp_range(&symbol.location)),
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get analysis
        let analyses = self.analyses.read().await;
        let Some(analysis) = analyses.get(uri) else {
            return Ok(None);
        };

        // Find definition
        let symbol = analysis.symbol_table.find_definition_at(position.line, position.character);

        let Some(symbol) = symbol else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range: Self::to_lsp_range(&symbol.location),
        })))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        // Get analysis
        let analyses = self.analyses.read().await;
        let Some(analysis) = analyses.get(uri) else {
            return Ok(None);
        };

        // Find references
        let references = analysis.symbol_table.find_references_at(
            position.line,
            position.character,
            include_declaration,
        );

        if references.is_empty() {
            return Ok(None);
        }

        let locations: Vec<Location> = references
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: Self::to_lsp_range(&r),
            })
            .collect();

        Ok(Some(locations))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let request_range = params.range;

        // Get analysis with diagnostics
        let analyses = self.analyses.read().await;
        let Some(analysis) = analyses.get(uri) else {
            return Ok(None);
        };

        let mut actions: Vec<CodeActionOrCommand> = Vec::new();

        // Find diagnostics that overlap with the requested range
        for diag in &analysis.diagnostics {
            let diag_range = Self::to_lsp_range(&diag.range);

            // Check if diagnostic overlaps with requested range
            if !Self::ranges_overlap(&diag_range, &request_range) {
                continue;
            }

            // Create code actions for each quick fix
            for fix in &diag.quick_fixes {
                let edits: Vec<TextEdit> = fix
                    .edits
                    .iter()
                    .map(|e| TextEdit {
                        range: Self::to_lsp_range(&e.range),
                        new_text: e.new_text.clone(),
                    })
                    .collect();

                let mut changes = HashMap::new();
                changes.insert(uri.clone(), edits);

                let action = CodeAction {
                    title: fix.title.clone(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![self.to_lsp_diagnostic(diag)]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command: None,
                    is_preferred: Some(true),
                    disabled: None,
                    data: None,
                };

                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
}

impl ArgusServer {
    /// Check if two ranges overlap
    fn ranges_overlap(a: &Range, b: &Range) -> bool {
        // a starts before b ends AND a ends after b starts
        (a.start.line < b.end.line || (a.start.line == b.end.line && a.start.character <= b.end.character))
            && (a.end.line > b.start.line || (a.end.line == b.start.line && a.end.character >= b.start.character))
    }
}

/// Run the LSP server on stdio
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ArgusServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Run the LSP server on TCP (for debugging)
pub async fn run_server_tcp(port: u16) -> std::io::Result<()> {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("Argus LSP server listening on port {}", port);

    loop {
        let (stream, addr) = listener.accept().await?;
        tracing::info!("Client connected from {}", addr);

        let (read, write) = tokio::io::split(stream);
        let (service, socket) = LspService::new(ArgusServer::new);

        tokio::spawn(async move {
            Server::new(read, write, socket).serve(service).await;
        });
    }
}
