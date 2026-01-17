//! Unified Symbol Table for cross-language semantic analysis
//!
//! Provides a common symbol representation for Python, TypeScript, and Rust.

use crate::diagnostic::Range;
use crate::syntax::{Language, ParsedFile};
use std::collections::HashMap;

/// Unique identifier for a symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

/// Kind of symbol (cross-language)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    // Common
    Variable,
    Function,
    Class,
    Parameter,
    Import,
    Module,

    // Python-specific
    TypeAlias,
    Decorator,

    // TypeScript-specific
    Interface,
    TypeParameter,
    Enum,
    EnumMember,

    // Rust-specific
    Struct,
    Trait,
    Impl,
    Macro,
    Const,
    Static,
}

impl SymbolKind {
    /// Get LSP symbol kind for hover display
    pub fn display_name(&self) -> &'static str {
        match self {
            SymbolKind::Variable => "variable",
            SymbolKind::Function => "function",
            SymbolKind::Class => "class",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Import => "import",
            SymbolKind::Module => "module",
            SymbolKind::TypeAlias => "type alias",
            SymbolKind::Decorator => "decorator",
            SymbolKind::Interface => "interface",
            SymbolKind::TypeParameter => "type parameter",
            SymbolKind::Enum => "enum",
            SymbolKind::EnumMember => "enum member",
            SymbolKind::Struct => "struct",
            SymbolKind::Trait => "trait",
            SymbolKind::Impl => "impl",
            SymbolKind::Macro => "macro",
            SymbolKind::Const => "const",
            SymbolKind::Static => "static",
        }
    }
}

/// Type information (basic)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeInfo {
    /// Primitive types (int, str, bool, etc.)
    Primitive(String),
    /// List/Array type
    List(Box<TypeInfo>),
    /// Dict/Map type
    Dict(Box<TypeInfo>, Box<TypeInfo>),
    /// Optional type
    Optional(Box<TypeInfo>),
    /// Union type
    Union(Vec<TypeInfo>),
    /// Callable/Function type
    Callable {
        params: Vec<TypeInfo>,
        ret: Box<TypeInfo>,
    },
    /// Named type (class, interface, etc.)
    Named(String),
    /// Generic type with parameters
    Generic(String, Vec<TypeInfo>),
    /// Unknown type
    Unknown,
    /// Any type
    Any,
}

impl TypeInfo {
    /// Format type for display
    pub fn display(&self) -> String {
        match self {
            TypeInfo::Primitive(name) => name.clone(),
            TypeInfo::List(inner) => format!("list[{}]", inner.display()),
            TypeInfo::Dict(key, value) => format!("dict[{}, {}]", key.display(), value.display()),
            TypeInfo::Optional(inner) => format!("{}?", inner.display()),
            TypeInfo::Union(types) => {
                types.iter().map(|t| t.display()).collect::<Vec<_>>().join(" | ")
            }
            TypeInfo::Callable { params, ret } => {
                let params_str = params.iter().map(|t| t.display()).collect::<Vec<_>>().join(", ");
                format!("({}) -> {}", params_str, ret.display())
            }
            TypeInfo::Named(name) => name.clone(),
            TypeInfo::Generic(name, args) => {
                let args_str = args.iter().map(|t| t.display()).collect::<Vec<_>>().join(", ");
                format!("{}[{}]", name, args_str)
            }
            TypeInfo::Unknown => "unknown".to_string(),
            TypeInfo::Any => "any".to_string(),
        }
    }

