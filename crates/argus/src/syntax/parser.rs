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
}
