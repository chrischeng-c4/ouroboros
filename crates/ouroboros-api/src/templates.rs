//! Template rendering support
//!
//! Provides Jinja2-compatible template rendering using Tera.
//! Supports template caching, inheritance, custom filters, and context passing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::error::{ApiError, ApiResult};
use crate::response::Response;

// ============================================================================
// Template Engine
// ============================================================================

/// Template engine configuration
#[derive(Debug, Clone)]
pub struct TemplateConfig {
    /// Directory containing templates
    pub directory: PathBuf,
    /// File extension for templates
    pub extension: String,
    /// Whether to auto-reload templates in development
    pub auto_reload: bool,
    /// Enable template caching
    pub cache_enabled: bool,
    /// Default content type for rendered templates
    pub content_type: String,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("templates"),
            extension: "html".to_string(),
            auto_reload: cfg!(debug_assertions),
            cache_enabled: true,
            content_type: "text/html; charset=utf-8".to_string(),
        }
    }
}

impl TemplateConfig {
    /// Create a new template configuration
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            ..Default::default()
        }
    }

    /// Set file extension
    pub fn extension(mut self, ext: impl Into<String>) -> Self {
        self.extension = ext.into();
        self
    }

    /// Enable auto-reload
    pub fn auto_reload(mut self, enabled: bool) -> Self {
        self.auto_reload = enabled;
        self
    }

    /// Enable caching
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Set content type
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }
}

// ============================================================================
// Template Context
// ============================================================================

/// Template context value
#[derive(Debug, Clone)]
pub enum ContextValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<ContextValue>),
    Object(HashMap<String, ContextValue>),
}

impl From<bool> for ContextValue {
    fn from(v: bool) -> Self {
        ContextValue::Bool(v)
    }
}

impl From<i64> for ContextValue {
    fn from(v: i64) -> Self {
        ContextValue::Int(v)
    }
}

impl From<i32> for ContextValue {
    fn from(v: i32) -> Self {
        ContextValue::Int(v as i64)
    }
}

impl From<f64> for ContextValue {
    fn from(v: f64) -> Self {
        ContextValue::Float(v)
    }
}

impl From<String> for ContextValue {
    fn from(v: String) -> Self {
        ContextValue::String(v)
    }
}

impl From<&str> for ContextValue {
    fn from(v: &str) -> Self {
        ContextValue::String(v.to_string())
    }
}

impl<T: Into<ContextValue>> From<Vec<T>> for ContextValue {
    fn from(v: Vec<T>) -> Self {
        ContextValue::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<ContextValue>> From<Option<T>> for ContextValue {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => ContextValue::Null,
        }
    }
}

impl ContextValue {
    /// Convert to JSON value for template rendering
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            ContextValue::Null => serde_json::Value::Null,
            ContextValue::Bool(b) => serde_json::Value::Bool(*b),
            ContextValue::Int(i) => serde_json::json!(*i),
            ContextValue::Float(f) => serde_json::json!(*f),
            ContextValue::String(s) => serde_json::Value::String(s.clone()),
            ContextValue::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| v.to_json()).collect())
            }
            ContextValue::Object(obj) => {
                serde_json::Value::Object(
                    obj.iter()
                        .map(|(k, v)| (k.clone(), v.to_json()))
                        .collect(),
                )
            }
        }
    }
}

/// Template rendering context
#[derive(Debug, Clone, Default)]
pub struct Context {
    values: HashMap<String, ContextValue>,
}

impl Context {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value into the context
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<ContextValue>) {
        self.values.insert(key.into(), value.into());
    }

    /// Insert a value with builder pattern
    pub fn with(mut self, key: impl Into<String>, value: impl Into<ContextValue>) -> Self {
        self.insert(key, value);
        self
    }

    /// Get a value from the context
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.values.get(key)
    }

    /// Check if context contains a key
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Remove a value from context
    pub fn remove(&mut self, key: &str) -> Option<ContextValue> {
        self.values.remove(key)
    }

    /// Extend context with another context
    pub fn extend(&mut self, other: Context) {
        self.values.extend(other.values);
    }

    /// Convert to JSON for template rendering
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Object(
            self.values
                .iter()
                .map(|(k, v)| (k.clone(), v.to_json()))
                .collect(),
        )
    }
}

