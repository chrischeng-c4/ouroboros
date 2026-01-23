use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

pub mod graph;
pub mod imports;
pub mod types;

pub use graph::{ModuleGraph, ModuleNode, EdgeKind};
pub use imports::{ModuleImports, ImportDeclaration, ImportKind};
pub use types::{BundleOptions, BundleOutput, ModuleId};

/// Determine module kind from file extension
fn determine_module_kind(path: &PathBuf) -> graph::ModuleKind {
    match path.extension().and_then(|e| e.to_str()) {
        Some("css") | Some("scss") | Some("sass") | Some("less") => graph::ModuleKind::Css,
        Some("json") => graph::ModuleKind::Json,
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") => {
            graph::ModuleKind::Asset
        }
        Some("woff") | Some("woff2") | Some("ttf") | Some("eot") => graph::ModuleKind::Asset,
        _ => graph::ModuleKind::Script,
    }
}

/// Calculate simple hash of content
fn calculate_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Generate runtime module system code
fn generate_runtime() -> String {
    r#"// Talos Module Runtime
(function() {
  'use strict';

  var modules = {};
  var cache = {};

  // Module definition
  function define(id, factory) {
    modules[id] = factory;
  }

  // Module require
  function require(id) {
    // Return cached module if exists
    if (cache[id]) {
      return cache[id].exports;
    }

    // Create module object
    var module = cache[id] = {
      exports: {},
      id: id,
      loaded: false
    };

    // Execute module factory
    var factory = modules[id];
    if (!factory) {
      throw new Error('Module not found: ' + id);
    }

    factory.call(module.exports, require, module, module.exports);
    module.loaded = true;

    return module.exports;
  }

  // Expose global runtime
  window.__talos__ = {
    define: define,
    require: require,
    modules: modules,
    cache: cache
  };
})();
"#.to_string()
}

/// Core bundler that orchestrates the build process
pub struct Bundler {
    /// Module resolver
    resolver: Arc<ouroboros_talos_resolver::ModuleResolver>,

    /// Code transformer
    transformer: Arc<ouroboros_talos_transform::Transformer>,

    /// Asset processor
    #[allow(dead_code)]
    asset_processor: Arc<ouroboros_talos_asset::AssetProcessor>,

    /// Module graph
    graph: Arc<RwLock<ModuleGraph>>,

    /// Compilation cache
    cache: Arc<CompilationCache>,
}

/// Compilation cache for incremental builds
pub struct CompilationCache {
    /// Maps (path, mtime) -> compiled module
    module_cache: DashMap<(PathBuf, u64), CompiledModule>,
}

/// Compiled module with metadata
#[derive(Debug, Clone)]
pub struct CompiledModule {
    /// Original source path
    pub path: PathBuf,

    /// Transformed code
    pub code: String,

    /// Source map (optional)
    pub source_map: Option<String>,

    /// Dependencies
    pub dependencies: Vec<String>,

    /// Module hash
    pub hash: String,
}

impl Bundler {
    /// Create a new bundler instance
    pub fn new(options: BundleOptions) -> Result<Self> {
        Ok(Self {
            resolver: Arc::new(ouroboros_talos_resolver::ModuleResolver::new(
                options.resolve_options,
            )?),
            transformer: Arc::new(ouroboros_talos_transform::Transformer::new(
                options.transform_options,
            )),
            asset_processor: Arc::new(ouroboros_talos_asset::AssetProcessor::new(
                options.asset_options,
            )),
            graph: Arc::new(RwLock::new(ModuleGraph::new())),
            cache: Arc::new(CompilationCache::new()),
        })
    }

    /// Bundle the application starting from entry point
    pub async fn bundle(&self, entry: PathBuf) -> Result<BundleOutput> {
        tracing::info!("Starting bundle from entry: {:?}", entry);

        // Build module graph
        self.build_graph(&entry).await?;

        // Transform modules
        let modules = self.transform_modules().await?;

        // Generate bundle
        let output = self.generate_bundle(modules)?;

        Ok(output)
    }