    /// Parse from Python type annotation string
    pub fn from_python_annotation(annotation: &str) -> Self {
        let annotation = annotation.trim();

        // Handle Optional
        if annotation.starts_with("Optional[") && annotation.ends_with(']') {
            let inner = &annotation[9..annotation.len() - 1];
            return TypeInfo::Optional(Box::new(Self::from_python_annotation(inner)));
        }

        // Handle List
        if annotation.starts_with("List[") && annotation.ends_with(']') {
            let inner = &annotation[5..annotation.len() - 1];
            return TypeInfo::List(Box::new(Self::from_python_annotation(inner)));
        }
        if annotation.starts_with("list[") && annotation.ends_with(']') {
            let inner = &annotation[5..annotation.len() - 1];
            return TypeInfo::List(Box::new(Self::from_python_annotation(inner)));
        }

        // Handle Dict
        if (annotation.starts_with("Dict[") || annotation.starts_with("dict[")) && annotation.ends_with(']') {
            let inner = &annotation[5..annotation.len() - 1];
            if let Some((key, value)) = inner.split_once(',') {
                return TypeInfo::Dict(
                    Box::new(Self::from_python_annotation(key.trim())),
                    Box::new(Self::from_python_annotation(value.trim())),
                );
            }
        }

        // Handle Union with |
        if annotation.contains(" | ") {
            let types: Vec<_> = annotation
                .split(" | ")
                .map(|t| Self::from_python_annotation(t.trim()))
                .collect();
            return TypeInfo::Union(types);
        }

        // Handle primitives
        match annotation {
            "int" => TypeInfo::Primitive("int".to_string()),
            "str" => TypeInfo::Primitive("str".to_string()),
            "bool" => TypeInfo::Primitive("bool".to_string()),
            "float" => TypeInfo::Primitive("float".to_string()),
            "None" => TypeInfo::Primitive("None".to_string()),
            "Any" => TypeInfo::Any,
            _ => TypeInfo::Named(annotation.to_string()),
        }
    }
}

/// A symbol in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub location: Range,
    pub type_info: Option<TypeInfo>,
    pub doc: Option<String>,
    pub scope_id: usize,
}

impl Symbol {
    /// Generate hover content for this symbol
    pub fn hover_content(&self, language: Language) -> String {
        let mut content = String::new();

        // Add code block with symbol signature
        let lang_str = match language {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
        };

        content.push_str(&format!("```{}\n", lang_str));

        match self.kind {
            SymbolKind::Function => {
                if let Some(ref type_info) = self.type_info {
                    content.push_str(&format!("def {}(...) -> {}\n", self.name, type_info.display()));
                } else {
                    content.push_str(&format!("def {}(...)\n", self.name));
                }
            }
            SymbolKind::Class | SymbolKind::Struct => {
                content.push_str(&format!("class {}\n", self.name));
            }
            SymbolKind::Variable | SymbolKind::Parameter => {
                if let Some(ref type_info) = self.type_info {
                    content.push_str(&format!("{}: {}\n", self.name, type_info.display()));
                } else {
                    content.push_str(&format!("{}\n", self.name));
                }
            }
            _ => {
                content.push_str(&format!("{} {}\n", self.kind.display_name(), self.name));
            }
        }

        content.push_str("```\n");

        // Add documentation if available
        if let Some(ref doc) = self.doc {
            content.push_str("\n---\n\n");
            content.push_str(doc);
        }

        content
    }
}

/// Reference to a symbol
#[derive(Debug, Clone)]
pub struct SymbolReference {
    pub symbol_id: SymbolId,
    pub location: Range,
    pub is_definition: bool,
}

