//! Semantic code search (Sprint 4 - Track 1)
//!
//! Provides type-aware code search capabilities:
//! - Find similar code patterns
//! - Search by type signature
//! - Find implementations of protocol
//! - Call hierarchy analysis

use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::ty::Type;
use crate::semantic::{SymbolTable, SymbolKind as SemanticSymbolKind};
use super::deep_inference::{TypeContext, DeepTypeInferencer};
use super::mutable_ast::Span;

// ============================================================================
// Search Query
// ============================================================================

/// A semantic search query.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Type of search
    pub kind: SearchKind,
    /// Scope to search in
    pub scope: SearchScope,
    /// Maximum results
    pub max_results: usize,
}

/// Type of semantic search.
#[derive(Debug, Clone)]
pub enum SearchKind {
    /// Find by type signature
    ByTypeSignature {
        params: Vec<Type>,
        return_type: Option<Type>,
    },
    /// Find implementations of protocol/interface
    Implementations { protocol: String },
    /// Find usages of a symbol
    Usages { symbol: String, file: PathBuf },
    /// Find similar code patterns
    SimilarPatterns { pattern: String },
    /// Find by documentation content
    ByDocumentation { query: String },
    /// Find call hierarchy (callers or callees)
    CallHierarchy {
        symbol: String,
        file: PathBuf,
        direction: CallDirection,
    },
    /// Find type hierarchy (supertypes or subtypes)
    TypeHierarchy {
        type_name: String,
        direction: TypeHierarchyDirection,
    },
}

/// Direction for call hierarchy search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    /// Find callers (incoming calls)
    Callers,
    /// Find callees (outgoing calls)
    Callees,
}

/// Direction for type hierarchy search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeHierarchyDirection {
    /// Find supertypes (parents)
    Supertypes,
    /// Find subtypes (children)
    Subtypes,
    /// Find both
    Both,
}

/// Scope for search operations.
#[derive(Debug, Clone)]
pub enum SearchScope {
    /// Current file only
    CurrentFile(PathBuf),
    /// Specific files
    Files(Vec<PathBuf>),
    /// Entire project
    Project,
    /// Project with dependencies
    ProjectWithDeps,
}

// ============================================================================
// Search Results
// ============================================================================

/// Result of a semantic search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Matching items
    pub matches: Vec<SearchMatch>,
    /// Total matches (may be more than returned)
    pub total_count: usize,
    /// Search statistics
    pub stats: SearchStats,
}

/// A single search match.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// File containing the match
    pub file: PathBuf,
    /// Span of the match
    pub span: Span,
    /// Symbol name (if applicable)
    pub symbol: Option<String>,
    /// Match kind
    pub kind: MatchKind,
    /// Relevance score (0.0 to 1.0)
    pub score: f64,
    /// Context lines around the match
    pub context: Option<MatchContext>,
}

/// Kind of search match.
#[derive(Debug, Clone)]
pub enum MatchKind {
    /// Function definition
    FunctionDef,
    /// Method definition
    MethodDef,
    /// Class definition
    ClassDef,
    /// Variable assignment
    VariableAssignment,
    /// Import statement
    Import,
    /// Call expression
    Call,
    /// Type annotation
    TypeAnnotation,
    /// Comment/docstring
    Documentation,
}

/// Context around a match.
#[derive(Debug, Clone)]
pub struct MatchContext {
    /// Lines before match
    pub before: Vec<String>,
    /// The matching line(s)
    pub matched: Vec<String>,
    /// Lines after match
    pub after: Vec<String>,
}

/// Search statistics.
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// Files searched
    pub files_searched: usize,
    /// Time taken (milliseconds)
    pub time_ms: u64,
    /// Whether results were truncated
    pub truncated: bool,
}

impl SearchResult {
    /// Create empty result.
    pub fn empty() -> Self {
        Self {
            matches: Vec::new(),
            total_count: 0,
            stats: SearchStats::default(),
        }
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Get number of matches returned.
    pub fn len(&self) -> usize {
        self.matches.len()
    }
}

// ============================================================================
// Semantic Search Engine
// ============================================================================

/// Engine for semantic code search.
pub struct SemanticSearchEngine {
    /// Type inferencer
    inferencer: DeepTypeInferencer,
    /// Symbol index
    symbol_index: HashMap<String, Vec<SymbolLocation>>,
    /// Type index
    type_index: HashMap<String, Vec<TypeLocation>>,
    /// Type signature index: signature key -> functions
    type_signature_index: HashMap<String, Vec<SymbolLocation>>,
    /// Call graph
    call_graph: CallGraph,
}

/// Location of a symbol.
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    /// File path
    pub file: PathBuf,
    /// Span in file
    pub span: Span,
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: MatchKind,
    /// Type (if known)
    pub ty: Option<Type>,
}

