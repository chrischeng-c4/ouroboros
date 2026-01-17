//! Import resolution for cross-file type checking
//!
//! This module handles:
//! - Parsing import statements
//! - Resolving module paths
//! - Loading exported types from other modules

use std::collections::HashMap;
use std::path::PathBuf;

use super::ty::Type;

/// Represents an import statement
#[derive(Debug, Clone)]
pub enum Import {
    /// import module
    Module {
        module: String,
        alias: Option<String>,
    },
    /// from module import name
    FromModule {
        module: String,
        names: Vec<ImportedName>,
    },
    /// from module import *
    WildcardImport { module: String },
}

/// A single imported name with optional alias
#[derive(Debug, Clone)]
pub struct ImportedName {
    pub name: String,
    pub alias: Option<String>,
}

/// Module information including exported types
#[derive(Debug, Clone, Default)]
pub struct ModuleInfo {
    /// Module path (e.g., "collections.abc")
    pub path: String,
    /// File path if available
    pub file_path: Option<PathBuf>,
    /// Exported types (name -> type)
    pub exports: HashMap<String, Type>,
    /// Re-exports from other modules
    pub reexports: HashMap<String, String>, // name -> source module
    /// Is this a package (__init__.py)?
    pub is_package: bool,
}

impl ModuleInfo {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            file_path: None,
            exports: HashMap::new(),
            reexports: HashMap::new(),
            is_package: false,
        }
    }

    /// Get an exported type by name
    pub fn get_export(&self, name: &str) -> Option<&Type> {
        self.exports.get(name)
    }
}

/// Import resolver that manages module loading and type resolution
#[derive(Debug, Default)]
pub struct ImportResolver {
    /// Loaded modules
    modules: HashMap<String, ModuleInfo>,
    /// Search paths for modules
    search_paths: Vec<PathBuf>,
    /// Current working directory
    cwd: PathBuf,
}

impl ImportResolver {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            search_paths: vec![],
            cwd: PathBuf::new(),
        }
    }

    /// Set search paths for module resolution
    pub fn set_search_paths(&mut self, paths: Vec<PathBuf>) {
        self.search_paths = paths;
    }

    /// Set current working directory
    pub fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
    }

    /// Register a module's exports
    pub fn register_module(&mut self, module_path: &str, info: ModuleInfo) {
        self.modules.insert(module_path.to_string(), info);
    }

    /// Get a registered module
    pub fn get_module(&self, module_path: &str) -> Option<&ModuleInfo> {
        self.modules.get(module_path)
    }

    /// Resolve an import and return the types it brings into scope
    pub fn resolve_import(&self, import: &Import) -> HashMap<String, Type> {
        let mut result = HashMap::new();

        match import {
            Import::Module { module, alias } => {
                // For `import foo`, we don't directly import types
                // The module name becomes available for attribute access
                let name = alias.as_ref().unwrap_or(module);
                result.insert(
                    name.clone(),
                    Type::Instance {
                        name: format!("module:{}", module),
                        module: Some(module.clone()),
                        type_args: vec![],
                    },
                );
            }
            Import::FromModule { module, names } => {
                if let Some(module_info) = self.modules.get(module) {
                    for imported_name in names {
                        let local_name = imported_name
                            .alias
                            .as_ref()
                            .unwrap_or(&imported_name.name);

                        if let Some(ty) = module_info.exports.get(&imported_name.name) {
                            result.insert(local_name.clone(), ty.clone());
                        }
                    }
                }
            }
            Import::WildcardImport { module } => {
                if let Some(module_info) = self.modules.get(module) {
                    for (name, ty) in &module_info.exports {
                        // Skip private names
                        if !name.starts_with('_') {
                            result.insert(name.clone(), ty.clone());
                        }
                    }
                }
            }
        }

        result
    }

    /// Resolve a module path to a file path
    #[allow(dead_code)]
    pub fn resolve_module_path(&self, module_path: &str) -> Option<PathBuf> {
        let parts: Vec<&str> = module_path.split('.').collect();
        let relative_path = parts.join("/");

        // Check search paths
        for search_path in &self.search_paths {
            // Try as a package (__init__.py)
            let package_init = search_path.join(&relative_path).join("__init__.py");
            if package_init.exists() {
                return Some(package_init);
            }

            // Try as a module (.py)
            let module_file = search_path.join(format!("{}.py", relative_path));
            if module_file.exists() {
                return Some(module_file);
            }

            // Try as a stub (.pyi)
            let stub_file = search_path.join(format!("{}.pyi", relative_path));
            if stub_file.exists() {
                return Some(stub_file);
            }
        }

        // Check relative to cwd
        let package_init = self.cwd.join(&relative_path).join("__init__.py");
        if package_init.exists() {
            return Some(package_init);
        }

        let module_file = self.cwd.join(format!("{}.py", relative_path));
        if module_file.exists() {
            return Some(module_file);
        }

        None
    }
}

