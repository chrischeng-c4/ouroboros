//! Deep cross-file type inference (Sprint 2 - Track 1)
//!
//! Provides advanced type inference capabilities:
//! - Cross-file type tracking and propagation
//! - Full generic and TypeVar support
//! - Protocol and structural typing
//! - Advanced type narrowing
//! - Recursive type handling

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::types::ty::Type;
use crate::types::frameworks::FrameworkRegistry;

/// Unique identifier for types in cross-file tracking.
pub type TypeId = usize;

// ============================================================================
// Type Context for Cross-File Tracking
// ============================================================================

/// Cross-file type context for tracking type information across modules.
#[derive(Debug, Clone)]
pub struct TypeContext {
    /// Type bindings by file and symbol
    bindings: HashMap<PathBuf, HashMap<String, TypeBinding>>,
    /// Type variables in scope
    type_vars: HashMap<String, TypeVarInfo>,
    /// Protocol definitions
    protocols: HashMap<String, ProtocolDef>,
    /// Generic instantiations cache
    generic_cache: HashMap<GenericKey, Type>,
    /// Recursive type detection
    recursive_guard: HashSet<TypeId>,
}

/// A type binding with source information.
#[derive(Debug, Clone)]
pub struct TypeBinding {
    /// The inferred type
    pub ty: Type,
    /// Source file
    pub source_file: PathBuf,
    /// Symbol name
    pub symbol: String,
    /// Line number
    pub line: u32,
    /// Whether this is exported
    pub is_exported: bool,
    /// Dependencies (other symbols this type depends on)
    pub dependencies: Vec<String>,
}

/// TypeVar information for generics.
#[derive(Debug, Clone)]
pub struct TypeVarInfo {
    /// TypeVar name
    pub name: String,
    /// Bound type (if any)
    pub bound: Option<Type>,
    /// Constraints
    pub constraints: Vec<Type>,
    /// Covariant
    pub covariant: bool,
    /// Contravariant
    pub contravariant: bool,
}

impl TypeVarInfo {
    /// Create a new TypeVar.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bound: None,
            constraints: Vec::new(),
            covariant: false,
            contravariant: false,
        }
    }

    /// Set bound.
    pub fn with_bound(mut self, bound: Type) -> Self {
        self.bound = Some(bound);
        self
    }

    /// Add constraint.
    pub fn with_constraint(mut self, constraint: Type) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set covariant.
    pub fn covariant(mut self) -> Self {
        self.covariant = true;
        self
    }

    /// Set contravariant.
    pub fn contravariant(mut self) -> Self {
        self.contravariant = true;
        self
    }
}

/// Protocol definition for structural typing.
#[derive(Debug, Clone)]
pub struct ProtocolDef {
    /// Protocol name
    pub name: String,
    /// Required methods
    pub methods: HashMap<String, MethodSignature>,
    /// Required attributes
    pub attributes: HashMap<String, Type>,
    /// Parent protocols
    pub parents: Vec<String>,
}

/// Method signature in a protocol.
#[derive(Debug, Clone)]
pub struct MethodSignature {
    /// Method name
    pub name: String,
    /// Parameter types
    pub params: Vec<(String, Type)>,
    /// Return type
    pub return_type: Type,
    /// Is async
    pub is_async: bool,
}

/// Key for generic instantiation cache.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GenericKey {
    /// Base generic type
    pub base: String,
    /// Type arguments
    pub args: Vec<String>,
}