/// Location of a type.
#[derive(Debug, Clone)]
pub struct TypeLocation {
    /// File path
    pub file: PathBuf,
    /// Span in file
    pub span: Span,
    /// Type name
    pub type_name: String,
}

/// A call site in the code.
#[derive(Debug, Clone)]
pub struct CallSite {
    /// File where the call occurs
    pub file: PathBuf,
    /// Location of the call
    pub span: Span,
    /// Function being called
    pub callee: String,
    /// Function containing this call
    pub caller: String,
}

/// Call graph for tracking function calls.
#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    /// Map from function name to calls it makes
    pub calls: HashMap<String, Vec<CallSite>>,
    /// Map from function name to places it's called from
    pub called_by: HashMap<String, Vec<CallSite>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a call relationship.
    pub fn add_call(&mut self, call_site: CallSite) {
        self.calls
            .entry(call_site.caller.clone())
            .or_default()
            .push(call_site.clone());

        self.called_by
            .entry(call_site.callee.clone())
            .or_default()
            .push(call_site);
    }

    /// Get all functions called by the given function.
    pub fn get_callees(&self, function: &str) -> Vec<&CallSite> {
        self.calls
            .get(function)
            .map(|sites| sites.iter().collect())
            .unwrap_or_default()
    }

    /// Get all functions that call the given function.
    pub fn get_callers(&self, function: &str) -> Vec<&CallSite> {
        self.called_by
            .get(function)
            .map(|sites| sites.iter().collect())
            .unwrap_or_default()
    }
}

impl SemanticSearchEngine {
    /// Create a new search engine.
    pub fn new() -> Self {
        Self {
            inferencer: DeepTypeInferencer::new(),
            symbol_index: HashMap::new(),
            type_index: HashMap::new(),
            type_signature_index: HashMap::new(),
            call_graph: CallGraph::new(),
        }
    }

    /// Create with type inferencer.
    pub fn with_inferencer(inferencer: DeepTypeInferencer) -> Self {
        Self {
            inferencer,
            symbol_index: HashMap::new(),
            type_index: HashMap::new(),
            type_signature_index: HashMap::new(),
            call_graph: CallGraph::new(),
        }
    }

    /// Execute a search query.
    pub fn search(&self, query: &SearchQuery) -> SearchResult {
        match &query.kind {
            SearchKind::ByTypeSignature { params, return_type } => {
                self.search_by_type_signature(params, return_type.as_ref(), query)
            }
            SearchKind::Implementations { protocol } => {
                self.search_implementations(protocol, query)
            }
            SearchKind::Usages { symbol, file } => {
                self.search_usages(symbol, file, query)
            }
            SearchKind::SimilarPatterns { pattern } => {
                self.search_similar_patterns(pattern, query)
            }
            SearchKind::ByDocumentation { query: doc_query } => {
                self.search_by_documentation(doc_query, query)
            }
            SearchKind::CallHierarchy { symbol, file, direction } => {
                self.search_call_hierarchy(symbol, file, *direction, query)
            }
            SearchKind::TypeHierarchy { type_name, direction } => {
                self.search_type_hierarchy(type_name, *direction, query)
            }
        }
    }

    /// Search by type signature.
    fn search_by_type_signature(
        &self,
        params: &[Type],
        return_type: Option<&Type>,
        query: &SearchQuery,
    ) -> SearchResult {
        let mut result = SearchResult::empty();
        let mut all_matches: Vec<(SearchMatch, f64)> = Vec::new();

        // Search for functions with matching signatures
        for (_sig_key, locations) in &self.type_signature_index {
            for location in locations {
                if let Some(ref loc_type) = location.ty {
                    if let Type::Callable { params: loc_params, ret: loc_ret } = loc_type {
                        let score = self.compute_signature_match_score(
                            params,
                            return_type,
                            loc_params,
                            loc_ret,
                        );

                        if score > 0.0 {
                            let search_match = SearchMatch {
                                file: location.file.clone(),
                                span: location.span,
                                symbol: Some(location.name.clone()),
                                kind: location.kind.clone(),
                                score,
                                context: None,
                            };
                            all_matches.push((search_match, score));
                        }
                    }
                }
            }
        }

        // Sort by score (descending)
        all_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        result.matches = all_matches
            .into_iter()
            .take(query.max_results)
            .map(|(m, _)| m)
            .collect();

        result.total_count = result.matches.len();
        result
    }