/// Parse an import statement from a tree-sitter node
#[allow(dead_code)]
pub fn parse_import(source: &str, node: &tree_sitter::Node) -> Option<Import> {
    let node_text = |n: &tree_sitter::Node| -> String {
        n.utf8_text(source.as_bytes())
            .unwrap_or("")
            .to_string()
    };

    match node.kind() {
        "import_statement" => {
            // import foo, bar as baz
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
                    let (module, alias) = if child.kind() == "aliased_import" {
                        let name = child.child_by_field_name("name").map(|n| node_text(&n));
                        let alias = child.child_by_field_name("alias").map(|n| node_text(&n));
                        (name.unwrap_or_default(), alias)
                    } else {
                        (node_text(&child), None)
                    };

                    return Some(Import::Module { module, alias });
                }
            }
            None
        }
        "import_from_statement" => {
            // from foo import bar, baz as qux
            let module = node
                .child_by_field_name("module_name")
                .map(|n| node_text(&n))
                .unwrap_or_default();

            // Check for wildcard import
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "wildcard_import" {
                    return Some(Import::WildcardImport { module });
                }
            }

            // Parse named imports
            let mut names = Vec::new();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "dotted_name" | "identifier" => {
                        names.push(ImportedName {
                            name: node_text(&child),
                            alias: None,
                        });
                    }
                    "aliased_import" => {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| node_text(&n))
                            .unwrap_or_default();
                        let alias = child.child_by_field_name("alias").map(|n| node_text(&n));
                        names.push(ImportedName { name, alias });
                    }
                    _ => {}
                }
            }

            if !names.is_empty() {
                Some(Import::FromModule { module, names })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Create module info for common Python builtins
#[allow(dead_code)]
pub fn create_builtins_module() -> ModuleInfo {
    let mut info = ModuleInfo::new("builtins");

    // Add common builtin types
    info.exports.insert("int".to_string(), Type::ClassType {
        name: "int".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("str".to_string(), Type::ClassType {
        name: "str".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("float".to_string(), Type::ClassType {
        name: "float".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("bool".to_string(), Type::ClassType {
        name: "bool".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("list".to_string(), Type::ClassType {
        name: "list".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("dict".to_string(), Type::ClassType {
        name: "dict".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("set".to_string(), Type::ClassType {
        name: "set".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("tuple".to_string(), Type::ClassType {
        name: "tuple".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("bytes".to_string(), Type::ClassType {
        name: "bytes".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("type".to_string(), Type::ClassType {
        name: "type".to_string(),
        module: Some("builtins".to_string()),
    });
    info.exports.insert("object".to_string(), Type::ClassType {
        name: "object".to_string(),
        module: Some("builtins".to_string()),
    });

    // Common functions
    info.exports.insert(
        "len".to_string(),
        Type::callable(vec![Type::Any], Type::Int),
    );
    info.exports.insert(
        "print".to_string(),
        Type::callable(vec![Type::Any], Type::None),
    );
    info.exports.insert(
        "range".to_string(),
        Type::callable(vec![Type::Int], Type::list(Type::Int)),
    );
    info.exports.insert(
        "enumerate".to_string(),
        Type::callable(
            vec![Type::list(Type::Unknown)],
            Type::list(Type::Tuple(vec![Type::Int, Type::Unknown])),
        ),
    );
    info.exports.insert(
        "zip".to_string(),
        Type::callable(
            vec![Type::list(Type::Unknown), Type::list(Type::Unknown)],
            Type::list(Type::Tuple(vec![Type::Unknown, Type::Unknown])),
        ),
    );

    info
}

/// Create module info for typing module
#[allow(dead_code)]
pub fn create_typing_module() -> ModuleInfo {
    let mut info = ModuleInfo::new("typing");

    // Generic type constructors
    info.exports.insert("List".to_string(), Type::ClassType {
        name: "list".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Dict".to_string(), Type::ClassType {
        name: "dict".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Set".to_string(), Type::ClassType {
        name: "set".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Tuple".to_string(), Type::ClassType {
        name: "tuple".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Optional".to_string(), Type::ClassType {
        name: "Optional".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Union".to_string(), Type::ClassType {
        name: "Union".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Callable".to_string(), Type::ClassType {
        name: "Callable".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Any".to_string(), Type::Any);
    info.exports.insert("TypeVar".to_string(), Type::ClassType {
        name: "TypeVar".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Generic".to_string(), Type::ClassType {
        name: "Generic".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Protocol".to_string(), Type::ClassType {
        name: "Protocol".to_string(),
        module: Some("typing".to_string()),
    });

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_registration() {
        let mut resolver = ImportResolver::new();

        let mut module_info = ModuleInfo::new("mymodule");
        module_info.exports.insert("MyClass".to_string(), Type::Instance {
            name: "MyClass".to_string(),
            module: Some("mymodule".to_string()),
            type_args: vec![],
        });

        resolver.register_module("mymodule", module_info);

        let module = resolver.get_module("mymodule");
        assert!(module.is_some());
        assert!(module.unwrap().exports.contains_key("MyClass"));
    }

    #[test]
    fn test_from_import_resolution() {
        let mut resolver = ImportResolver::new();

        let mut module_info = ModuleInfo::new("mymodule");
        module_info.exports.insert("foo".to_string(), Type::Int);
        module_info.exports.insert("bar".to_string(), Type::Str);

        resolver.register_module("mymodule", module_info);

        let import = Import::FromModule {
            module: "mymodule".to_string(),
            names: vec![
                ImportedName {
                    name: "foo".to_string(),
                    alias: None,
                },
                ImportedName {
                    name: "bar".to_string(),
                    alias: Some("baz".to_string()),
                },
            ],
        };

        let resolved = resolver.resolve_import(&import);
        assert_eq!(resolved.get("foo"), Some(&Type::Int));
        assert_eq!(resolved.get("baz"), Some(&Type::Str));
        assert!(!resolved.contains_key("bar")); // aliased, not available as "bar"
    }

    #[test]
    fn test_builtins_module() {
        let builtins = create_builtins_module();
        assert!(builtins.exports.contains_key("int"));
        assert!(builtins.exports.contains_key("str"));
        assert!(builtins.exports.contains_key("len"));
    }

    #[test]
    fn test_typing_module() {
        let typing = create_typing_module();
        assert!(typing.exports.contains_key("List"));
        assert!(typing.exports.contains_key("Optional"));
        assert!(typing.exports.contains_key("TypeVar"));
    }
}