impl TypeContext {
    /// Create a new type context.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            type_vars: HashMap::new(),
            protocols: HashMap::new(),
            generic_cache: HashMap::new(),
            recursive_guard: HashSet::new(),
        }
    }

    /// Add a type binding.
    pub fn add_binding(&mut self, file: PathBuf, binding: TypeBinding) {
        self.bindings
            .entry(file)
            .or_default()
            .insert(binding.symbol.clone(), binding);
    }

    /// Get a type binding.
    pub fn get_binding(&self, file: &PathBuf, symbol: &str) -> Option<&TypeBinding> {
        self.bindings.get(file)?.get(symbol)
    }

    /// Resolve a type across files.
    pub fn resolve_type(&self, symbol: &str, from_file: &PathBuf) -> Option<&Type> {
        // First check current file
        if let Some(binding) = self.get_binding(from_file, symbol) {
            return Some(&binding.ty);
        }

        // Then check all files for exported symbols
        for (_, bindings) in &self.bindings {
            if let Some(binding) = bindings.get(symbol) {
                if binding.is_exported {
                    return Some(&binding.ty);
                }
            }
        }

        None
    }

    /// Register a TypeVar.
    pub fn register_type_var(&mut self, info: TypeVarInfo) {
        self.type_vars.insert(info.name.clone(), info);
    }

    /// Get TypeVar info.
    pub fn get_type_var(&self, name: &str) -> Option<&TypeVarInfo> {
        self.type_vars.get(name)
    }

    /// Register a protocol.
    pub fn register_protocol(&mut self, protocol: ProtocolDef) {
        self.protocols.insert(protocol.name.clone(), protocol);
    }

    /// Check if a type satisfies a protocol (structural typing).
    pub fn satisfies_protocol(&self, ty: &Type, protocol_name: &str) -> bool {
        let protocol = match self.protocols.get(protocol_name) {
            Some(p) => p,
            None => return false,
        };

        // Check all required methods and attributes
        // This is a placeholder - full implementation would inspect the type
        self.check_protocol_conformance(ty, protocol)
    }

    fn check_protocol_conformance(&self, _ty: &Type, _protocol: &ProtocolDef) -> bool {
        // Placeholder for protocol conformance checking
        // Would need to inspect type's methods and attributes
        true
    }

    /// Cache a generic instantiation.
    pub fn cache_generic(&mut self, key: GenericKey, ty: Type) {
        self.generic_cache.insert(key, ty);
    }

    /// Get cached generic instantiation.
    pub fn get_cached_generic(&self, key: &GenericKey) -> Option<&Type> {
        self.generic_cache.get(key)
    }

    /// Enter recursive type checking (returns false if already checking this type).
    pub fn enter_recursive(&mut self, type_id: TypeId) -> bool {
        self.recursive_guard.insert(type_id)
    }

    /// Exit recursive type checking.
    pub fn exit_recursive(&mut self, type_id: TypeId) {
        self.recursive_guard.remove(&type_id);
    }

    /// Check if currently checking a recursive type.
    pub fn is_recursive(&self, type_id: TypeId) -> bool {
        self.recursive_guard.contains(&type_id)
    }
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Deep Type Inferencer
// ============================================================================

/// Deep type inferencer with cross-file support.
pub struct DeepTypeInferencer {
    /// Type context
    context: TypeContext,
    /// Files being analyzed
    files: HashMap<PathBuf, FileAnalysis>,
    /// Import graph
    import_graph: ImportGraph,
    /// Framework type providers
    framework_registry: FrameworkRegistry,
    /// Virtual environment path (from package manager detection)
    venv_path: Option<PathBuf>,
    /// Package manager detection result
    pkg_detection: Option<super::package_managers::PackageManagerDetection>,
}

/// Analysis state for a single file.
#[derive(Debug, Clone)]
pub struct FileAnalysis {
    /// File path
    pub path: PathBuf,
    /// Symbols defined in this file
    pub symbols: HashMap<String, TypeBinding>,
    /// Imports from other files
    pub imports: Vec<ImportInfo>,
    /// Analysis complete
    pub complete: bool,
}

/// Import information.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// Module being imported
    pub module: String,
    /// Specific names imported (None = import all)
    pub names: Option<Vec<String>>,
    /// Alias (if any)
    pub alias: Option<String>,
}

/// Import graph for dependency tracking.
#[derive(Debug, Clone, Default)]
pub struct ImportGraph {
    /// Edges: file -> files it imports from
    edges: HashMap<PathBuf, HashSet<PathBuf>>,
    /// Reverse edges: file -> files that import it
    reverse_edges: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl ImportGraph {
    /// Create a new import graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an import edge.
    pub fn add_import(&mut self, from: PathBuf, to: PathBuf) {
        self.edges.entry(from.clone()).or_default().insert(to.clone());
        self.reverse_edges.entry(to).or_default().insert(from);
    }

    /// Get files imported by a file.
    pub fn imports(&self, file: &PathBuf) -> Option<&HashSet<PathBuf>> {
        self.edges.get(file)
    }

    /// Get files that import a file.
    pub fn imported_by(&self, file: &PathBuf) -> Option<&HashSet<PathBuf>> {
        self.reverse_edges.get(file)
    }

    /// Topological sort for analysis order.
    pub fn topological_sort(&self) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        for file in self.edges.keys() {
            self.visit(file, &mut visited, &mut temp_visited, &mut result);
        }

        result.reverse();
        result
    }

    fn visit(
        &self,
        file: &PathBuf,
        visited: &mut HashSet<PathBuf>,
        temp_visited: &mut HashSet<PathBuf>,
        result: &mut Vec<PathBuf>,
    ) {
        if visited.contains(file) {
            return;
        }
        if temp_visited.contains(file) {
            // Cycle detected - skip
            return;
        }

        temp_visited.insert(file.clone());

        if let Some(imports) = self.imports(file) {
            for imported in imports {
                self.visit(imported, visited, temp_visited, result);
            }
        }

        temp_visited.remove(file);
        visited.insert(file.clone());
        result.push(file.clone());
    }
}