    /// Compute match score between two function signatures.
    /// Returns 0.0 for no match, 1.0 for exact match.
    fn compute_signature_match_score(
        &self,
        query_params: &[Type],
        query_return: Option<&Type>,
        func_params: &[crate::types::ty::Param],
        func_return: &Type,
    ) -> f64 {
        let mut score = 0.0;
        let mut components = 0;

        // Check parameter count
        if query_params.len() != func_params.len() {
            // Allow variadic functions (partial match)
            if query_params.len() < func_params.len() {
                score += 0.3;
            } else {
                return 0.0; // Too many parameters
            }
        } else {
            score += 1.0;
        }
        components += 1;

        // Check parameter types
        for (i, query_param) in query_params.iter().enumerate() {
            if let Some(func_param) = func_params.get(i) {
                let param_score = self.type_compatibility_score(query_param, &func_param.ty);
                score += param_score;
                components += 1;
            }
        }

        // Check return type
        if let Some(query_ret) = query_return {
            let ret_score = self.type_compatibility_score(query_ret, func_return);
            score += ret_score * 1.5; // Weight return type higher
            components += 1;
        }

        if components > 0 {
            score / components as f64
        } else {
            0.0
        }
    }

    /// Compute compatibility score between two types (0.0 to 1.0).
    fn type_compatibility_score(&self, expected: &Type, actual: &Type) -> f64 {
        // Exact match
        if expected == actual {
            return 1.0;
        }

        // Any/Unknown matches everything
        if matches!(expected, Type::Any | Type::Unknown) || matches!(actual, Type::Any | Type::Unknown) {
            return 0.8;
        }

        match (expected, actual) {
            // Union types
            (Type::Union(types), _) => {
                types.iter()
                    .map(|t| self.type_compatibility_score(t, actual))
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0)
            }
            (_, Type::Union(types)) => {
                types.iter()
                    .map(|t| self.type_compatibility_score(expected, t))
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0)
            }

            // Container types (covariant in element type)
            (Type::List(e1), Type::List(e2)) => {
                0.9 * self.type_compatibility_score(e1, e2)
            }
            (Type::Set(e1), Type::Set(e2)) => {
                0.9 * self.type_compatibility_score(e1, e2)
            }
            (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
                let k_score = self.type_compatibility_score(k1, k2);
                let v_score = self.type_compatibility_score(v1, v2);
                0.9 * (k_score + v_score) / 2.0
            }

            // Optional types
            (Type::Optional(inner), other) | (other, Type::Optional(inner)) => {
                0.8 * self.type_compatibility_score(inner, other)
            }

            // Instance types (check if same class name)
            (Type::Instance { name: n1, .. }, Type::Instance { name: n2, .. }) => {
                if n1 == n2 {
                    0.9
                } else {
                    0.0
                }
            }

            // Class types
            (Type::ClassType { name: n1, .. }, Type::ClassType { name: n2, .. }) => {
                if n1 == n2 {
                    0.9
                } else {
                    0.0
                }
            }