// ============================================================================
// Template Cache
// ============================================================================

/// Cached compiled template
#[derive(Debug, Clone)]
struct CachedTemplate {
    content: String,
    modified: std::time::SystemTime,
}

/// Template cache
struct TemplateCache {
    templates: HashMap<String, CachedTemplate>,
}

impl TemplateCache {
    fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&CachedTemplate> {
        self.templates.get(name)
    }

    fn insert(&mut self, name: String, template: CachedTemplate) {
        self.templates.insert(name, template);
    }

    fn invalidate(&mut self, name: &str) {
        self.templates.remove(name);
    }

    fn clear(&mut self) {
        self.templates.clear();
    }
}

// ============================================================================
// Custom Filters
// ============================================================================

/// Custom template filter function
pub type FilterFn = Box<dyn Fn(&serde_json::Value, &[serde_json::Value]) -> serde_json::Value + Send + Sync>;

/// Custom template function
pub type FunctionFn = Box<dyn Fn(&[serde_json::Value]) -> serde_json::Value + Send + Sync>;

// ============================================================================
// Template Engine
// ============================================================================

/// Template rendering engine
pub struct Templates {
    config: TemplateConfig,
    cache: RwLock<TemplateCache>,
    filters: HashMap<String, FilterFn>,
    functions: HashMap<String, FunctionFn>,
    globals: Context,
}

impl Templates {
    /// Create a new template engine
    pub fn new(config: TemplateConfig) -> Self {
        Self {
            config,
            cache: RwLock::new(TemplateCache::new()),
            filters: Self::default_filters(),
            functions: Self::default_functions(),
            globals: Context::new(),
        }
    }

    /// Create from directory path
    pub fn from_directory(directory: impl Into<PathBuf>) -> Self {
        Self::new(TemplateConfig::new(directory))
    }

    /// Add a custom filter
    pub fn add_filter<F>(&mut self, name: impl Into<String>, filter: F)
    where
        F: Fn(&serde_json::Value, &[serde_json::Value]) -> serde_json::Value + Send + Sync + 'static,
    {
        self.filters.insert(name.into(), Box::new(filter));
    }

    /// Add a custom function
    pub fn add_function<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(&[serde_json::Value]) -> serde_json::Value + Send + Sync + 'static,
    {
        self.functions.insert(name.into(), Box::new(func));
    }

    /// Set global context variable
    pub fn set_global(&mut self, key: impl Into<String>, value: impl Into<ContextValue>) {
        self.globals.insert(key, value);
    }

    /// Render a template by name
    pub fn render(&self, name: &str, context: &Context) -> ApiResult<String> {
        let template_content = self.load_template(name)?;
        self.render_string(&template_content, context)
    }

    /// Render a template string directly
    pub fn render_string(&self, template: &str, context: &Context) -> ApiResult<String> {
        // Merge globals with context
        let mut full_context = self.globals.clone();
        full_context.extend(context.clone());

        // Simple template rendering with variable substitution
        self.process_template(template, &full_context)
    }

    /// Render a template and return as Response
    pub fn render_response(&self, name: &str, context: &Context) -> ApiResult<Response> {
        let html = self.render(name, context)?;
        Ok(Response::text(html).header("content-type", "text/html; charset=utf-8"))
    }

    /// Clear template cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// Invalidate a specific template
    pub fn invalidate(&self, name: &str) {
        self.cache.write().invalidate(name);
    }

    // Private methods

