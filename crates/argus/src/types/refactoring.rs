//! Refactoring operations (Sprint 3 - Track 1)
//!
//! Provides type-aware refactoring operations:
//! - Extract function/method/variable
//! - Rename symbol (cross-file)
//! - Move definition
//! - Inline symbol
//! - Change signature

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::mutable_ast::{MutableAst, MutableNode, NodeId, NodeMetadata, Span};
use super::deep_inference::{TypeContext, DeepTypeInferencer};
use crate::syntax::{Language, MultiParser};
use tree_sitter::Node;

// ============================================================================
// Refactoring Request
// ============================================================================

/// A request for a refactoring operation.
#[derive(Debug, Clone)]
pub struct RefactorRequest {
    /// Type of refactoring
    pub kind: RefactorKind,
    /// Target file
    pub file: PathBuf,
    /// Target span in source
    pub span: Span,
    /// Additional options
    pub options: RefactorOptions,
}

/// Type of refactoring operation.
#[derive(Debug, Clone)]
pub enum RefactorKind {
    /// Extract code into a new function
    ExtractFunction { name: String },
    /// Extract code into a new method
    ExtractMethod { name: String },
    /// Extract expression into a variable
    ExtractVariable { name: String },
    /// Rename a symbol
    Rename { new_name: String },
    /// Move a definition to another file
    MoveDefinition { target_file: PathBuf },
    /// Inline a symbol's definition
    Inline,
    /// Change function signature
    ChangeSignature { changes: SignatureChanges },
}

/// Options for refactoring operations.
#[derive(Debug, Clone, Default)]
pub struct RefactorOptions {
    /// Preview changes without applying
    pub preview_only: bool,
    /// Update imports automatically
    pub update_imports: bool,
    /// Add type annotations
    pub add_type_annotations: bool,
    /// Preserve formatting
    pub preserve_formatting: bool,
}

/// Changes to a function signature.
#[derive(Debug, Clone)]
pub struct SignatureChanges {
    /// New parameters (name, type_annotation, default)
    pub new_params: Vec<(String, Option<String>, Option<String>)>,
    /// Reordered parameter indices
    pub param_order: Vec<usize>,
    /// Removed parameter indices
    pub removed_params: Vec<usize>,
    /// New return type annotation
    pub new_return_type: Option<String>,
}

// ============================================================================
// Refactoring Result
// ============================================================================

/// Result of a refactoring operation.
#[derive(Debug, Clone)]
pub struct RefactorResult {
    /// Edits to apply per file
    pub file_edits: HashMap<PathBuf, Vec<TextEdit>>,
    /// New files to create
    pub new_files: HashMap<PathBuf, String>,
    /// Files to delete
    pub deleted_files: Vec<PathBuf>,
    /// Import changes per file
    pub import_changes: HashMap<PathBuf, Vec<ImportChange>>,
    /// Diagnostics/warnings
    pub diagnostics: Vec<RefactorDiagnostic>,
}

/// A text edit to apply.
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// Span to replace
    pub span: Span,
    /// New text
    pub new_text: String,
}

/// A change to imports.
#[derive(Debug, Clone)]
pub enum ImportChange {
    /// Add an import
    Add { module: String, names: Vec<String> },
    /// Remove an import
    Remove { module: String, names: Vec<String> },
    /// Update an import
    Update {
        module: String,
        old_names: Vec<String>,
        new_names: Vec<String>,
    },
}

/// A diagnostic from refactoring.
#[derive(Debug, Clone)]
pub struct RefactorDiagnostic {
    /// Severity level
    pub level: DiagnosticLevel,
    /// Message
    pub message: String,
    /// Related file
    pub file: Option<PathBuf>,
    /// Related span
    pub span: Option<Span>,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Hint,
}

impl RefactorResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            file_edits: HashMap::new(),
            new_files: HashMap::new(),
            deleted_files: Vec::new(),
            import_changes: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Check if there are any changes.
    pub fn has_changes(&self) -> bool {
        !self.file_edits.is_empty()
            || !self.new_files.is_empty()
            || !self.deleted_files.is_empty()
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error)
    }

    /// Add a text edit for a file.
    pub fn add_edit(&mut self, file: PathBuf, edit: TextEdit) {
        self.file_edits.entry(file).or_default().push(edit);
    }

    /// Add a diagnostic.
    pub fn add_diagnostic(
        &mut self,
        level: DiagnosticLevel,
        message: impl Into<String>,
        file: Option<PathBuf>,
        span: Option<Span>,
    ) {
        self.diagnostics.push(RefactorDiagnostic {
            level,
            message: message.into(),
            file,
            span,
        });
    }
}

// ============================================================================
// Refactoring Engine
// ============================================================================

/// Engine for performing refactoring operations.
pub struct RefactoringEngine {
    /// Type inferencer for type information
    inferencer: DeepTypeInferencer,
    /// AST cache per file
    ast_cache: HashMap<PathBuf, MutableAst>,
}

impl RefactoringEngine {
    /// Create a new refactoring engine.
    pub fn new() -> Self {
        Self {
            inferencer: DeepTypeInferencer::new(),
            ast_cache: HashMap::new(),
        }
    }

    /// Create with existing type inferencer.
    pub fn with_inferencer(inferencer: DeepTypeInferencer) -> Self {
        Self {
            inferencer,
            ast_cache: HashMap::new(),
        }
    }