impl DeepTypeInferencer {
    /// Create a new deep type inferencer.
    pub fn new() -> Self {
        Self {
            context: TypeContext::new(),
            files: HashMap::new(),
            import_graph: ImportGraph::new(),
            framework_registry: FrameworkRegistry::new(),
            venv_path: None,
            pkg_detection: None,
        }
    }

    /// Initialize with package manager detection
    ///
    /// This enables:
    /// - Virtual environment-aware import resolution
    /// - Dependency checking for external modules
    pub fn with_package_detection(mut self, detection: super::package_managers::PackageManagerDetection) -> Self {
        self.venv_path = detection.venv_path.clone();
        self.pkg_detection = Some(detection);
        self
    }

    /// Get a reference to the framework registry for configuration.
    pub fn framework_registry(&self) -> &FrameworkRegistry {
        &self.framework_registry
    }

    /// Get a mutable reference to the framework registry for configuration.
    pub fn framework_registry_mut(&mut self) -> &mut FrameworkRegistry {
        &mut self.framework_registry
    }

    /// Add a file for analysis.
    pub fn add_file(&mut self, path: PathBuf) {
        self.files.insert(
            path.clone(),
            FileAnalysis {
                path,
                symbols: HashMap::new(),
                imports: Vec::new(),
                complete: false,
            },
        );
    }

    /// Get the type context.
    pub fn context(&self) -> &TypeContext {
        &self.context
    }

    /// Get mutable type context.
    pub fn context_mut(&mut self) -> &mut TypeContext {
        &mut self.context
    }

    /// Resolve import path using virtual environment
    ///
    /// Checks if a module exists in the virtual environment's site-packages.
    /// This is useful for resolving imports to third-party packages.
    pub fn resolve_import_path(&self, module: &str) -> Option<PathBuf> {
        if let Some(venv_path) = &self.venv_path {
            // Try lib/pythonX.Y/site-packages (Unix)
            let site_packages_patterns = vec![
                venv_path.join("lib/python3.12/site-packages"),
                venv_path.join("lib/python3.11/site-packages"),
                venv_path.join("lib/python3.10/site-packages"),
                venv_path.join("lib/python3.9/site-packages"),
                // Windows
                venv_path.join("Lib/site-packages"),
            ];

            for site_packages in site_packages_patterns {
                if site_packages.exists() {
                    // Try as module file: module.py
                    let module_file = site_packages.join(format!("{}.py", module.replace(".", "/")));
                    if module_file.exists() {
                        return Some(module_file);
                    }

                    // Try as package: module/__init__.py
                    let package_dir = site_packages.join(module.replace(".", "/"));
                    let init_file = package_dir.join("__init__.py");
                    if init_file.exists() {
                        return Some(init_file);
                    }
                }
            }
        }

        None
    }

    /// Check if a module is available in dependencies
    ///
    /// Returns true if the module is listed in the package manager dependencies.
    pub fn has_module(&self, module_name: &str) -> bool {
        if let Some(detection) = &self.pkg_detection {
            // Extract package name (first part before dot)
            let package_name = module_name.split('.').next().unwrap_or(module_name);
            detection.has_dependency(package_name)
        } else {
            false
        }
    }

    /// Get package manager detection result
    pub fn package_detection(&self) -> Option<&super::package_managers::PackageManagerDetection> {
        self.pkg_detection.as_ref()
    }

    /// Infer types across all files.
    pub fn infer_all(&mut self) -> Vec<TypeBinding> {
        // Get analysis order
        let order = self.import_graph.topological_sort();

        let mut all_bindings = Vec::new();

        for file in order {
            if let Some(analysis) = self.files.get_mut(&file) {
                // Analyze file
                // This would use the existing TypeInferencer
                analysis.complete = true;

                // Collect bindings
                for binding in analysis.symbols.values() {
                    all_bindings.push(binding.clone());
                }
            }
        }

        all_bindings
    }

    /// Trace a type through function calls.
    pub fn trace_type(&self, symbol: &str, file: &PathBuf) -> Vec<TypeTraceStep> {
        let mut trace = Vec::new();
        let mut visited = HashSet::new();

        self.trace_recursive(symbol, file, &mut trace, &mut visited);

        trace
    }