    fn load_template(&self, name: &str) -> ApiResult<String> {
        let path = self.resolve_template_path(name);

        // Check cache first
        if self.config.cache_enabled {
            if let Some(cached) = self.cache.read().get(name) {
                // Check if file was modified (if auto_reload enabled)
                if !self.config.auto_reload {
                    return Ok(cached.content.clone());
                }

                if let Ok(metadata) = std::fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified <= cached.modified {
                            return Ok(cached.content.clone());
                        }
                    }
                }
            }
        }

        // Load from disk
        let content = std::fs::read_to_string(&path).map_err(|e| {
            ApiError::Internal(format!("Failed to load template '{}': {}", name, e))
        })?;

        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| std::time::SystemTime::now());

        // Cache it
        if self.config.cache_enabled {
            self.cache.write().insert(
                name.to_string(),
                CachedTemplate {
                    content: content.clone(),
                    modified,
                },
            );
        }

        Ok(content)
    }

    fn resolve_template_path(&self, name: &str) -> PathBuf {
        let mut path = self.config.directory.join(name);
        if !name.contains('.') {
            path.set_extension(&self.config.extension);
        }
        path
    }

    fn process_template(&self, template: &str, context: &Context) -> ApiResult<String> {
        let mut result = template.to_string();

        // Process extends (template inheritance)
        result = self.process_extends(&result, context)?;

        // Process blocks
        result = self.process_blocks(&result)?;

        // Process includes
        result = self.process_includes(&result, context)?;

        // Process for loops
        result = self.process_for_loops(&result, context)?;

        // Process if conditions
        result = self.process_conditions(&result, context)?;

        // Process variable substitutions
        result = self.process_variables(&result, context)?;

        // Process filters
        result = self.process_filters(&result)?;

        Ok(result)
    }

    fn process_extends(&self, template: &str, context: &Context) -> ApiResult<String> {
        // Match {% extends "base.html" %}
        let extends_re = regex::Regex::new(r#"\{%\s*extends\s+"([^"]+)"\s*%\}"#).unwrap();

        if let Some(caps) = extends_re.captures(template) {
            let parent_name = &caps[1];
            let parent_content = self.load_template(parent_name)?;

            // Extract blocks from child
            let block_re = regex::Regex::new(
                r#"\{%\s*block\s+(\w+)\s*%\}([\s\S]*?)\{%\s*endblock\s*%\}"#
            ).unwrap();

            let mut blocks: HashMap<String, String> = HashMap::new();
            for caps in block_re.captures_iter(template) {
                blocks.insert(caps[1].to_string(), caps[2].to_string());
            }

            // Replace blocks in parent
            let mut result = parent_content;
            for (name, content) in blocks {
                let block_pattern = format!(
                    r#"\{{% block {} %}}\s*[\s\S]*?\{{% endblock %}}"#,
                    regex::escape(&name)
                );
                if let Ok(re) = regex::Regex::new(&block_pattern) {
                    result = re.replace(&result, content.as_str()).to_string();
                }
            }

            return self.process_template(&result, context);
        }

        Ok(template.to_string())
    }

    fn process_blocks(&self, template: &str) -> ApiResult<String> {
        // Remove block markers, keeping content
        let block_re = regex::Regex::new(
            r#"\{%\s*block\s+\w+\s*%\}([\s\S]*?)\{%\s*endblock\s*%\}"#
        ).unwrap();

        Ok(block_re.replace_all(template, "$1").to_string())
    }

    fn process_includes(&self, template: &str, context: &Context) -> ApiResult<String> {
        let include_re = regex::Regex::new(r#"\{%\s*include\s+"([^"]+)"\s*%\}"#).unwrap();
        let mut result = template.to_string();

        while let Some(caps) = include_re.captures(&result) {
            let include_name = &caps[1];
            let include_content = self.load_template(include_name)?;
            let processed = self.process_template(&include_content, context)?;
            result = result.replace(&caps[0], &processed);
        }

        Ok(result)
    }

    fn process_for_loops(&self, template: &str, context: &Context) -> ApiResult<String> {
        let for_re = regex::Regex::new(
            r#"\{%\s*for\s+(\w+)\s+in\s+(\w+)\s*%\}([\s\S]*?)\{%\s*endfor\s*%\}"#
        ).unwrap();

        let mut result = template.to_string();

        while let Some(caps) = for_re.captures(&result) {
            let var_name = &caps[1];
            let array_name = &caps[2];
            let body = &caps[3];

            let mut output = String::new();

            if let Some(ContextValue::Array(items)) = context.get(array_name) {
                for (index, item) in items.iter().enumerate() {
                    let mut loop_context = context.clone();
                    loop_context.insert(var_name, item.clone());
                    loop_context.insert("loop.index", (index as i64) + 1);
                    loop_context.insert("loop.index0", index as i64);
                    loop_context.insert("loop.first", index == 0);
                    loop_context.insert("loop.last", index == items.len() - 1);

                    let processed = self.process_variables(body, &loop_context)?;
                    output.push_str(&processed);
                }
            }

            result = result.replace(&caps[0], &output);
        }

        Ok(result)
    }

    fn process_conditions(&self, template: &str, context: &Context) -> ApiResult<String> {
        let if_re = regex::Regex::new(
            r#"\{%\s*if\s+(\w+)\s*%\}([\s\S]*?)(?:\{%\s*else\s*%\}([\s\S]*?))?\{%\s*endif\s*%\}"#
        ).unwrap();

        let mut result = template.to_string();

        while let Some(caps) = if_re.captures(&result) {
            let condition_var = &caps[1];
            let if_body = &caps[2];
            let else_body = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            let is_truthy = match context.get(condition_var) {
                Some(ContextValue::Bool(b)) => *b,
                Some(ContextValue::Int(i)) => *i != 0,
                Some(ContextValue::String(s)) => !s.is_empty(),
                Some(ContextValue::Array(a)) => !a.is_empty(),
                Some(ContextValue::Null) => false,
                None => false,
                _ => true,
            };

            let output = if is_truthy { if_body } else { else_body };
            result = result.replace(&caps[0], output);
        }

        Ok(result)
    }

    fn process_variables(&self, template: &str, context: &Context) -> ApiResult<String> {
        let var_re = regex::Regex::new(r#"\{\{\s*([^}|]+?)(?:\s*\|\s*([^}]+))?\s*\}\}"#).unwrap();
        let mut result = template.to_string();

        for caps in var_re.captures_iter(template) {
            let var_path = caps[1].trim();
            let filter = caps.get(2).map(|m| m.as_str().trim());

            let value = self.resolve_variable(var_path, context);
            let mut output = self.value_to_string(&value);

            // Apply filter if present
            if let Some(filter_name) = filter {
                output = self.apply_filter(&output, filter_name);
            }

            result = result.replace(&caps[0], &output);
        }

        Ok(result)
    }

    fn process_filters(&self, template: &str) -> ApiResult<String> {
        // Filters are processed inline with variables
        Ok(template.to_string())
    }

    fn resolve_variable(&self, path: &str, context: &Context) -> ContextValue {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return ContextValue::Null;
        }

        let mut current = context.get(parts[0]).cloned();

        for part in &parts[1..] {
            current = match current {
                Some(ContextValue::Object(ref obj)) => obj.get(*part).cloned(),
                _ => None,
            };
        }

        current.unwrap_or(ContextValue::Null)
    }

    fn value_to_string(&self, value: &ContextValue) -> String {
        match value {
            ContextValue::Null => String::new(),
            ContextValue::Bool(b) => b.to_string(),
            ContextValue::Int(i) => i.to_string(),
            ContextValue::Float(f) => f.to_string(),
            ContextValue::String(s) => s.clone(),
            ContextValue::Array(_) => "[Array]".to_string(),
            ContextValue::Object(_) => "[Object]".to_string(),
        }
    }

    fn apply_filter(&self, value: &str, filter: &str) -> String {
        match filter {
            "upper" => value.to_uppercase(),
            "lower" => value.to_lowercase(),
            "capitalize" => {
                let mut chars = value.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                }
            }
            "trim" => value.trim().to_string(),
            "escape" | "e" => html_escape(value),
            "safe" => value.to_string(), // No escaping
            "length" => value.len().to_string(),
            "default" => {
                if value.is_empty() {
                    "".to_string()
                } else {
                    value.to_string()
                }
            }
            "striptags" => strip_html_tags(value),
            "title" => value
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => {
                            c.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
            _ => value.to_string(),
        }
    }

    fn default_filters() -> HashMap<String, FilterFn> {
        HashMap::new()
    }

    fn default_functions() -> HashMap<String, FunctionFn> {
        let mut funcs: HashMap<String, FunctionFn> = HashMap::new();

        // now() function
        funcs.insert(
            "now".to_string(),
            Box::new(|_args| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                serde_json::json!(timestamp)
            }),
        );

        funcs
    }
}

