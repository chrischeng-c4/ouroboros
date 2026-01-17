//! Stub file (.pyi) support for type information
//!
//! This module handles:
//! - Parsing .pyi stub files
//! - Loading standard library stubs
//! - Third-party stubs (typeshed)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::imports::ModuleInfo;
use super::ty::Type;

/// Stub file loader and cache
#[derive(Debug, Default)]
pub struct StubLoader {
    /// Loaded stubs (module path -> module info)
    stubs: HashMap<String, ModuleInfo>,
    /// Stub search paths (e.g., typeshed location)
    stub_paths: Vec<PathBuf>,
    /// Built-in stubs (preloaded)
    builtins_loaded: bool,
}

impl StubLoader {
    pub fn new() -> Self {
        Self {
            stubs: HashMap::new(),
            stub_paths: vec![],
            builtins_loaded: false,
        }
    }

    /// Set stub search paths
    pub fn set_stub_paths(&mut self, paths: Vec<PathBuf>) {
        self.stub_paths = paths;
    }

    /// Add a stub search path
    pub fn add_stub_path(&mut self, path: PathBuf) {
        if !self.stub_paths.contains(&path) {
            self.stub_paths.push(path);
        }
    }

    /// Load builtin stubs
    pub fn load_builtins(&mut self) {
        if self.builtins_loaded {
            return;
        }

        // Load core builtin types
        self.stubs.insert("builtins".to_string(), create_builtins_stub());
        self.stubs.insert("typing".to_string(), create_typing_stub());
        self.stubs.insert("collections".to_string(), create_collections_stub());
        self.stubs.insert("collections.abc".to_string(), create_collections_abc_stub());

        self.builtins_loaded = true;
    }

    /// Get stub for a module
    pub fn get_stub(&self, module_path: &str) -> Option<&ModuleInfo> {
        self.stubs.get(module_path)
    }

    /// Check if a stub exists for a module
    pub fn has_stub(&self, module_path: &str) -> bool {
        self.stubs.contains_key(module_path)
    }

    /// Try to load a stub file for a module
    #[allow(dead_code)]
    pub fn load_stub(&mut self, module_path: &str) -> Option<&ModuleInfo> {
        if self.stubs.contains_key(module_path) {
            return self.stubs.get(module_path);
        }

        // Try to find and load the stub
        if let Some(stub_path) = self.find_stub_file(module_path) {
            if let Some(info) = self.parse_stub_file(&stub_path) {
                self.stubs.insert(module_path.to_string(), info);
                return self.stubs.get(module_path);
            }
        }

        None
    }

    /// Find stub file for a module
    fn find_stub_file(&self, module_path: &str) -> Option<PathBuf> {
        let parts: Vec<&str> = module_path.split('.').collect();
        let relative_path = parts.join("/");

        for stub_path in &self.stub_paths {
            // Try as package stub
            let package_stub = stub_path.join(&relative_path).join("__init__.pyi");
            if package_stub.exists() {
                return Some(package_stub);
            }

            // Try as module stub
            let module_stub = stub_path.join(format!("{}.pyi", relative_path));
            if module_stub.exists() {
                return Some(module_stub);
            }
        }

        None
    }

    /// Parse a stub file (simplified - just structure)
    #[allow(dead_code)]
    fn parse_stub_file(&self, _path: &Path) -> Option<ModuleInfo> {
        // TODO: Implement full stub file parsing using tree-sitter
        // For now, return None to indicate stub parsing not implemented
        None
    }

    /// Get all loaded modules
    pub fn modules(&self) -> impl Iterator<Item = (&String, &ModuleInfo)> {
        self.stubs.iter()
    }
}

