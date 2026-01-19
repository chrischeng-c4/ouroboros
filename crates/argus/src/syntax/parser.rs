//! Multi-language parser using tree-sitter

use std::path::Path;
use tree_sitter::{Parser, Tree};

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    TypeScript,
    Rust,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
        }
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Python => &["py", "pyi"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Rust => &["rs"],
        }
    }
}

/// Multi-language parser
pub struct MultiParser {
    python_parser: Parser,
    typescript_parser: Parser,
    rust_parser: Parser,
}

impl MultiParser {
    pub fn new() -> Result<Self, String> {
        let mut python_parser = Parser::new();
        python_parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| format!("Failed to load Python grammar: {}", e))?;

        let mut typescript_parser = Parser::new();
        typescript_parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|e| format!("Failed to load TypeScript grammar: {}", e))?;

        let mut rust_parser = Parser::new();
        rust_parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| format!("Failed to load Rust grammar: {}", e))?;

        Ok(Self {
            python_parser,
            typescript_parser,
            rust_parser,
        })
    }

    /// Detect language from file extension
    pub fn detect_language(path: &Path) -> Option<Language> {
        let ext = path.extension()?.to_str()?;
        match ext {
            "py" | "pyi" => Some(Language::Python),
            "ts" | "tsx" => Some(Language::TypeScript),
            "rs" => Some(Language::Rust),
            _ => None,
        }
    }

    /// Parse source code
    pub fn parse(&mut self, source: &str, language: Language) -> Option<ParsedFile> {
        let parser = match language {
            Language::Python => &mut self.python_parser,
            Language::TypeScript => &mut self.typescript_parser,
            Language::Rust => &mut self.rust_parser,
        };

        let tree = parser.parse(source, None)?;
        let has_errors = tree.root_node().has_error();

        Some(ParsedFile {
            source: source.to_string(),
            tree,
            language,
            has_errors,
        })
    }
}

/// Information about a parse error
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Start byte offset of the error
    pub start_byte: usize,
    /// End byte offset of the error
    pub end_byte: usize,
    /// Start position (line, column)
    pub start_position: (usize, usize),
    /// End position (line, column)
    pub end_position: (usize, usize),
    /// The error node kind (usually "ERROR")
    pub kind: String,
}

/// Parsed file with AST
pub struct ParsedFile {
    pub source: String,
    pub tree: Tree,
    pub language: Language,
    pub has_errors: bool,
}

impl ParsedFile {
    /// Get the root node
    pub fn root_node(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }

    /// Get source text for a node
    pub fn node_text(&self, node: &tree_sitter::Node<'_>) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Walk the AST with a visitor function
    /// Returns true to continue traversal, false to stop
    pub fn walk<F>(&self, mut visitor: F)
    where
        F: FnMut(&tree_sitter::Node<'_>, usize) -> bool,
    {
        Self::walk_recursive(&self.root_node(), 0, &mut visitor);
    }

    fn walk_recursive<F>(node: &tree_sitter::Node<'_>, depth: usize, visitor: &mut F)
    where
        F: FnMut(&tree_sitter::Node<'_>, usize) -> bool,
    {
        if !visitor(node, depth) {
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk_recursive(&child, depth + 1, visitor);
        }
    }

    /// Collect all parse errors from the tree
    ///
    /// Returns a list of ParseError for each ERROR node found in the AST.
    pub fn collect_errors(&self) -> Vec<ParseError> {
        let mut errors = Vec::new();
        self.walk(|node, _depth| {
            if node.is_error() || node.is_missing() {
                errors.push(ParseError {
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    start_position: (node.start_position().row + 1, node.start_position().column + 1),
                    end_position: (node.end_position().row + 1, node.end_position().column + 1),
                    kind: node.kind().to_string(),
                });
            }
            true // Continue traversal
        });
        errors
    }

    /// Walk the AST with error recovery, skipping ERROR nodes
    ///
    /// The visitor function receives non-error nodes only.
    /// When an ERROR node is encountered, its children are skipped and
    /// traversal continues to the next sibling.
    pub fn walk_with_recovery<F>(&self, mut visitor: F)
    where
        F: FnMut(&tree_sitter::Node<'_>, usize) -> bool,
    {
        Self::walk_with_recovery_recursive(&self.root_node(), 0, &mut visitor);
    }

    fn walk_with_recovery_recursive<F>(node: &tree_sitter::Node<'_>, depth: usize, visitor: &mut F)
    where
        F: FnMut(&tree_sitter::Node<'_>, usize) -> bool,
    {
        // Skip ERROR nodes and their children
        if node.is_error() || node.is_missing() {
            return;
        }

        if !visitor(node, depth) {
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk_with_recovery_recursive(&child, depth + 1, visitor);
        }
    }

    /// Get valid (non-error) top-level statements
    ///
    /// This is useful for analyzing a file with syntax errors - we can
    /// still process the valid parts.
    pub fn valid_statements(&self) -> Vec<tree_sitter::Node<'_>> {
        let root = self.root_node();
        let mut cursor = root.walk();
        let mut valid = Vec::new();

        for child in root.children(&mut cursor) {
            if !child.is_error() && !child.is_missing() {
                valid.push(child);
            }
        }

        valid
    }

    /// Check if a node is inside an error region
    pub fn is_inside_error(&self, node: &tree_sitter::Node<'_>) -> bool {
        let mut current = *node;
        while let Some(parent) = current.parent() {
            if parent.is_error() {
                return true;
            }
            current = parent;
        }
        false
    }

    /// Get the next valid sibling after an error node
    ///
    /// This implements "synchronization" - finding where to resume
    /// analysis after encountering an error.
    pub fn synchronize_after<'a>(node: &tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        let mut current = node.next_sibling();
        while let Some(sibling) = current {
            if !sibling.is_error() && !sibling.is_missing() {
                return Some(sibling);
            }
            current = sibling.next_sibling();
        }
        None
    }
}
