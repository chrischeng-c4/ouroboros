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
use super::semantic_search::{SemanticSearchEngine, SearchQuery, SearchKind, SearchScope};
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

/// Data flow analysis result.
struct DataFlow {
    /// Variables used but not defined in the selection
    external_vars: Vec<String>,
    /// Variables defined in the selection
    defined_vars: Vec<String>,
}

/// Engine for performing refactoring operations.
pub struct RefactoringEngine {
    /// Type inferencer for type information
    inferencer: DeepTypeInferencer,
    /// AST cache per file
    ast_cache: HashMap<PathBuf, MutableAst>,
    /// Semantic search engine for finding references
    search_engine: SemanticSearchEngine,
}

impl RefactoringEngine {
    /// Create a new refactoring engine.
    pub fn new() -> Self {
        Self {
            inferencer: DeepTypeInferencer::new(),
            ast_cache: HashMap::new(),
            search_engine: SemanticSearchEngine::new(),
        }
    }

    /// Create with existing type inferencer.
    pub fn with_inferencer(inferencer: DeepTypeInferencer) -> Self {
        Self {
            inferencer,
            ast_cache: HashMap::new(),
            search_engine: SemanticSearchEngine::new(),
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

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Analyze data flow for a code selection.
    fn analyze_data_flow(&self, _target: &MutableNode, _root: &MutableNode, _source: &str) -> DataFlow {
        // Simplified implementation - in reality would do full data flow analysis
        DataFlow {
            external_vars: Vec::new(),
            defined_vars: Vec::new(),
        }
    }

    /// Get indentation at a specific byte position.
    fn get_indent_at_position_static(source: &str, byte_pos: usize) -> String {
        // Find the line number containing this byte position
        let mut current_pos = 0;
        for line in source.lines() {
            let line_end = current_pos + line.len();

            // Check if byte_pos is in this line
            if byte_pos >= current_pos && byte_pos <= line_end {
                let indent_len = line.len() - line.trim_start().len();
                return " ".repeat(indent_len);
            }

            // +1 for newline character
            current_pos = line_end + 1;
        }

        String::new()
    }

    /// Find the statement containing an expression.
    fn find_containing_statement<'a>(
        &self,
        target: &'a MutableNode,
        root: &'a MutableNode,
    ) -> Option<&'a MutableNode> {
        // Walk up from target to find a statement node
        self.find_ancestor_of_kind(target, root, &[
            "expression_statement",
            "assignment",
            "return_statement",
            "if_statement",
            "for_statement",
            "while_statement",
        ])
    }