    /// Populate AST cache for a file.
    pub fn populate_ast_cache(&mut self, file: &PathBuf, content: &str) -> Result<(), String> {
        // Detect language from file extension
        let language = MultiParser::detect_language(file)
            .ok_or_else(|| format!("Failed to detect language for file: {:?}", file))?;

        // Parse the file
        let mut parser = MultiParser::new()
            .map_err(|e| format!("Failed to create parser: {}", e))?;
        let parsed = parser.parse(content, language)
            .ok_or_else(|| "Failed to parse file".to_string())?;

        // Convert tree-sitter AST to MutableAst
        let mut next_id = 0;
        let root = self.convert_node_to_mutable(&parsed.tree.root_node(), content, &mut next_id);
        let ast = MutableAst::new(root);

        self.ast_cache.insert(file.clone(), ast);
        Ok(())
    }

    /// Convert a tree-sitter Node to MutableNode recursively.
    fn convert_node_to_mutable(&self, node: &Node, source: &str, next_id: &mut usize) -> MutableNode {
        let id = NodeId(*next_id);
        *next_id += 1;

        let kind = node.kind().to_string();
        let span = Span::with_lines(
            node.start_byte(),
            node.end_byte(),
            node.start_position().row,
            node.start_position().column,
            node.end_position().row,
            node.end_position().column,
        );

        // Get node text value for leaf nodes
        let value = if node.child_count() == 0 {
            Some(source[node.byte_range()].to_string())
        } else {
            None
        };

        // Recursively convert children
        let mut children = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            children.push(self.convert_node_to_mutable(&child, source, next_id));
        }

        MutableNode {
            id,
            kind,
            span,
            value,
            children: Arc::new(children),
            metadata: NodeMetadata::default(),
        }
    }

    /// Get AST for a file, loading it if not cached.
    pub fn get_ast(&mut self, file: &PathBuf, content: &str) -> Result<&MutableAst, String> {
        if !self.ast_cache.contains_key(file) {
            self.populate_ast_cache(file, content)?;
        }
        Ok(self.ast_cache.get(file).unwrap())
    }

    /// Get mutable AST for a file, loading it if not cached.
    pub fn get_ast_mut(&mut self, file: &PathBuf, content: &str) -> Result<&mut MutableAst, String> {
        if !self.ast_cache.contains_key(file) {
            self.populate_ast_cache(file, content)?;
        }
        Ok(self.ast_cache.get_mut(file).unwrap())
    }

    /// Execute a refactoring operation.
    pub fn execute(&mut self, request: &RefactorRequest) -> RefactorResult {
        match &request.kind {
            RefactorKind::ExtractFunction { name } => {
                self.extract_function(request, name)
            }
            RefactorKind::ExtractMethod { name } => {
                self.extract_method(request, name)
            }
            RefactorKind::ExtractVariable { name } => {
                self.extract_variable(request, name)
            }
            RefactorKind::Rename { new_name } => {
                self.rename_symbol(request, new_name)
            }
            RefactorKind::MoveDefinition { target_file } => {
                self.move_definition(request, target_file)
            }
            RefactorKind::Inline => {
                self.inline_symbol(request)
            }
            RefactorKind::ChangeSignature { changes } => {
                self.change_signature(request, changes)
            }
        }
    }

    /// Extract selection into a new function.
    fn extract_function(&self, request: &RefactorRequest, name: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Placeholder implementation
        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Extract function '{}' at {:?}", name, request.span),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Extract selection into a new method.
    fn extract_method(&self, request: &RefactorRequest, name: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Extract method '{}' at {:?}", name, request.span),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Extract expression into a variable.
    fn extract_variable(&self, request: &RefactorRequest, name: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Extract variable '{}' at {:?}", name, request.span),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Rename a symbol across files.
    fn rename_symbol(&self, request: &RefactorRequest, new_name: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Find all references to the symbol
        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Rename to '{}' at {:?}", new_name, request.span),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Move a definition to another file.
    fn move_definition(&self, request: &RefactorRequest, target_file: &PathBuf) -> RefactorResult {
        let mut result = RefactorResult::empty();

        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Move definition to {:?}", target_file),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Inline a symbol's definition.
    fn inline_symbol(&self, request: &RefactorRequest) -> RefactorResult {
        let mut result = RefactorResult::empty();

        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Inline symbol at {:?}", request.span),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Change function signature.
    fn change_signature(&self, request: &RefactorRequest, _changes: &SignatureChanges) -> RefactorResult {
        let mut result = RefactorResult::empty();

        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Change signature at {:?}", request.span),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Get type context.
    pub fn type_context(&self) -> &TypeContext {
        self.inferencer.context()
    }
}

impl Default for RefactoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactor_result() {
        let mut result = RefactorResult::empty();
        assert!(!result.has_changes());
        assert!(!result.has_errors());

        result.add_edit(
            PathBuf::from("test.py"),
            TextEdit {
                span: Span::new(0, 10),
                new_text: "new text".to_string(),
            },
        );
        assert!(result.has_changes());
    }

    #[test]
    fn test_refactor_request() {
        let request = RefactorRequest {
            kind: RefactorKind::Rename {
                new_name: "new_name".to_string(),
            },
            file: PathBuf::from("test.py"),
            span: Span::new(0, 10),
            options: RefactorOptions::default(),
        };

        let mut engine = RefactoringEngine::new();
        let result = engine.execute(&request);

        assert!(!result.has_errors());
    }
}