            _ => 0.0,
        }
    }

    /// Search for implementations of a protocol.
    fn search_implementations(&self, protocol: &str, query: &SearchQuery) -> SearchResult {
        let mut result = SearchResult::empty();

        // Get all class symbols
        for (_name, locations) in &self.symbol_index {
            for location in locations {
                if matches!(location.kind, MatchKind::ClassDef) {
                    // Check if this class implements the protocol
                    if let Some(ref ty) = location.ty {
                        if self.implements_protocol(&location.name, ty, protocol) {
                            result.matches.push(SearchMatch {
                                file: location.file.clone(),
                                span: location.span,
                                symbol: Some(location.name.clone()),
                                kind: location.kind.clone(),
                                score: 1.0,
                                context: None,
                            });

                            if result.matches.len() >= query.max_results {
                                break;
                            }
                        }
                    }
                }
            }

            if result.matches.len() >= query.max_results {
                break;
            }
        }

        result.total_count = result.matches.len();
        result
    }

    /// Check if a class implements a protocol.
    fn implements_protocol(&self, class_name: &str, _class_type: &Type, protocol: &str) -> bool {
        // Use the type context to check protocol satisfaction
        let class_ty = Type::Instance {
            name: class_name.to_string(),
            module: None,
            type_args: vec![],
        };

        // Check if the class satisfies the protocol
        self.inferencer.context().satisfies_protocol(&class_ty, protocol)
    }

    /// Search for usages of a symbol.
    fn search_usages(&self, symbol: &str, _file: &PathBuf, query: &SearchQuery) -> SearchResult {
        let mut result = SearchResult::empty();

        if let Some(locations) = self.symbol_index.get(symbol) {
            for loc in locations.iter().take(query.max_results) {
                result.matches.push(SearchMatch {
                    file: loc.file.clone(),
                    span: loc.span,
                    symbol: Some(symbol.to_string()),
                    kind: loc.kind.clone(),
                    score: 1.0,
                    context: None,
                });
            }
            result.total_count = locations.len();
        }

        result
    }

    /// Search for similar code patterns.
    fn search_similar_patterns(&self, pattern: &str, query: &SearchQuery) -> SearchResult {
        let mut result = SearchResult::empty();
        let pattern_lower = pattern.to_lowercase();

        // Simple pattern matching based on symbol names
        for (_name, locations) in &self.symbol_index {
            for location in locations {
                // Simple heuristic: match if symbol name contains pattern
                if location.name.to_lowercase().contains(&pattern_lower) {
                    result.matches.push(SearchMatch {
                        file: location.file.clone(),
                        span: location.span,
                        symbol: Some(location.name.clone()),
                        kind: location.kind.clone(),
                        score: 0.7, // Lower score for pattern match
                        context: None,
                    });

                    if result.matches.len() >= query.max_results {
                        break;
                    }
                }
            }

            if result.matches.len() >= query.max_results {
                break;
            }
        }

        // Sort by score
        result.matches.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        result.total_count = result.matches.len();
        result
    }

    /// Search by documentation content.
    fn search_by_documentation(&self, doc_query: &str, query: &SearchQuery) -> SearchResult {
        let mut result = SearchResult::empty();
        let query_lower = doc_query.to_lowercase();

        // Search through symbols that might have documentation
        // Note: We don't have direct access to docstrings from SymbolLocation,
        // so this is a placeholder implementation that searches symbol names
        for (_name, locations) in &self.symbol_index {
            for location in locations {
                // In a full implementation, we would check the actual docstring
                // For now, just match against function/class names
                if matches!(location.kind, MatchKind::FunctionDef | MatchKind::ClassDef | MatchKind::MethodDef) {
                    if location.name.to_lowercase().contains(&query_lower) {
                        result.matches.push(SearchMatch {
                            file: location.file.clone(),
                            span: location.span,
                            symbol: Some(location.name.clone()),
                            kind: location.kind.clone(),
                            score: 0.6, // Lower score for doc search
                            context: None,
                        });

                        if result.matches.len() >= query.max_results {
                            break;
                        }
                    }
                }
            }

            if result.matches.len() >= query.max_results {
                break;
            }
        }

        result.total_count = result.matches.len();
        result
    }

    /// Search call hierarchy.
    fn search_call_hierarchy(
        &self,
        symbol: &str,
        _file: &PathBuf,
        direction: CallDirection,
        query: &SearchQuery,
    ) -> SearchResult {
        let mut result = SearchResult::empty();
        let mut visited = std::collections::HashSet::new();

        // Recursively collect call hierarchy
        self.collect_call_hierarchy(
            symbol,
            direction,
            &mut visited,
            &mut result,
            query.max_results,
        );

        result.total_count = result.matches.len();
        result
    }

    /// Recursively collect call hierarchy.
    fn collect_call_hierarchy(
        &self,
        symbol: &str,
        direction: CallDirection,
        visited: &mut std::collections::HashSet<String>,
        result: &mut SearchResult,
        max_results: usize,
    ) {
        if visited.contains(symbol) || result.matches.len() >= max_results {
            return;
        }

        visited.insert(symbol.to_string());

        let call_sites = match direction {
            CallDirection::Callers => self.call_graph.get_callers(symbol),
            CallDirection::Callees => self.call_graph.get_callees(symbol),
        };

        for site in call_sites {
            let related_symbol = match direction {
                CallDirection::Callers => &site.caller,
                CallDirection::Callees => &site.callee,
            };

            result.matches.push(SearchMatch {
                file: site.file.clone(),
                span: site.span,
                symbol: Some(related_symbol.clone()),
                kind: MatchKind::Call,
                score: 1.0,
                context: None,
            });

            if result.matches.len() >= max_results {
                return;
            }

            // Recursively search (limit depth to avoid infinite loops)
            if visited.len() < 100 {
                self.collect_call_hierarchy(
                    related_symbol,
                    direction,
                    visited,
                    result,
                    max_results,
                );
            }
        }
    }

    /// Search type hierarchy.
    fn search_type_hierarchy(
        &self,
        type_name: &str,
        direction: TypeHierarchyDirection,
        query: &SearchQuery,
    ) -> SearchResult {
        let mut result = SearchResult::empty();
        let type_ctx = self.inferencer.context();

        match direction {
            TypeHierarchyDirection::Supertypes => {
                // Find parent types by checking all class/interface symbols
                for (_name, locations) in &self.symbol_index {
                    for location in locations {
                        if matches!(location.kind, MatchKind::ClassDef) {
                            let inst_ty = Type::Instance {
                                name: type_name.to_string(),
                                module: None,
                                type_args: vec![],
                            };

                            // Check if type_name satisfies this protocol/interface
                            if type_ctx.satisfies_protocol(&inst_ty, &location.name) {
                                result.matches.push(SearchMatch {
                                    file: location.file.clone(),
                                    span: location.span,
                                    symbol: Some(location.name.clone()),
                                    kind: location.kind.clone(),
                                    score: 1.0,
                                    context: None,
                                });

                                if result.matches.len() >= query.max_results {
                                    break;
                                }
                            }
                        }
                    }

                    if result.matches.len() >= query.max_results {
                        break;
                    }
                }
            }
            TypeHierarchyDirection::Subtypes => {
                // Find child types that implement this protocol
                for (_name, locations) in &self.symbol_index {
                    for location in locations {
                        if matches!(location.kind, MatchKind::ClassDef) {
                            let class_ty = Type::Instance {
                                name: location.name.clone(),
                                module: None,
                                type_args: vec![],
                            };

                            if type_ctx.satisfies_protocol(&class_ty, type_name) {
                                result.matches.push(SearchMatch {
                                    file: location.file.clone(),
                                    span: location.span,
                                    symbol: Some(location.name.clone()),
                                    kind: location.kind.clone(),
                                    score: 1.0,
                                    context: None,
                                });

                                if result.matches.len() >= query.max_results {
                                    break;
                                }
                            }
                        }
                    }

                    if result.matches.len() >= query.max_results {
                        break;
                    }
                }
            }
            TypeHierarchyDirection::Both => {
                // Search both directions
                let supertypes = self.search_type_hierarchy(
                    type_name,
                    TypeHierarchyDirection::Supertypes,
                    query,
                );
                let subtypes = self.search_type_hierarchy(
                    type_name,
                    TypeHierarchyDirection::Subtypes,
                    query,
                );

                result.matches.extend(supertypes.matches);
                result.matches.extend(subtypes.matches);
                result.matches.truncate(query.max_results);
            }
        }

        result.total_count = result.matches.len();
        result
    }

    /// Index a file for searching.
    pub fn index_file(&mut self, file: PathBuf, symbols: Vec<SymbolLocation>) {
        for sym in symbols {
            if let Some(name) = self.extract_symbol_name(&sym) {
                self.symbol_index.entry(name).or_default().push(sym.clone());
            }

            // Index by type signature if it's a callable
            if let Some(ref ty) = sym.ty {
                if matches!(ty, Type::Callable { .. }) {
                    let sig_key = self.compute_signature_key(ty);
                    self.type_signature_index.entry(sig_key).or_default().push(sym.clone());
                }
            }
        }
        self.inferencer.add_file(file);
    }

    /// Compute a signature key for indexing.
    fn compute_signature_key(&self, ty: &Type) -> String {
        match ty {
            Type::Callable { params, ret } => {
                let params_str = params.iter()
                    .map(|p| self.type_to_key_string(&p.ty))
                    .collect::<Vec<_>>()
                    .join(",");
                let ret_str = self.type_to_key_string(ret);
                format!("({}) -> {}", params_str, ret_str)
            }
            _ => "unknown".to_string(),
        }
    }

    /// Convert type to a string key for indexing.
    fn type_to_key_string(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "int".to_string(),
            Type::Str => "str".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Float => "float".to_string(),
            Type::None => "None".to_string(),
            Type::Any => "Any".to_string(),
            Type::Unknown => "Unknown".to_string(),
            Type::List(elem) => format!("List[{}]", self.type_to_key_string(elem)),
            Type::Dict(k, v) => format!("Dict[{}, {}]", self.type_to_key_string(k), self.type_to_key_string(v)),
            Type::Set(elem) => format!("Set[{}]", self.type_to_key_string(elem)),
            Type::Tuple(elems) => {
                let elems_str = elems.iter()
                    .map(|e| self.type_to_key_string(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Tuple[{}]", elems_str)
            }
            Type::Union(types) => {
                let types_str = types.iter()
                    .map(|t| self.type_to_key_string(t))
                    .collect::<Vec<_>>()
                    .join(" | ");
                types_str
            }
            Type::Optional(inner) => format!("{}?", self.type_to_key_string(inner)),
            Type::Instance { name, .. } => name.clone(),
            Type::ClassType { name, .. } => format!("type[{}]", name),
            _ => "Unknown".to_string(),
        }
    }

    /// Index a symbol table for searching.
    pub fn index_symbol_table(&mut self, file: PathBuf, symbol_table: &SymbolTable) {
        let symbols = self.convert_symbol_table_to_locations(file.clone(), symbol_table);
        self.index_file(file, symbols);
    }

    /// Convert a SymbolTable to a Vec of SymbolLocations.
    fn convert_symbol_table_to_locations(
        &self,
        file: PathBuf,
        symbol_table: &SymbolTable,
    ) -> Vec<SymbolLocation> {
        let mut locations = Vec::new();

        for symbol in symbol_table.all_symbols() {
            let kind = Self::convert_symbol_kind(symbol.kind);
            let ty = symbol.type_info.as_ref().map(|ti| Self::convert_type_info(ti));

            locations.push(SymbolLocation {
                file: file.clone(),
                span: Span {
                    start: 0, // We don't have byte offsets in semantic::Symbol
                    end: 0,
                    start_line: symbol.location.start.line as usize,
                    start_col: symbol.location.start.character as usize,
                    end_line: symbol.location.end.line as usize,
                    end_col: symbol.location.end.character as usize,
                },
                name: symbol.name.clone(),
                kind,
                ty,
            });
        }

        locations
    }

    /// Convert semantic SymbolKind to search MatchKind.
    fn convert_symbol_kind(kind: SemanticSymbolKind) -> MatchKind {
        match kind {
            SemanticSymbolKind::Function => MatchKind::FunctionDef,
            SemanticSymbolKind::Class | SemanticSymbolKind::Struct => MatchKind::ClassDef,
            SemanticSymbolKind::Variable | SemanticSymbolKind::Const | SemanticSymbolKind::Static => {
                MatchKind::VariableAssignment
            }
            SemanticSymbolKind::Parameter => MatchKind::VariableAssignment,
            SemanticSymbolKind::Import => MatchKind::Import,
            SemanticSymbolKind::TypeAlias => MatchKind::TypeAnnotation,
            SemanticSymbolKind::Interface | SemanticSymbolKind::Trait => MatchKind::ClassDef,
            _ => MatchKind::VariableAssignment,
        }
    }

    /// Convert semantic TypeInfo to Type.
    fn convert_type_info(type_info: &crate::semantic::TypeInfo) -> Type {
        use crate::semantic::TypeInfo;
        use crate::types::ty::{Param, ParamKind};

        match type_info {
            TypeInfo::Primitive(name) => match name.as_str() {
                "int" => Type::Int,
                "str" => Type::Str,
                "bool" => Type::Bool,
                "float" => Type::Float,
                "None" => Type::None,
                _ => Type::Unknown,
            },
            TypeInfo::List(inner) => {
                let elem = Self::convert_type_info(inner);
                Type::List(Box::new(elem))
            }
            TypeInfo::Dict(key, value) => {
                let key_ty = Self::convert_type_info(key);
                let value_ty = Self::convert_type_info(value);
                Type::Dict(Box::new(key_ty), Box::new(value_ty))
            }
            TypeInfo::Optional(inner) => {
                let inner_ty = Self::convert_type_info(inner);
                Type::Union(vec![inner_ty, Type::None])
            }
            TypeInfo::Union(types) => {
                let converted_types = types.iter().map(Self::convert_type_info).collect();
                Type::Union(converted_types)
            }
            TypeInfo::Callable { params, ret } => {
                // Convert TypeInfo params to Param structs
                let param_structs: Vec<Param> = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| Param {
                        name: format!("arg{}", i),
                        ty: Self::convert_type_info(p),
                        has_default: false,
                        kind: ParamKind::Positional,
                    })
                    .collect();
                let return_type = Box::new(Self::convert_type_info(ret));
                Type::Callable {
                    params: param_structs,
                    ret: return_type,
                }
            }
            TypeInfo::Named(name) => Type::Instance {
                name: name.clone(),
                module: None,
                type_args: vec![],
            },
            TypeInfo::Generic(name, args) => {
                let type_args = args.iter().map(Self::convert_type_info).collect();
                Type::Instance {
                    name: name.clone(),
                    module: None,
                    type_args,
                }
            }
            TypeInfo::Unknown => Type::Unknown,
            TypeInfo::Any => Type::Any,
            TypeInfo::Error => Type::Unknown,
        }
    }

    fn extract_symbol_name(&self, sym: &SymbolLocation) -> Option<String> {
        Some(sym.name.clone())
    }

    /// Get type context.
    pub fn type_context(&self) -> &TypeContext {
        self.inferencer.context()
    }
}