    /// Find ancestor node of specific kinds.
    fn find_ancestor_of_kind<'a>(
        &self,
        target: &'a MutableNode,
        root: &'a MutableNode,
        kinds: &[&str],
    ) -> Option<&'a MutableNode> {
        // Check if current node is one of the target kinds
        if kinds.contains(&target.kind.as_str()) {
            return Some(target);
        }

        // Recursively search parent nodes
        self.find_ancestor_helper(target, root, root, kinds)
    }

    /// Helper for finding ancestors.
    fn find_ancestor_helper<'a>(
        &self,
        target: &'a MutableNode,
        current: &'a MutableNode,
        root: &'a MutableNode,
        kinds: &[&str],
    ) -> Option<&'a MutableNode> {
        for child in current.children.iter() {
            if child.id == target.id {
                // Found the target, check if current is the right kind
                if kinds.contains(&current.kind.as_str()) {
                    return Some(current);
                }
                // Continue searching up
                return None;
            }

            // Recursively search in child
            if let Some(found) = self.find_ancestor_helper(target, child, root, kinds) {
                return Some(found);
            }
        }

        None
    }

    /// Find insertion point for a new function definition.
    fn find_function_insertion_point(&self, _root: &MutableNode, current_line: usize) -> usize {
        // Simplified: insert at beginning of file
        // In reality, would find the end of the current function or class
        _ = current_line;
        0
    }

    // ========================================================================
    // Public Methods
    // ========================================================================

    /// Execute a refactoring operation.
    pub fn execute(&mut self, request: &RefactorRequest, source: &str) -> RefactorResult {
        match &request.kind {
            RefactorKind::ExtractFunction { name } => {
                self.extract_function(request, name, source)
            }
            RefactorKind::ExtractMethod { name } => {
                self.extract_method(request, name, source)
            }
            RefactorKind::ExtractVariable { name } => {
                self.extract_variable(request, name, source)
            }
            RefactorKind::Rename { new_name } => {
                self.rename_symbol(request, new_name, source)
            }
            RefactorKind::MoveDefinition { target_file } => {
                self.move_definition(request, target_file, source)
            }
            RefactorKind::Inline => {
                self.inline_symbol(request, source)
            }
            RefactorKind::ChangeSignature { changes } => {
                self.change_signature(request, changes, source)
            }
        }
    }

    /// Extract selection into a new function.
    fn extract_function(&mut self, request: &RefactorRequest, name: &str, source: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Get or populate AST
        if let Err(e) = self.populate_ast_cache(&request.file, source) {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Failed to parse file: {}", e),
                Some(request.file.clone()),
                None,
            );
            return result;
        }

        // Extract the selected code
        let selected_code = &source[request.span.start..request.span.end];

        // Simplified: no parameters for now
        let params_str = String::new();

        let func_def = format!(
            "def {}({}):\n{}\n\n",
            name,
            params_str,
            selected_code.lines()
                .map(|line| format!("    {}", line))
                .collect::<Vec<_>>()
                .join("\n    ")
        );

        // Generate function call
        let call_str = format!("{}()", name);

        // Insert at beginning of file
        let insert_pos = 0;

        // Create edits
        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: Span::new(insert_pos, insert_pos),
                new_text: func_def,
            },
        );

        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: request.span,
                new_text: call_str,
            },
        );

        result
    }

    /// Extract selection into a new method.
    fn extract_method(&mut self, request: &RefactorRequest, name: &str, source: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Get or populate AST
        if let Err(e) = self.populate_ast_cache(&request.file, source) {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Failed to parse file: {}", e),
                Some(request.file.clone()),
                None,
            );
            return result;
        }

        // Extract the selected code
        let selected_code = &source[request.span.start..request.span.end];

        // Generate method definition with self parameter
        let method_def = format!(
            "    def {}(self):\n{}\n\n",
            name,
            selected_code.lines()
                .map(|line| format!("        {}", line))
                .collect::<Vec<_>>()
                .join("\n        ")
        );

        // Generate method call
        let call_str = format!("self.{}()", name);

        // Simplified: insert at beginning of file
        let insert_pos = 0;

        // Create edits
        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: Span::new(insert_pos, insert_pos),
                new_text: method_def,
            },
        );

        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: request.span,
                new_text: call_str,
            },
        );

        result
    }

    /// Extract expression into a variable.
    fn extract_variable(&mut self, request: &RefactorRequest, name: &str, source: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Get or populate AST
        if let Err(e) = self.populate_ast_cache(&request.file, source) {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Failed to parse file: {}", e),
                Some(request.file.clone()),
                None,
            );
            return result;
        }

        // Extract the expression text
        let expr_text = &source[request.span.start..request.span.end];

        // Get indentation at the current position
        let indent = Self::get_indent_at_position_static(source, request.span.start);

        // Generate variable assignment
        let assignment = format!("{}{} = {}\n", indent, name, expr_text);

        // Find line start position
        let line_start = Self::find_line_start(source, request.span.start);

        // Insert the assignment before the current line
        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: Span::new(line_start, line_start),
                new_text: assignment,
            },
        );

        // Replace the expression with the variable name
        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: request.span,
                new_text: name.to_string(),
            },
        );

        result
    }

    /// Find the start of a line containing a byte position.
    fn find_line_start(source: &str, pos: usize) -> usize {
        let mut line_start = 0;
        for (i, ch) in source.char_indices() {
            if i >= pos {
                break;
            }
            if ch == '\n' {
                line_start = i + 1;
            }
        }
        line_start
    }

    /// Rename a symbol across files.
    fn rename_symbol(&mut self, request: &RefactorRequest, new_name: &str, source: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Extract the old symbol name from the source
        let old_name = &source[request.span.start..request.span.end];

        // Validate new name
        if old_name == new_name {
            result.add_diagnostic(
                DiagnosticLevel::Info,
                "New name is the same as the old name",
                Some(request.file.clone()),
                Some(request.span),
            );
            return result;
        }

        if new_name.is_empty() {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                "New name cannot be empty",
                Some(request.file.clone()),
                Some(request.span),
            );
            return result;
        }

        // Basic name validation (simplified - just check it's a valid identifier)
        if !new_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                "New name must be a valid identifier",
                Some(request.file.clone()),
                Some(request.span),
            );
            return result;
        }

        // For simplified implementation: just rename in current file
        // In a full implementation, would use semantic search to find all references
        let query = SearchQuery {
            kind: SearchKind::Usages {
                symbol: old_name.to_string(),
                file: request.file.clone(),
            },
            scope: SearchScope::Project,
            max_results: 1000,
        };

        let search_result = self.search_engine.search(&query);

        // If no matches found in index, do simple text-based search in current file
        if search_result.matches.is_empty() {
            // Simple approach: find all occurrences of the old name in the current file
            let mut pos = 0;
            while let Some(found_pos) = source[pos..].find(old_name) {
                let absolute_pos = pos + found_pos;

                // Create a text edit for this occurrence
                result.add_edit(
                    request.file.clone(),
                    TextEdit {
                        span: Span::new(absolute_pos, absolute_pos + old_name.len()),
                        new_text: new_name.to_string(),
                    },
                );

                pos = absolute_pos + old_name.len();
            }
        } else {
            // Use search results
            for search_match in &search_result.matches {
                result.add_edit(
                    search_match.file.clone(),
                    TextEdit {
                        span: search_match.span,
                        new_text: new_name.to_string(),
                    },
                );
            }
        }

        if !result.has_changes() {
            result.add_diagnostic(
                DiagnosticLevel::Warning,
                format!("No occurrences of '{}' found", old_name),
                Some(request.file.clone()),
                Some(request.span),
            );
        } else {
            result.add_diagnostic(
                DiagnosticLevel::Info,
                format!("Renamed '{}' to '{}' ({} occurrences)", old_name, new_name, result.file_edits.values().map(|v| v.len()).sum::<usize>()),
                Some(request.file.clone()),
                Some(request.span),
            );
        }

        result
    }

    /// Move a definition to another file.
    fn move_definition(&mut self, request: &RefactorRequest, target_file: &PathBuf, source: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Get or populate AST
        if let Err(e) = self.populate_ast_cache(&request.file, source) {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Failed to parse file: {}", e),
                Some(request.file.clone()),
                None,
            );
            return result;
        }

        // Extract the definition code
        let definition_code = &source[request.span.start..request.span.end];

        // Remove from current file
        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: request.span,
                new_text: String::new(),
            },
        );

        // Add to target file (at beginning for simplicity)
        result.new_files.insert(
            target_file.clone(),
            format!("{}\n\n", definition_code),
        );

        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Moved definition to {:?}", target_file),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Inline a symbol's definition.
    fn inline_symbol(&mut self, request: &RefactorRequest, source: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Get or populate AST
        if let Err(e) = self.populate_ast_cache(&request.file, source) {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Failed to parse file: {}", e),
                Some(request.file.clone()),
                None,
            );
            return result;
        }

        // Extract the symbol name to inline
        let symbol_name = &source[request.span.start..request.span.end];

        // Simplified implementation: Find the definition and replace all usages
        // Look for pattern: "symbol_name = expression"
        let mut definition_value = None;
        let mut definition_span = None;

        // Simple pattern matching for variable assignment
        if let Some(def_pos) = source.find(&format!("{} = ", symbol_name)) {
            let start = def_pos;
            // Find the end of the line (or end of file if no newline)
            let rest = &source[start..];
            let newline_pos = rest.find('\n').unwrap_or(rest.len());
            let end = start + newline_pos;
            let line = &source[start..end];

            // Extract the value part (after "symbol_name = ")
            if let Some(eq_pos) = line.find(" = ") {
                let value_start = start + eq_pos + 3;
                definition_value = Some(source[value_start..end].trim().to_string());
                // Include newline only if it exists
                let span_end = if newline_pos < rest.len() { end + 1 } else { end };
                definition_span = Some(Span::new(start, span_end));
            }
        }

        if definition_value.is_none() {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Could not find definition for '{}'", symbol_name),
                Some(request.file.clone()),
                Some(request.span),
            );
            return result;
        }

        let def_value = definition_value.unwrap();
        let def_span = definition_span.unwrap();

        // Find all usages of the symbol (excluding the definition)
        let mut usages = Vec::new();
        let mut pos = 0;
        while let Some(found_pos) = source[pos..].find(symbol_name) {
            let absolute_pos = pos + found_pos;

            // Skip if this is the definition itself
            if absolute_pos >= def_span.start && absolute_pos < def_span.end {
                pos = absolute_pos + symbol_name.len();
                continue;
            }

            // Check if it's a complete identifier (not part of another word)
            let before_ok = absolute_pos == 0 || !source.chars().nth(absolute_pos - 1).unwrap_or(' ').is_alphanumeric();
            let after_pos = absolute_pos + symbol_name.len();
            let after_ok = after_pos >= source.len() || !source.chars().nth(after_pos).unwrap_or(' ').is_alphanumeric();

            if before_ok && after_ok {
                usages.push(Span::new(absolute_pos, absolute_pos + symbol_name.len()));
            }

            pos = absolute_pos + symbol_name.len();
        }

        if usages.is_empty() {
            result.add_diagnostic(
                DiagnosticLevel::Warning,
                format!("No usages of '{}' found to inline", symbol_name),
                Some(request.file.clone()),
                Some(request.span),
            );
            return result;
        }

        // Store count before iterating
        let usage_count = usages.len();

        // Replace all usages with the definition value
        for usage_span in usages {
            result.add_edit(
                request.file.clone(),
                TextEdit {
                    span: usage_span,
                    new_text: def_value.clone(),
                },
            );
        }

        // Remove the definition line
        result.add_edit(
            request.file.clone(),
            TextEdit {
                span: def_span,
                new_text: String::new(),
            },
        );

        result.add_diagnostic(
            DiagnosticLevel::Info,
            format!("Inlined '{}' ({} usages)", symbol_name, usage_count),
            Some(request.file.clone()),
            Some(request.span),
        );

        result
    }

    /// Change function signature.
    fn change_signature(&mut self, request: &RefactorRequest, changes: &SignatureChanges, source: &str) -> RefactorResult {
        let mut result = RefactorResult::empty();

        // Get or populate AST
        if let Err(e) = self.populate_ast_cache(&request.file, source) {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                format!("Failed to parse file: {}", e),
                Some(request.file.clone()),
                None,
            );
            return result;
        }

        // Simplified implementation: Find function definition and update parameters
        let function_code = &source[request.span.start..request.span.end];

        // Find the opening parenthesis
        if let Some(open_paren) = function_code.find('(') {
            if let Some(close_paren) = function_code.find(')') {
                let func_name_part = &function_code[..open_paren];
                let after_params = &function_code[close_paren + 1..];

                // Build new parameters list
                let mut new_params = Vec::new();

                // Add new parameters
                for (param_name, type_ann, default) in &changes.new_params {
                    let param_str = if let Some(type_str) = type_ann {
                        if let Some(default_val) = default {
                            format!("{}: {} = {}", param_name, type_str, default_val)
                        } else {
                            format!("{}: {}", param_name, type_str)
                        }
                    } else if let Some(default_val) = default {
                        format!("{}={}", param_name, default_val)
                    } else {
                        param_name.clone()
                    };
                    new_params.push(param_str);
                }

                let new_params_str = new_params.join(", ");

                // Construct new function signature
                let new_signature = format!("{}({}){}", func_name_part, new_params_str, after_params);

                // Replace the function signature
                result.add_edit(
                    request.file.clone(),
                    TextEdit {
                        span: request.span,
                        new_text: new_signature,
                    },
                );

                result.add_diagnostic(
                    DiagnosticLevel::Info,
                    format!("Changed function signature ({} parameters)", new_params.len()),
                    Some(request.file.clone()),
                    Some(request.span),
                );
            } else {
                result.add_diagnostic(
                    DiagnosticLevel::Error,
                    "Could not find closing parenthesis",
                    Some(request.file.clone()),
                    Some(request.span),
                );
            }
        } else {
            result.add_diagnostic(
                DiagnosticLevel::Error,
                "Could not find function parameters",
                Some(request.file.clone()),
                Some(request.span),
            );
        }

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
        let source = "old_name = 42";
        let request = RefactorRequest {
            kind: RefactorKind::Rename {
                new_name: "new_name".to_string(),
            },
            file: PathBuf::from("test.py"),
            span: Span::new(0, 8),
            options: RefactorOptions::default(),
        };

        let mut engine = RefactoringEngine::new();
        let result = engine.execute(&request, source);

        assert!(!result.has_errors());
    }
}