/// Symbol table for a file
#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
    by_name: HashMap<String, Vec<SymbolId>>,
    references: Vec<SymbolReference>,
    next_id: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a symbol to the table
    pub fn add_symbol(
        &mut self,
        name: String,
        kind: SymbolKind,
        location: Range,
        type_info: Option<TypeInfo>,
        doc: Option<String>,
        scope_id: usize,
    ) -> SymbolId {
        let id = SymbolId(self.next_id);
        self.next_id += 1;

        let symbol = Symbol {
            id,
            name: name.clone(),
            kind,
            location: location.clone(),
            type_info,
            doc,
            scope_id,
        };

        self.symbols.push(symbol);
        self.by_name.entry(name).or_default().push(id);

        // Add definition reference
        self.references.push(SymbolReference {
            symbol_id: id,
            location,
            is_definition: true,
        });

        id
    }

    /// Add a reference to a symbol
    pub fn add_reference(&mut self, symbol_id: SymbolId, location: Range) {
        self.references.push(SymbolReference {
            symbol_id,
            location,
            is_definition: false,
        });
    }

    /// Get symbol by ID
    pub fn get(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.0)
    }

    /// Find symbols by name
    pub fn find_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.by_name
            .get(name)
            .map(|ids| ids.iter().filter_map(|id| self.get(*id)).collect())
            .unwrap_or_default()
    }

    /// Find symbol at position
    pub fn find_at_position(&self, line: u32, character: u32) -> Option<&Symbol> {
        // First check references (more precise)
        for reference in &self.references {
            if reference.location.contains(line, character) {
                return self.get(reference.symbol_id);
            }
        }

        // Then check symbol definitions
        for symbol in &self.symbols {
            if symbol.location.contains(line, character) {
                return Some(symbol);
            }
        }

        None
    }

    /// Find definition of symbol at position
    pub fn find_definition_at(&self, line: u32, character: u32) -> Option<&Symbol> {
        // Find what's at position
        for reference in &self.references {
            if reference.location.contains(line, character) {
                return self.get(reference.symbol_id);
            }
        }
        None
    }

    /// Find all references to symbol at position
    pub fn find_references_at(&self, line: u32, character: u32, include_definition: bool) -> Vec<Range> {
        // Find the symbol at position
        let symbol_id = self.references
            .iter()
            .find(|r| r.location.contains(line, character))
            .map(|r| r.symbol_id);

        let Some(id) = symbol_id else {
            return Vec::new();
        };

        // Find all references to this symbol
        self.references
            .iter()
            .filter(|r| r.symbol_id == id && (include_definition || !r.is_definition))
            .map(|r| r.location.clone())
            .collect()
    }

    /// Get all symbols
    pub fn all_symbols(&self) -> &[Symbol] {
        &self.symbols
    }
}

/// Build symbol table from parsed file
pub struct SymbolTableBuilder {
    table: SymbolTable,
    current_scope: usize,
    scope_stack: Vec<usize>,
    next_scope: usize,
}

impl SymbolTableBuilder {
    pub fn new() -> Self {
        Self {
            table: SymbolTable::new(),
            current_scope: 0,
            scope_stack: vec![0],
            next_scope: 1,
        }
    }

    /// Build symbol table for a Python file
    pub fn build_python(mut self, file: &ParsedFile) -> SymbolTable {
        self.visit_python_node(&file.root_node(), file);
        self.table
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(self.current_scope);
        self.current_scope = self.next_scope;
        self.next_scope += 1;
    }

    fn pop_scope(&mut self) {
        if let Some(parent) = self.scope_stack.pop() {
            self.current_scope = parent;
        }
    }