    /// Build the module dependency graph using iterative approach (avoids async recursion)
    async fn build_graph(&self, entry: &PathBuf) -> Result<()> {
        tracing::debug!("Building module graph from: {:?}", entry);

        let entry_abs = std::fs::canonicalize(entry)?;

        // Work queue: (module_path, parent_id, edge_kind)
        let mut queue: Vec<(PathBuf, Option<ModuleId>, Option<graph::EdgeKind>)> =
            vec![(entry_abs, None, None)];
        let mut visited = std::collections::HashSet::new();

        while let Some((module_path, parent_id, edge_kind)) = queue.pop() {
            // Skip if already visited
            if visited.contains(&module_path) {
                // Add edge if parent exists
                if let (Some(parent), Some(kind)) = (parent_id, edge_kind) {
                    let graph = self.graph.read();
                    if let Some(module_id) = graph.get_module(&module_path) {
                        drop(graph); // Release read lock
                        let mut graph = self.graph.write();
                        graph.add_dependency(parent, module_id, kind);
                    }
                }
                continue;
            }

            visited.insert(module_path.clone());

            tracing::debug!("Processing module: {:?}", module_path);

            // Read module source
            let source = match std::fs::read_to_string(&module_path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to read module {:?}: {}", module_path, e);
                    continue;
                }
            };

            let file_size = source.len() as u64;
            let module_kind = determine_module_kind(&module_path);

            // Add module to graph
            let module_id = {
                let mut graph = self.graph.write();
                graph.add_module(module_path.clone(), module_kind, file_size)
            };

            // Add edge from parent if exists
            if let (Some(parent), Some(kind)) = (parent_id, edge_kind) {
                let mut graph = self.graph.write();
                graph.add_dependency(parent, module_id, kind);
            }

            // Extract imports (only for script modules)
            if module_kind == graph::ModuleKind::Script {
                let is_typescript = module_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "ts" || e == "tsx")
                    .unwrap_or(false);

                let module_imports = match imports::extract_imports(&source, is_typescript) {
                    Ok(imports) => imports,
                    Err(e) => {
                        tracing::warn!("Failed to extract imports from {:?}: {}", module_path, e);
                        continue;
                    }
                };

                // Queue static imports
                for import_decl in &module_imports.static_imports {
                    match self.resolve_dependency(&module_path, &import_decl.source) {
                        Ok(resolved_path) => {
                            // Detect CSS files and use appropriate edge type
                            let edge_kind = if resolved_path.extension()
                                .and_then(|e| e.to_str())
                                .map(|e| e == "css" || e == "scss" || e == "sass" || e == "less")
                                .unwrap_or(false)
                            {
                                graph::EdgeKind::CssImport
                            } else {
                                graph::EdgeKind::Import
                            };

                            queue.push((resolved_path, Some(module_id), Some(edge_kind)));
                        }
                        Err(e) => {
                            // Track non-external failures
                            let err_msg = e.to_string();
                            if !err_msg.contains("External module") {
                                tracing::warn!(
                                    "Failed to resolve '{}' from {:?}: {}",
                                    import_decl.source,
                                    module_path,
                                    e
                                );
                            } else {
                                tracing::debug!("External module '{}' (not bundled)", import_decl.source);
                            }
                        }
                    }
                }

                // Queue dynamic imports
                for dynamic_import in &module_imports.dynamic_imports {
                    match self.resolve_dependency(&module_path, dynamic_import) {
                        Ok(resolved_path) => {
                            queue.push((
                                resolved_path,
                                Some(module_id),
                                Some(graph::EdgeKind::DynamicImport),
                            ));
                        }
                        Err(e) => {
                            let err_msg = e.to_string();
                            if !err_msg.contains("External module") {
                                tracing::warn!(
                                    "Failed to resolve '{}' from {:?}: {}",
                                    dynamic_import,
                                    module_path,
                                    e
                                );
                            } else {
                                tracing::debug!("External module '{}' (not bundled)", dynamic_import);
                            }
                        }
                    }
                }
            }
        }

        let graph = self.graph.read();
        let module_count = graph.module_count();

        // Check for cycles
        if graph.has_cycle() {
            tracing::warn!("Circular dependencies detected in module graph");
            if let Err(cycle_paths) = graph.topological_sort() {
                tracing::error!("Dependency cycle:");
                for (i, path) in cycle_paths.iter().enumerate() {
                    tracing::error!("  {} -> {:?}", i + 1, path);
                }
                return Err(anyhow::anyhow!(
                    "Circular dependency detected: {} modules in cycle",
                    cycle_paths.len()
                ));
            }
        }

        tracing::info!("Module graph built: {} modules", module_count);

        Ok(())
    }

    /// Resolve dependency path
    fn resolve_dependency(&self, from: &PathBuf, specifier: &str) -> Result<PathBuf> {
        let resolved = self.resolver.resolve(specifier, from)?;

        if resolved.is_external {
            tracing::debug!("Skipping external module: {}", specifier);
            return Err(anyhow::anyhow!("External module: {}", specifier));
        }

        Ok(std::fs::canonicalize(&resolved.path)?)
    }

    /// Transform all modules in the graph
    async fn transform_modules(&self) -> Result<Vec<CompiledModule>> {
        tracing::debug!("Transforming modules");

        let graph = self.graph.read();

        // Get modules in topological order
        let sorted_ids = graph.topological_sort()
            .map_err(|cycle_paths| {
                anyhow::anyhow!("Cannot transform modules with circular dependencies: {:?}", cycle_paths)
            })?;

        // Build module ID map for ES6 -> CommonJS transformation
        let module_map: std::collections::HashMap<PathBuf, usize> = sorted_ids
            .iter()
            .enumerate()
            .filter_map(|(idx, &id)| {
                let node = graph.get_node(id)?;
                Some((node.path.clone(), idx))
            })
            .collect();

        tracing::debug!("Built module map with {} entries", module_map.len());

        // Transform modules in parallel using rayon
        use rayon::prelude::*;

        let modules: Vec<CompiledModule> = sorted_ids
            .par_iter()
            .filter_map(|&id| {
                let node = graph.get_node(id)?;

                // Check cache first
                let metadata = std::fs::metadata(&node.path).ok()?;
                let mtime = metadata.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();

                if let Some(cached) = self.cache.get(&node.path, mtime) {
                    tracing::debug!("Using cached module: {:?}", node.path);
                    return Some(Ok(cached));
                }

                // Read source
                let source = std::fs::read_to_string(&node.path).ok()?;

                // Transform based on module kind
                let result = match node.kind {
                    graph::ModuleKind::Script => {
                        // Transform with module context (ES6 -> CommonJS)
                        self.transformer.transform_js_with_context(&source, &node.path, &module_map)
                    }
                    graph::ModuleKind::Css => {
                        // Transform CSS to JS injection code
                        self.transformer.transform_css(&source)
                    }
                    _ => {
                        tracing::debug!("Skipping unsupported module kind: {:?}", node.path);
                        return None;
                    }
                };

                match result {
                    Ok(transform_result) => {
                        let compiled = CompiledModule {
                            path: node.path.clone(),
                            code: transform_result.code.clone(),
                            source_map: transform_result.source_map.clone(),
                            dependencies: Vec::new(), // Will be filled from graph
                            hash: calculate_hash(&transform_result.code),
                        };

                        // Cache the result
                        self.cache.insert(node.path.clone(), mtime, compiled.clone());

                        tracing::debug!("Transformed module: {:?}", node.path);
                        Some(Ok(compiled))
                    }
                    Err(e) => {
                        tracing::error!("Failed to transform {:?}: {}", node.path, e);
                        Some(Err(e))
                    }
                }
            })
            .collect::<Result<Vec<_>>>()?;

        tracing::info!("Transformed {} modules", modules.len());

        Ok(modules)
    }

    /// Generate final bundle from compiled modules
    fn generate_bundle(&self, modules: Vec<CompiledModule>) -> Result<BundleOutput> {
        tracing::debug!("Generating bundle from {} modules", modules.len());

        if modules.is_empty() {
            return Ok(BundleOutput {
                code: String::new(),
                source_map: None,
                assets: Vec::new(),
            });
        }

        let mut bundle = String::new();

        // Add runtime code
        bundle.push_str(&generate_runtime());
        bundle.push_str("\n\n");

        // Add each module wrapped in the module system
        for (idx, module) in modules.iter().enumerate() {
            let module_id = idx;
            let module_path = module.path.to_string_lossy();

            bundle.push_str(&format!("// Module {}: {}\n", module_id, module_path));
            bundle.push_str(&format!("__talos__.define({}, function(require, module, exports) {{\n", module_id));
            bundle.push_str(&module.code);
            bundle.push_str("\n});\n\n");
        }

        // Add entry point execution
        bundle.push_str("// Execute entry point\n");
        bundle.push_str("__talos__.require(0);\n");

        Ok(BundleOutput {
            code: bundle,
            source_map: None, // TODO: Implement source map merging
            assets: Vec::new(),
        })
    }
}

impl CompilationCache {
    /// Create a new compilation cache
    pub fn new() -> Self {
        Self {
            module_cache: DashMap::new(),
        }
    }

    /// Get cached module if exists and not stale
    pub fn get(&self, path: &PathBuf, mtime: u64) -> Option<CompiledModule> {
        self.module_cache
            .get(&(path.clone(), mtime))
            .map(|entry| entry.clone())
    }

    /// Cache a compiled module
    pub fn insert(&self, path: PathBuf, mtime: u64, module: CompiledModule) {
        self.module_cache.insert((path, mtime), module);
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.module_cache.clear();
    }
}

impl Default for CompilationCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = CompilationCache::new();
        assert_eq!(cache.module_cache.len(), 0);
    }
}
