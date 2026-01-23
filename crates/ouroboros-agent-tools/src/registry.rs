use crate::error::{ToolError, ToolResult};
use crate::tool::Tool;
use dashmap::DashMap;
use std::sync::Arc;

/// Global tool registry (thread-safe)
/// Stores all registered tools by name
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&self, tool: Arc<dyn Tool>) -> ToolResult<()> {
        let name = tool.name().to_string();

        if self.tools.contains_key(&name) {
            return Err(ToolError::ValidationFailed(format!(
                "Tool '{}' is already registered",
                name
            )));
        }

        self.tools.insert(name, tool);
        Ok(())
    }

    /// Unregister a tool by name
    pub fn unregister(&self, name: &str) -> ToolResult<()> {
        self.tools
            .remove(name)
            .ok_or_else(|| ToolError::NotFound(format!("Tool '{}' not found", name)))?;
        Ok(())
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|entry| entry.value().clone())
    }

    /// Check if a tool is registered
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all registered tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get count of registered tools
    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// Clear all tools
    pub fn clear(&self) {
        self.tools.clear();
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global singleton tool registry
static GLOBAL_REGISTRY: once_cell::sync::Lazy<ToolRegistry> =
    once_cell::sync::Lazy::new(ToolRegistry::new);

/// Get the global tool registry
pub fn global_registry() -> &'static ToolRegistry {
    &GLOBAL_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{FunctionTool, ToolParameter};

    #[test]
    fn test_registry() {
        let registry = ToolRegistry::new();

        let tool = Arc::new(FunctionTool::new(
            "test",
            "Test tool",
            vec![],
            |_| Box::pin(async move { Ok(serde_json::json!({})) }),
        ));

        assert_eq!(registry.count(), 0);

        registry.register(tool.clone()).unwrap();
        assert_eq!(registry.count(), 1);
        assert!(registry.contains("test"));

        let retrieved = registry.get("test").unwrap();
        assert_eq!(retrieved.name(), "test");

        registry.unregister("test").unwrap();
        assert_eq!(registry.count(), 0);
    }
}