    fn visit_python_node(&mut self, node: &tree_sitter::Node<'_>, file: &ParsedFile) {
        match node.kind() {
            "function_definition" | "async_function_definition" => {
                self.visit_python_function(node, file);
                return;
            }
            "class_definition" => {
                self.visit_python_class(node, file);
                return;
            }
            "assignment" => {
                self.visit_python_assignment(node, file);
            }
            "identifier" => {
                // This is a reference to a symbol
                let name = file.node_text(node);
                if let Some(symbols) = self.table.by_name.get(name) {
                    if let Some(&id) = symbols.last() {
                        self.table.add_reference(id, Range::from_node(node));
                    }
                }
                return;
            }
            _ => {}
        }

        // Visit children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_python_node(&child, file);
        }
    }

    fn visit_python_function(&mut self, node: &tree_sitter::Node<'_>, file: &ParsedFile) {
        // Get function name
        let name_node = node.child_by_field_name("name");
        let name = name_node.map(|n| file.node_text(&n).to_string()).unwrap_or_default();
        let location = name_node.map(|n| Range::from_node(&n)).unwrap_or_default();

        // Get return type annotation
        let return_type = node
            .child_by_field_name("return_type")
            .map(|n| {
                let type_str = file.node_text(&n);
                TypeInfo::from_python_annotation(type_str)
            });

        // Get docstring
        let doc = self.extract_python_docstring(node, file);

        // Add function symbol
        self.table.add_symbol(
            name,
            SymbolKind::Function,
            location,
            return_type,
            doc,
            self.current_scope,
        );

        // Enter function scope
        self.push_scope();

        // Process parameters
        if let Some(params) = node.child_by_field_name("parameters") {
            self.visit_python_parameters(&params, file);
        }

        // Process body
        if let Some(body) = node.child_by_field_name("body") {
            self.visit_python_node(&body, file);
        }

        self.pop_scope();
    }

    fn visit_python_class(&mut self, node: &tree_sitter::Node<'_>, file: &ParsedFile) {
        let name_node = node.child_by_field_name("name");
        let name = name_node.map(|n| file.node_text(&n).to_string()).unwrap_or_default();
        let location = name_node.map(|n| Range::from_node(&n)).unwrap_or_default();

        let doc = self.extract_python_docstring(node, file);

        self.table.add_symbol(
            name,
            SymbolKind::Class,
            location,
            None,
            doc,
            self.current_scope,
        );

        // Enter class scope
        self.push_scope();

        if let Some(body) = node.child_by_field_name("body") {
            self.visit_python_node(&body, file);
        }

        self.pop_scope();
    }

    fn visit_python_assignment(&mut self, node: &tree_sitter::Node<'_>, file: &ParsedFile) {
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "identifier" {
                let name = file.node_text(&left).to_string();
                let location = Range::from_node(&left);

                // Try to get type annotation
                let type_info = node
                    .child_by_field_name("type")
                    .map(|n| TypeInfo::from_python_annotation(file.node_text(&n)));

                self.table.add_symbol(
                    name,
                    SymbolKind::Variable,
                    location,
                    type_info,
                    None,
                    self.current_scope,
                );
            }
        }

        // Visit right side for references
        if let Some(right) = node.child_by_field_name("right") {
            self.visit_python_node(&right, file);
        }
    }

    fn visit_python_parameters(&mut self, params: &tree_sitter::Node<'_>, file: &ParsedFile) {
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    let name = file.node_text(&child).to_string();
                    self.table.add_symbol(
                        name,
                        SymbolKind::Parameter,
                        Range::from_node(&child),
                        None,
                        None,
                        self.current_scope,
                    );
                }
                "typed_parameter" | "typed_default_parameter" | "default_parameter" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = file.node_text(&name_node).to_string();
                        let type_info = child
                            .child_by_field_name("type")
                            .map(|n| TypeInfo::from_python_annotation(file.node_text(&n)));

                        self.table.add_symbol(
                            name,
                            SymbolKind::Parameter,
                            Range::from_node(&name_node),
                            type_info,
                            None,
                            self.current_scope,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn extract_python_docstring(&self, node: &tree_sitter::Node<'_>, file: &ParsedFile) -> Option<String> {
        let body = node.child_by_field_name("body")?;
        let mut cursor = body.walk();
        let first_child = body.children(&mut cursor).next()?;

        if first_child.kind() == "expression_statement" {
            if let Some(expr) = first_child.child(0) {
                if expr.kind() == "string" {
                    let text = file.node_text(&expr);
                    // Strip quotes
                    let doc = text
                        .trim_start_matches("\"\"\"")
                        .trim_start_matches("'''")
                        .trim_end_matches("\"\"\"")
                        .trim_end_matches("'''")
                        .trim();
                    return Some(doc.to_string());
                }
            }
        }
        None
    }
}

impl Default for SymbolTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_info_display() {
        assert_eq!(TypeInfo::Primitive("int".to_string()).display(), "int");
        assert_eq!(
            TypeInfo::List(Box::new(TypeInfo::Primitive("str".to_string()))).display(),
            "list[str]"
        );
        assert_eq!(
            TypeInfo::Optional(Box::new(TypeInfo::Primitive("int".to_string()))).display(),
            "int?"
        );
    }

    #[test]
    fn test_type_info_from_annotation() {
        assert_eq!(
            TypeInfo::from_python_annotation("int"),
            TypeInfo::Primitive("int".to_string())
        );
        assert_eq!(
            TypeInfo::from_python_annotation("List[str]"),
            TypeInfo::List(Box::new(TypeInfo::Primitive("str".to_string())))
        );
        assert_eq!(
            TypeInfo::from_python_annotation("Optional[int]"),
            TypeInfo::Optional(Box::new(TypeInfo::Primitive("int".to_string())))
        );
    }
}