/// Create builtins stub
fn create_builtins_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("builtins");

    // Primitive type constructors
    let primitives = [
        ("int", Type::ClassType { name: "int".to_string(), module: Some("builtins".to_string()) }),
        ("float", Type::ClassType { name: "float".to_string(), module: Some("builtins".to_string()) }),
        ("str", Type::ClassType { name: "str".to_string(), module: Some("builtins".to_string()) }),
        ("bool", Type::ClassType { name: "bool".to_string(), module: Some("builtins".to_string()) }),
        ("bytes", Type::ClassType { name: "bytes".to_string(), module: Some("builtins".to_string()) }),
        ("bytearray", Type::ClassType { name: "bytearray".to_string(), module: Some("builtins".to_string()) }),
        ("object", Type::ClassType { name: "object".to_string(), module: Some("builtins".to_string()) }),
        ("type", Type::ClassType { name: "type".to_string(), module: Some("builtins".to_string()) }),
    ];

    for (name, ty) in primitives {
        info.exports.insert(name.to_string(), ty);
    }

    // Container types
    let containers = [
        ("list", Type::ClassType { name: "list".to_string(), module: Some("builtins".to_string()) }),
        ("dict", Type::ClassType { name: "dict".to_string(), module: Some("builtins".to_string()) }),
        ("set", Type::ClassType { name: "set".to_string(), module: Some("builtins".to_string()) }),
        ("frozenset", Type::ClassType { name: "frozenset".to_string(), module: Some("builtins".to_string()) }),
        ("tuple", Type::ClassType { name: "tuple".to_string(), module: Some("builtins".to_string()) }),
    ];

    for (name, ty) in containers {
        info.exports.insert(name.to_string(), ty);
    }

    // Common functions
    info.exports.insert("len".to_string(), Type::callable(vec![Type::Any], Type::Int));
    info.exports.insert("abs".to_string(), Type::callable(vec![Type::Any], Type::Int));
    info.exports.insert("min".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("max".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("sum".to_string(), Type::callable(vec![Type::Any], Type::Int));
    info.exports.insert("sorted".to_string(), Type::callable(vec![Type::Any], Type::list(Type::Unknown)));
    info.exports.insert("reversed".to_string(), Type::callable(vec![Type::Any], Type::list(Type::Unknown)));
    info.exports.insert("enumerate".to_string(), Type::callable(
        vec![Type::Any],
        Type::list(Type::Tuple(vec![Type::Int, Type::Unknown])),
    ));
    info.exports.insert("zip".to_string(), Type::callable(
        vec![Type::Any, Type::Any],
        Type::list(Type::Tuple(vec![Type::Unknown, Type::Unknown])),
    ));
    info.exports.insert("map".to_string(), Type::callable(
        vec![Type::Any, Type::Any],
        Type::list(Type::Unknown),
    ));
    info.exports.insert("filter".to_string(), Type::callable(
        vec![Type::Any, Type::Any],
        Type::list(Type::Unknown),
    ));
    info.exports.insert("range".to_string(), Type::callable(
        vec![Type::Int],
        Type::list(Type::Int),
    ));
    info.exports.insert("print".to_string(), Type::callable(vec![Type::Any], Type::None));
    info.exports.insert("input".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("open".to_string(), Type::callable(vec![Type::Str], Type::Any)); // Simplified
    info.exports.insert("isinstance".to_string(), Type::callable(vec![Type::Any, Type::Any], Type::Bool));
    info.exports.insert("issubclass".to_string(), Type::callable(vec![Type::Any, Type::Any], Type::Bool));
    info.exports.insert("hasattr".to_string(), Type::callable(vec![Type::Any, Type::Str], Type::Bool));
    info.exports.insert("getattr".to_string(), Type::callable(vec![Type::Any, Type::Str], Type::Any));
    info.exports.insert("setattr".to_string(), Type::callable(vec![Type::Any, Type::Str, Type::Any], Type::None));
    info.exports.insert("delattr".to_string(), Type::callable(vec![Type::Any, Type::Str], Type::None));
    info.exports.insert("callable".to_string(), Type::callable(vec![Type::Any], Type::Bool));
    info.exports.insert("repr".to_string(), Type::callable(vec![Type::Any], Type::Str));
    info.exports.insert("hash".to_string(), Type::callable(vec![Type::Any], Type::Int));
    info.exports.insert("id".to_string(), Type::callable(vec![Type::Any], Type::Int));
    info.exports.insert("iter".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("next".to_string(), Type::callable(vec![Type::Any], Type::Any));

    // Exception types
    let exceptions = [
        "Exception", "BaseException", "ValueError", "TypeError", "KeyError",
        "IndexError", "AttributeError", "RuntimeError", "StopIteration",
        "ImportError", "ModuleNotFoundError", "FileNotFoundError", "IOError",
        "OSError", "AssertionError", "ZeroDivisionError", "OverflowError",
        "NameError", "UnboundLocalError", "NotImplementedError",
    ];

    for exc in exceptions {
        info.exports.insert(exc.to_string(), Type::ClassType {
            name: exc.to_string(),
            module: Some("builtins".to_string()),
        });
    }

    // Constants
    info.exports.insert("None".to_string(), Type::None);
    info.exports.insert("True".to_string(), Type::Bool);
    info.exports.insert("False".to_string(), Type::Bool);
    info.exports.insert("Ellipsis".to_string(), Type::Any);
    info.exports.insert("NotImplemented".to_string(), Type::Any);

    info
}

/// Create typing module stub
fn create_typing_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("typing");

    // Type constructors
    let type_constructors = [
        "List", "Dict", "Set", "FrozenSet", "Tuple",
        "Optional", "Union", "Callable", "Type",
        "Sequence", "Mapping", "MutableMapping",
        "Iterable", "Iterator", "Generator",
        "Coroutine", "AsyncGenerator", "AsyncIterator", "AsyncIterable",
        "Awaitable", "ContextManager", "AsyncContextManager",
    ];

    for tc in type_constructors {
        info.exports.insert(tc.to_string(), Type::ClassType {
            name: tc.to_string(),
            module: Some("typing".to_string()),
        });
    }

    // Special forms
    info.exports.insert("Any".to_string(), Type::Any);
    info.exports.insert("NoReturn".to_string(), Type::Never);
    info.exports.insert("Never".to_string(), Type::Never);

    // TypeVar and related
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

    // Literal and Final
    info.exports.insert("Literal".to_string(), Type::ClassType {
        name: "Literal".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Final".to_string(), Type::ClassType {
        name: "Final".to_string(),
        module: Some("typing".to_string()),
    });

    // Other typing utilities
    info.exports.insert("ClassVar".to_string(), Type::ClassType {
        name: "ClassVar".to_string(),
        module: Some("typing".to_string()),
    });
    info.exports.insert("Self".to_string(), Type::SelfType);
    info.exports.insert("TypeAlias".to_string(), Type::ClassType {
        name: "TypeAlias".to_string(),
        module: Some("typing".to_string()),
    });

    // Functions
    info.exports.insert("cast".to_string(), Type::callable(
        vec![Type::Any, Type::Any],
        Type::Any,
    ));
    info.exports.insert("overload".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));
    info.exports.insert("no_type_check".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));
    info.exports.insert("get_type_hints".to_string(), Type::callable(
        vec![Type::Any],
        Type::dict(Type::Str, Type::Any),
    ));

    info
}

