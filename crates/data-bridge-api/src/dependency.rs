//! Dependency Injection Container
//!
//! Provides a flexible DI system with:
//! - Topological dependency resolution
//! - Multiple scopes (transient, request, singleton)
//! - Cycle detection
//! - Async support

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::any::Any;

use crate::error::{ApiError, ApiResult};

/// Dependency scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DependencyScope {
    /// New instance every call
    Transient,
    /// Cached per request
    #[default]
    Request,
    /// Singleton for app lifetime
    Singleton,
}

/// Dependency descriptor
#[derive(Clone)]
pub struct DependencyDescriptor {
    /// Unique dependency ID
    pub id: String,
    /// Dependencies this depends on
    pub depends_on: Vec<String>,
    /// Dependency scope
    pub scope: DependencyScope,
    // Factory function (stored externally in Python)
    // This is just metadata; actual factory is called via Python
}

impl DependencyDescriptor {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            depends_on: Vec::new(),
            scope: DependencyScope::Request,
        }
    }

    pub fn depends_on(mut self, dep: impl Into<String>) -> Self {
        self.depends_on.push(dep.into());
        self
    }

    pub fn scope(mut self, scope: DependencyScope) -> Self {
        self.scope = scope;
        self
    }
}

/// Cached dependency value
#[derive(Clone)]
pub struct CachedValue {
    pub value: Arc<dyn Any + Send + Sync>,
    pub type_name: String,
}

/// Request-scoped cache
pub struct RequestScope {
    cache: HashMap<String, CachedValue>,
}

impl RequestScope {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<&CachedValue> {
        self.cache.get(id)
    }

    pub fn set(&mut self, id: String, value: CachedValue) {
        self.cache.insert(id, value);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for RequestScope {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency Injection Container
pub struct DependencyContainer {
    /// Registered dependencies
    dependencies: HashMap<String, DependencyDescriptor>,
    /// Singleton cache
    singletons: RwLock<HashMap<String, CachedValue>>,
    /// Resolution order (topologically sorted)
    resolution_order: Vec<String>,
    /// Whether the container has been compiled
    compiled: bool,
}

impl DependencyContainer {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            singletons: RwLock::new(HashMap::new()),
            resolution_order: Vec::new(),
            compiled: false,
        }
    }

    /// Register a dependency
    pub fn register(&mut self, descriptor: DependencyDescriptor) -> ApiResult<()> {
        if self.compiled {
            return Err(ApiError::Internal(
                "Cannot register dependencies after container is compiled".to_string()
            ));
        }

        self.dependencies.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    /// Compile the container (compute resolution order)
    pub fn compile(&mut self) -> ApiResult<()> {
        if self.compiled {
            return Ok(());
        }

        // Detect cycles and compute topological order
        self.resolution_order = self.topological_sort()?;
        self.compiled = true;
        Ok(())
    }

    /// Topological sort with cycle detection (Kahn's algorithm)
    fn topological_sort(&self) -> ApiResult<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize
        for (id, desc) in &self.dependencies {
            in_degree.entry(id.as_str()).or_insert(0);
            graph.entry(id.as_str()).or_default();

            for dep in &desc.depends_on {
                if !self.dependencies.contains_key(dep.as_str()) {
                    return Err(ApiError::Internal(format!(
                        "Dependency '{}' required by '{}' is not registered",
                        dep, id
                    )));
                }
                graph.entry(dep.as_str()).or_default().push(id.as_str());
                *in_degree.entry(id.as_str()).or_insert(0) += 1;
            }
        }

        // Start with nodes that have no dependencies
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result: Vec<String> = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node.to_string());

            if let Some(neighbors) = graph.get(node) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.dependencies.len() {
            let remaining: Vec<&str> = self.dependencies
                .keys()
                .map(|s| s.as_str())
                .filter(|id| !result.iter().any(|r| r == *id))
                .collect();
            return Err(ApiError::Internal(format!(
                "Circular dependency detected involving: {:?}",
                remaining
            )));
        }

        Ok(result)
    }

    /// Get resolution order for specific dependencies
    pub fn get_resolution_order(&self, required: &[String]) -> ApiResult<Vec<String>> {
        if !self.compiled {
            return Err(ApiError::Internal(
                "Container must be compiled before resolution".to_string()
            ));
        }

        // Find all dependencies (transitive)
        let mut needed: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = required.to_vec();

        while let Some(id) = stack.pop() {
            if needed.contains(&id) {
                continue;
            }
            needed.insert(id.clone());

            if let Some(desc) = self.dependencies.get(&id) {
                for dep in &desc.depends_on {
                    stack.push(dep.to_string());
                }
            }
        }

        // Filter and return in topological order
        Ok(self.resolution_order
            .iter()
            .filter(|id| needed.contains(*id))
            .cloned()
            .collect())
    }

    /// Get dependency descriptor
    pub fn get(&self, id: &str) -> Option<&DependencyDescriptor> {
        self.dependencies.get(id)
    }

    /// Check if singleton is cached
    pub fn get_singleton(&self, id: &str) -> Option<CachedValue> {
        self.singletons.read().ok()?.get(id).cloned()
    }

    /// Cache singleton value
    pub fn set_singleton(&self, id: String, value: CachedValue) -> ApiResult<()> {
        self.singletons
            .write()
            .map_err(|e| ApiError::Internal(format!("Lock error: {}", e)))?
            .insert(id, value);
        Ok(())
    }

    /// Get all dependency IDs
    pub fn dependency_ids(&self) -> Vec<String> {
        self.dependencies.keys().cloned().collect()
    }

    /// Check if container is compiled
    pub fn is_compiled(&self) -> bool {
        self.compiled
    }
}