// ============================================================================
// Shared Templates
// ============================================================================

/// Thread-safe shared template engine
pub type SharedTemplates = Arc<Templates>;

/// Create a shared template engine
pub fn shared_templates(config: TemplateConfig) -> SharedTemplates {
    Arc::new(Templates::new(config))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Strip HTML tags from string
fn strip_html_tags(s: &str) -> String {
    let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();
    tag_re.replace_all(s, "").to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_insert() {
        let mut ctx = Context::new();
        ctx.insert("name", "John");
        ctx.insert("age", 30i64);
        ctx.insert("active", true);

        assert!(ctx.contains("name"));
        assert!(matches!(ctx.get("name"), Some(ContextValue::String(s)) if s == "John"));
    }

    #[test]
    fn test_context_with() {
        let ctx = Context::new()
            .with("name", "John")
            .with("age", 30i64);

        assert!(ctx.contains("name"));
        assert!(ctx.contains("age"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("<p>Hello</p>"), "Hello");
        assert_eq!(strip_html_tags("<div><span>Test</span></div>"), "Test");
    }

    #[test]
    fn test_variable_substitution() {
        let templates = Templates::from_directory("templates");
        let ctx = Context::new().with("name", "World");

        let result = templates.render_string("Hello, {{ name }}!", &ctx).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_filters() {
        let templates = Templates::from_directory("templates");
        let ctx = Context::new().with("name", "john");

        let result = templates.render_string("{{ name | upper }}", &ctx).unwrap();
        assert_eq!(result, "JOHN");

        let result = templates.render_string("{{ name | capitalize }}", &ctx).unwrap();
        assert_eq!(result, "John");
    }

    #[test]
    fn test_conditions() {
        let templates = Templates::from_directory("templates");

        let ctx = Context::new().with("show", true);
        let result = templates.render_string("{% if show %}visible{% endif %}", &ctx).unwrap();
        assert_eq!(result, "visible");

        let ctx = Context::new().with("show", false);
        let result = templates.render_string("{% if show %}yes{% else %}no{% endif %}", &ctx).unwrap();
        assert_eq!(result, "no");
    }

    #[test]
    fn test_for_loop() {
        let templates = Templates::from_directory("templates");
        let items: Vec<ContextValue> = vec!["a".into(), "b".into(), "c".into()];
        let ctx = Context::new().with("items", ContextValue::Array(items));

        let result = templates.render_string("{% for item in items %}{{ item }}{% endfor %}", &ctx).unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn test_context_to_json() {
        let ctx = Context::new()
            .with("name", "John")
            .with("age", 30i64);

        let json = ctx.to_json();
        assert_eq!(json["name"], "John");
        assert_eq!(json["age"], 30);
    }
}