/// Create collections module stub
fn create_collections_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("collections");

    let types = [
        ("deque", Type::ClassType { name: "deque".to_string(), module: Some("collections".to_string()) }),
        ("defaultdict", Type::ClassType { name: "defaultdict".to_string(), module: Some("collections".to_string()) }),
        ("OrderedDict", Type::ClassType { name: "OrderedDict".to_string(), module: Some("collections".to_string()) }),
        ("Counter", Type::ClassType { name: "Counter".to_string(), module: Some("collections".to_string()) }),
        ("ChainMap", Type::ClassType { name: "ChainMap".to_string(), module: Some("collections".to_string()) }),
        ("namedtuple", Type::ClassType { name: "namedtuple".to_string(), module: Some("collections".to_string()) }),
    ];

    for (name, ty) in types {
        info.exports.insert(name.to_string(), ty);
    }

    info
}

/// Create collections.abc module stub
fn create_collections_abc_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("collections.abc");

    let abstract_types = [
        "Awaitable", "Coroutine", "AsyncIterable", "AsyncIterator", "AsyncGenerator",
        "Hashable", "Iterable", "Iterator", "Generator", "Reversible",
        "Container", "Collection", "Callable", "Set", "MutableSet",
        "Mapping", "MutableMapping", "MappingView", "KeysView", "ItemsView", "ValuesView",
        "Sequence", "MutableSequence", "ByteString",
    ];

    for name in abstract_types {
        info.exports.insert(name.to_string(), Type::ClassType {
            name: name.to_string(),
            module: Some("collections.abc".to_string()),
        });
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_loader_builtins() {
        let mut loader = StubLoader::new();
        loader.load_builtins();

        assert!(loader.has_stub("builtins"));
        assert!(loader.has_stub("typing"));

        let builtins = loader.get_stub("builtins").unwrap();
        assert!(builtins.exports.contains_key("int"));
        assert!(builtins.exports.contains_key("str"));
        assert!(builtins.exports.contains_key("len"));
        assert!(builtins.exports.contains_key("Exception"));
    }

    #[test]
    fn test_typing_stub() {
        let mut loader = StubLoader::new();
        loader.load_builtins();

        let typing = loader.get_stub("typing").unwrap();
        assert!(typing.exports.contains_key("List"));
        assert!(typing.exports.contains_key("Optional"));
        assert!(typing.exports.contains_key("TypeVar"));
        assert!(typing.exports.contains_key("Protocol"));
        assert!(typing.exports.contains_key("Any"));
    }

    #[test]
    fn test_collections_stub() {
        let mut loader = StubLoader::new();
        loader.load_builtins();

        let collections = loader.get_stub("collections").unwrap();
        assert!(collections.exports.contains_key("deque"));
        assert!(collections.exports.contains_key("defaultdict"));
        assert!(collections.exports.contains_key("Counter"));
    }

    #[test]
    fn test_stub_path_management() {
        let mut loader = StubLoader::new();

        loader.add_stub_path(PathBuf::from("/usr/lib/python3/stubs"));
        loader.add_stub_path(PathBuf::from("/home/user/.stubs"));

        assert_eq!(loader.stub_paths.len(), 2);

        // Adding same path again shouldn't duplicate
        loader.add_stub_path(PathBuf::from("/usr/lib/python3/stubs"));
        assert_eq!(loader.stub_paths.len(), 2);
    }
}