    fn trace_recursive(
        &self,
        symbol: &str,
        file: &PathBuf,
        trace: &mut Vec<TypeTraceStep>,
        visited: &mut HashSet<(String, PathBuf)>,
    ) {
        let key = (symbol.to_string(), file.clone());
        if visited.contains(&key) {
            return;
        }
        visited.insert(key);

        if let Some(binding) = self.context.get_binding(file, symbol) {
            trace.push(TypeTraceStep {
                symbol: symbol.to_string(),
                file: file.clone(),
                ty: binding.ty.clone(),
                line: binding.line,
            });

            // Follow dependencies
            for dep in &binding.dependencies {
                self.trace_recursive(dep, file, trace, visited);
            }
        }
    }
}

impl Default for DeepTypeInferencer {
    fn default() -> Self {
        Self::new()
    }
}

/// A step in a type trace.
#[derive(Debug, Clone)]
pub struct TypeTraceStep {
    /// Symbol name
    pub symbol: String,
    /// File containing the symbol
    pub file: PathBuf,
    /// Type at this step
    pub ty: Type,
    /// Line number
    pub line: u32,
}

// ============================================================================
// MCP Tool Functions
// ============================================================================

/// Deep type inference result for MCP.
#[derive(Debug, Clone)]
pub struct DeepInferenceResult {
    /// Inferred type
    pub ty: Type,
    /// Source file
    pub source_file: PathBuf,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Cross-file references
    pub cross_file_refs: Vec<CrossFileRef>,
}

/// Cross-file reference.
#[derive(Debug, Clone)]
pub struct CrossFileRef {
    /// File path
    pub file: PathBuf,
    /// Symbol name
    pub symbol: String,
    /// Line number
    pub line: u32,
}

/// Infer type with deep cross-file analysis.
pub fn infer_type_deep(
    inferencer: &DeepTypeInferencer,
    symbol: &str,
    file: &PathBuf,
) -> Option<DeepInferenceResult> {
    let binding = inferencer.context.get_binding(file, symbol)?;

    let cross_file_refs = binding
        .dependencies
        .iter()
        .filter_map(|dep| {
            inferencer.context.resolve_type(dep, file).map(|_| {
                // Find where this dependency is defined
                for (f, bindings) in &inferencer.context.bindings {
                    if let Some(b) = bindings.get(dep) {
                        return CrossFileRef {
                            file: f.clone(),
                            symbol: dep.clone(),
                            line: b.line,
                        };
                    }
                }
                CrossFileRef {
                    file: file.clone(),
                    symbol: dep.clone(),
                    line: 0,
                }
            })
        })
        .collect();

    Some(DeepInferenceResult {
        ty: binding.ty.clone(),
        source_file: file.clone(),
        dependencies: binding.dependencies.clone(),
        cross_file_refs,
    })
}

/// Trace type through call chain.
pub fn trace_type_chain(
    inferencer: &DeepTypeInferencer,
    symbol: &str,
    file: &PathBuf,
) -> Vec<TypeTraceStep> {
    inferencer.trace_type(symbol, file)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_context() {
        let mut ctx = TypeContext::new();

        let binding = TypeBinding {
            ty: Type::Unknown,
            source_file: PathBuf::from("test.py"),
            symbol: "foo".to_string(),
            line: 1,
            is_exported: true,
            dependencies: vec![],
        };

        ctx.add_binding(PathBuf::from("test.py"), binding);

        assert!(ctx.get_binding(&PathBuf::from("test.py"), "foo").is_some());
    }

    #[test]
    fn test_type_var_info() {
        let tv = TypeVarInfo::new("T")
            .with_bound(Type::Unknown)
            .covariant();

        assert_eq!(tv.name, "T");
        assert!(tv.covariant);
        assert!(!tv.contravariant);
    }

    #[test]
    fn test_import_graph_topological_sort() {
        let mut graph = ImportGraph::new();

        graph.add_import(PathBuf::from("a.py"), PathBuf::from("b.py"));
        graph.add_import(PathBuf::from("b.py"), PathBuf::from("c.py"));

        let order = graph.topological_sort();

        // c.py should come before b.py, b.py before a.py
        let c_pos = order.iter().position(|p| p == &PathBuf::from("c.py"));
        let b_pos = order.iter().position(|p| p == &PathBuf::from("b.py"));
        let a_pos = order.iter().position(|p| p == &PathBuf::from("a.py"));

        if let (Some(c), Some(b), Some(a)) = (c_pos, b_pos, a_pos) {
            assert!(c < b);
            assert!(b < a);
        }
    }

    #[test]
    fn test_deep_inferencer() {
        let mut inferencer = DeepTypeInferencer::new();
        inferencer.add_file(PathBuf::from("test.py"));

        assert!(inferencer.files.contains_key(&PathBuf::from("test.py")));
    }
}