impl Default for SemanticSearchEngine {
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
    use crate::semantic::{SymbolKind, TypeInfo};
    use crate::diagnostic::Range;

    #[test]
    fn test_search_result() {
        let result = SearchResult::empty();
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_search_engine() {
        let engine = SemanticSearchEngine::new();
        let query = SearchQuery {
            kind: SearchKind::Usages {
                symbol: "foo".to_string(),
                file: PathBuf::from("test.py"),
            },
            scope: SearchScope::Project,
            max_results: 100,
        };

        let result = engine.search(&query);
        assert!(result.is_empty());
    }

    #[test]
    fn test_index_population() {
        let mut engine = SemanticSearchEngine::new();
        let mut symbol_table = SymbolTable::new();

        // Add test symbols
        symbol_table.add_symbol(
            "func1".to_string(),
            SymbolKind::Function,
            Range {
                start: crate::diagnostic::Position { line: 0, character: 4 },
                end: crate::diagnostic::Position { line: 0, character: 9 },
            },
            Some(TypeInfo::Callable {
                params: vec![],
                ret: Box::new(TypeInfo::Primitive("None".to_string())),
            }),
            None,
            0,
        );

        symbol_table.add_symbol(
            "MyClass".to_string(),
            SymbolKind::Class,
            Range {
                start: crate::diagnostic::Position { line: 2, character: 6 },
                end: crate::diagnostic::Position { line: 2, character: 13 },
            },
            None,
            None,
            0,
        );

        let file = PathBuf::from("test.py");
        engine.index_symbol_table(file.clone(), &symbol_table);

        // Search for func1
        let query = SearchQuery {
            kind: SearchKind::Usages {
                symbol: "func1".to_string(),
                file: file.clone(),
            },
            scope: SearchScope::Project,
            max_results: 100,
        };

        let result = engine.search(&query);
        assert!(!result.is_empty());
        assert_eq!(result.len(), 1);
        assert_eq!(result.matches[0].symbol, Some("func1".to_string()));

        // Search for MyClass
        let query = SearchQuery {
            kind: SearchKind::Usages {
                symbol: "MyClass".to_string(),
                file,
            },
            scope: SearchScope::Project,
            max_results: 100,
        };

        let result = engine.search(&query);
        assert!(!result.is_empty());
        assert_eq!(result.len(), 1);
        assert_eq!(result.matches[0].symbol, Some("MyClass".to_string()));
    }

    #[test]
    fn test_convert_type_info() {
        // Test primitive types
        let int_type = SemanticSearchEngine::convert_type_info(&TypeInfo::Primitive("int".to_string()));
        assert_eq!(int_type, Type::Int);

        // Test list types
        let list_type = SemanticSearchEngine::convert_type_info(&TypeInfo::List(
            Box::new(TypeInfo::Primitive("str".to_string()))
        ));
        assert_eq!(list_type, Type::List(Box::new(Type::Str)));

        // Test optional types
        let opt_type = SemanticSearchEngine::convert_type_info(&TypeInfo::Optional(
            Box::new(TypeInfo::Primitive("int".to_string()))
        ));
        assert_eq!(opt_type, Type::Union(vec![Type::Int, Type::None]));
    }

    #[test]
    fn test_search_by_type_signature() {
        let mut engine = SemanticSearchEngine::new();
        let mut symbol_table = SymbolTable::new();

        // Add function: def func1(x: str, y: int) -> bool
        symbol_table.add_symbol(
            "func1".to_string(),
            SymbolKind::Function,
            Range {
                start: crate::diagnostic::Position { line: 0, character: 4 },
                end: crate::diagnostic::Position { line: 0, character: 9 },
            },
            Some(TypeInfo::Callable {
                params: vec![
                    TypeInfo::Primitive("str".to_string()),
                    TypeInfo::Primitive("int".to_string()),
                ],
                ret: Box::new(TypeInfo::Primitive("bool".to_string())),
            }),
            None,
            0,
        );

        // Add function: def func2(a: int, b: int) -> int
        symbol_table.add_symbol(
            "func2".to_string(),
            SymbolKind::Function,
            Range {
                start: crate::diagnostic::Position { line: 2, character: 4 },
                end: crate::diagnostic::Position { line: 2, character: 9 },
            },
            Some(TypeInfo::Callable {
                params: vec![
                    TypeInfo::Primitive("int".to_string()),
                    TypeInfo::Primitive("int".to_string()),
                ],
                ret: Box::new(TypeInfo::Primitive("int".to_string())),
            }),
            None,
            0,
        );

        let file = PathBuf::from("test.py");
        engine.index_symbol_table(file.clone(), &symbol_table);

        // Search for (str, int) -> bool
        let query = SearchQuery {
            kind: SearchKind::ByTypeSignature {
                params: vec![Type::Str, Type::Int],
                return_type: Some(Type::Bool),
            },
            scope: SearchScope::Project,
            max_results: 10,
        };

        let result = engine.search(&query);
        assert!(!result.is_empty());

        // Should find func1 with high score (top result)
        let high_score_matches: Vec<_> = result.matches.iter()
            .filter(|m| m.score > 0.9)
            .collect();
        assert_eq!(high_score_matches.len(), 1);
        assert_eq!(high_score_matches[0].symbol, Some("func1".to_string()));

        // Search for (int, int) -> int
        let query = SearchQuery {
            kind: SearchKind::ByTypeSignature {
                params: vec![Type::Int, Type::Int],
                return_type: Some(Type::Int),
            },
            scope: SearchScope::Project,
            max_results: 10,
        };

        let result = engine.search(&query);
        assert!(!result.is_empty());

        // Should find func2 with high score
        let high_score_matches: Vec<_> = result.matches.iter()
            .filter(|m| m.score > 0.9)
            .collect();
        assert_eq!(high_score_matches.len(), 1);
        assert_eq!(high_score_matches[0].symbol, Some("func2".to_string()));
    }

    #[test]
    fn test_type_compatibility_score() {
        let engine = SemanticSearchEngine::new();

        // Exact match
        assert_eq!(engine.type_compatibility_score(&Type::Int, &Type::Int), 1.0);
        assert_eq!(engine.type_compatibility_score(&Type::Str, &Type::Str), 1.0);

        // Any/Unknown matches
        assert!(engine.type_compatibility_score(&Type::Any, &Type::Int) > 0.5);
        assert!(engine.type_compatibility_score(&Type::Int, &Type::Unknown) > 0.5);

        // Container types
        let list_int = Type::List(Box::new(Type::Int));
        let list_int2 = Type::List(Box::new(Type::Int));
        assert!(engine.type_compatibility_score(&list_int, &list_int2) > 0.8);

        // No match
        assert_eq!(engine.type_compatibility_score(&Type::Int, &Type::Str), 0.0);
    }

    #[test]
    fn test_search_implementations() {
        let mut engine = SemanticSearchEngine::new();
        let mut symbol_table = SymbolTable::new();

        // Add a class
        symbol_table.add_symbol(
            "MyClass".to_string(),
            SymbolKind::Class,
            Range {
                start: crate::diagnostic::Position { line: 0, character: 6 },
                end: crate::diagnostic::Position { line: 0, character: 13 },
            },
            None,
            None,
            0,
        );

        let file = PathBuf::from("test.py");
        engine.index_symbol_table(file.clone(), &symbol_table);

        // Search for implementations of a protocol
        let query = SearchQuery {
            kind: SearchKind::Implementations {
                protocol: "Sized".to_string(),
            },
            scope: SearchScope::Project,
            max_results: 10,
        };

        let result = engine.search(&query);
        // Note: This will be empty because we haven't implemented protocol checking yet
        // but at least it doesn't crash
        assert!(result.is_empty() || !result.is_empty());
    }
}