impl Default for DependencyContainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency resolution context
pub struct ResolutionContext {
    /// Request-scoped cache
    pub request_scope: RequestScope,
    /// Resolved values in current resolution
    pub resolved: HashMap<String, CachedValue>,
}

impl ResolutionContext {
    pub fn new() -> Self {
        Self {
            request_scope: RequestScope::new(),
            resolved: HashMap::new(),
        }
    }

    pub fn with_request_scope(request_scope: RequestScope) -> Self {
        Self {
            request_scope,
            resolved: HashMap::new(),
        }
    }
}

impl Default for ResolutionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_registration() {
        let mut container = DependencyContainer::new();

        container.register(DependencyDescriptor::new("db")).unwrap();
        container.register(
            DependencyDescriptor::new("user_service")
                .depends_on("db")
        ).unwrap();

        assert!(container.get("db").is_some());
        assert!(container.get("user_service").is_some());
    }

    #[test]
    fn test_topological_sort() {
        let mut container = DependencyContainer::new();

        // A -> B -> C
        container.register(DependencyDescriptor::new("c")).unwrap();
        container.register(
            DependencyDescriptor::new("b").depends_on("c")
        ).unwrap();
        container.register(
            DependencyDescriptor::new("a").depends_on("b")
        ).unwrap();

        container.compile().unwrap();

        let order = &container.resolution_order;
        let pos_a = order.iter().position(|x| x == "a").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();

        // c must come before b, b must come before a
        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn test_cycle_detection() {
        let mut container = DependencyContainer::new();

        // A -> B -> C -> A (cycle)
        container.register(
            DependencyDescriptor::new("a").depends_on("b")
        ).unwrap();
        container.register(
            DependencyDescriptor::new("b").depends_on("c")
        ).unwrap();
        container.register(
            DependencyDescriptor::new("c").depends_on("a")
        ).unwrap();

        let result = container.compile();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_missing_dependency() {
        let mut container = DependencyContainer::new();

        container.register(
            DependencyDescriptor::new("a").depends_on("nonexistent")
        ).unwrap();

        let result = container.compile();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not registered"));
    }

    #[test]
    fn test_resolution_order() {
        let mut container = DependencyContainer::new();

        container.register(DependencyDescriptor::new("config")).unwrap();
        container.register(
            DependencyDescriptor::new("db").depends_on("config")
        ).unwrap();
        container.register(
            DependencyDescriptor::new("cache").depends_on("config")
        ).unwrap();
        container.register(
            DependencyDescriptor::new("user_service")
                .depends_on("db")
                .depends_on("cache")
        ).unwrap();

        container.compile().unwrap();

        let order = container.get_resolution_order(&["user_service".to_string()]).unwrap();

        // All dependencies should be included
        assert!(order.contains(&"config".to_string()));
        assert!(order.contains(&"db".to_string()));
        assert!(order.contains(&"cache".to_string()));
        assert!(order.contains(&"user_service".to_string()));

        // config must come first
        assert_eq!(order[0], "config");
    }

    #[test]
    fn test_scopes() {
        let desc = DependencyDescriptor::new("test")
            .scope(DependencyScope::Singleton);

        assert_eq!(desc.scope, DependencyScope::Singleton);
    }

    #[test]
    fn test_request_scope_cache() {
        let mut scope = RequestScope::new();

        let value = CachedValue {
            value: Arc::new(42i32),
            type_name: "i32".to_string(),
        };

        scope.set("test".to_string(), value);

        let cached = scope.get("test").unwrap();
        let val = cached.value.downcast_ref::<i32>().unwrap();
        assert_eq!(*val, 42);
    }

    #[test]
    fn test_singleton_cache() {
        let container = DependencyContainer::new();

        let value = CachedValue {
            value: Arc::new("singleton_value".to_string()),
            type_name: "String".to_string(),
        };

        container.set_singleton("test".to_string(), value).unwrap();

        let cached = container.get_singleton("test").unwrap();
        let val = cached.value.downcast_ref::<String>().unwrap();
        assert_eq!(val, "singleton_value");
    }

    #[test]
    fn test_complex_dependency_graph() {
        let mut container = DependencyContainer::new();

        // Diamond dependency
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        container.register(DependencyDescriptor::new("d")).unwrap();
        container.register(
            DependencyDescriptor::new("b").depends_on("d")
        ).unwrap();
        container.register(
            DependencyDescriptor::new("c").depends_on("d")
        ).unwrap();
        container.register(
            DependencyDescriptor::new("a")
                .depends_on("b")
                .depends_on("c")
        ).unwrap();

        container.compile().unwrap();

        let order = container.get_resolution_order(&["a".to_string()]).unwrap();

        // D must come before B and C
        let pos_d = order.iter().position(|x| x == "d").unwrap();
        let pos_b = order.iter().position(|x| x == "b").unwrap();
        let pos_c = order.iter().position(|x| x == "c").unwrap();
        let pos_a = order.iter().position(|x| x == "a").unwrap();

        assert!(pos_d < pos_b);
        assert!(pos_d < pos_c);
        assert!(pos_b < pos_a);
        assert!(pos_c < pos_a);
    }

    #[test]
    fn test_cannot_register_after_compile() {
        let mut container = DependencyContainer::new();
        container.register(DependencyDescriptor::new("a")).unwrap();
        container.compile().unwrap();

        let result = container.register(DependencyDescriptor::new("b"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("after container is compiled"));
    }

    #[test]
    fn test_resolution_context() {
        let ctx = ResolutionContext::new();
        assert!(ctx.resolved.is_empty());
        assert!(ctx.request_scope.cache.is_empty());

        let scope = RequestScope::new();
        let ctx2 = ResolutionContext::with_request_scope(scope);
        assert!(ctx2.resolved.is_empty());
    }
}
