//! Framework support (Sprint 6 - Track 1)
//!
//! Provides type inference for popular Python frameworks:
//! - Django: models, views, templates
//! - Flask: routes, blueprints
//! - FastAPI: endpoints, dependency injection
//! - Pydantic: models, validators
//! - SQLAlchemy: ORM mappings

use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::ty::Type;
use super::deep_inference::TypeContext;

// ============================================================================
// Framework Detection
// ============================================================================

/// Detected framework in a project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Framework {
    Django,
    Flask,
    FastAPI,
    Pydantic,
    SQLAlchemy,
    Celery,
    Custom(String),
}

/// Framework detection result.
#[derive(Debug, Clone)]
pub struct FrameworkDetection {
    /// Detected frameworks
    pub frameworks: Vec<Framework>,
    /// Framework-specific files
    pub framework_files: HashMap<Framework, Vec<PathBuf>>,
    /// Confidence scores (0.0 to 1.0)
    pub confidence: HashMap<Framework, f64>,
}

impl FrameworkDetection {
    /// Create empty detection.
    pub fn empty() -> Self {
        Self {
            frameworks: Vec::new(),
            framework_files: HashMap::new(),
            confidence: HashMap::new(),
        }
    }

    /// Check if a framework was detected.
    pub fn has_framework(&self, framework: &Framework) -> bool {
        self.frameworks.contains(framework)
    }

    /// Get confidence for a framework.
    pub fn confidence_for(&self, framework: &Framework) -> f64 {
        self.confidence.get(framework).copied().unwrap_or(0.0)
    }

    /// Add a detected framework with confidence.
    pub fn add_framework(&mut self, framework: Framework, confidence: f64) {
        if !self.frameworks.contains(&framework) {
            self.frameworks.push(framework.clone());
        }
        self.confidence.insert(framework, confidence);
    }
}

/// Detect frameworks in a project.
pub struct FrameworkDetector {
    /// Project root
    root: PathBuf,
}

impl FrameworkDetector {
    /// Create a new detector.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Detect frameworks in the project.
    pub fn detect(&self) -> FrameworkDetection {
        let mut result = FrameworkDetection::empty();

        // Detect each framework
        self.detect_django(&mut result);
        self.detect_flask(&mut result);
        self.detect_fastapi(&mut result);

        result
    }

    /// Check for Django.
    fn detect_django(&self, result: &mut FrameworkDetection) {
        let mut confidence: f64 = 0.0;
        let mut indicators = 0;

        // 1. Check for manage.py (strong indicator)
        let manage_py = self.root.join("manage.py");
        if manage_py.exists() {
            confidence += 0.4;
            indicators += 1;
        }

        // 2. Check for settings.py or settings module
        if self.find_files_recursive("settings.py", 2) {
            confidence += 0.3;
            indicators += 1;
        }

        // 3. Check for models.py files
        if self.find_files_recursive("models.py", 2) {
            confidence += 0.15;
            indicators += 1;
        }

        // 4. Check requirements files for Django (no version check needed)
        if self.check_requirements_for("django", 0.0) {
            confidence += 0.25;
            indicators += 1;
        }

        // 5. Check pyproject.toml for Django
        if self.check_pyproject_for("django") {
            confidence += 0.2;
            indicators += 1;
        }

        if indicators > 0 {
            result.add_framework(Framework::Django, confidence.min(1.0));
        }
    }

    /// Check for Flask.
    fn detect_flask(&self, result: &mut FrameworkDetection) {
        let mut confidence: f64 = 0.0;
        let mut indicators = 0;

        // 1. Check requirements for Flask (no version check needed)
        if self.check_requirements_for("flask", 0.0) {
            confidence += 0.4;
            indicators += 1;
        }

        // 2. Check pyproject.toml
        if self.check_pyproject_for("flask") {
            confidence += 0.3;
            indicators += 1;
        }

        // 3. Check for app.py or similar Flask app files with Flask code
        let app_files = ["app.py", "application.py", "wsgi.py"];
        for app_file in &app_files {
            let path = self.root.join(app_file);
            if path.exists() {
                // Check if file contains Flask code
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.contains("from flask") || content.contains("import flask")
                        || content.contains("Flask(__name__)") {
                        confidence += 0.2;
                        indicators += 1;
                        break;
                    }
                }
            }
        }

        // 4. Look for blueprints directory
        if self.root.join("blueprints").is_dir() || self.find_files_recursive("blueprints.py", 2) {
            confidence += 0.1;
            indicators += 1;
        }

        if indicators > 0 {
            result.add_framework(Framework::Flask, confidence.min(1.0));
        }
    }

    /// Check for FastAPI.
    fn detect_fastapi(&self, result: &mut FrameworkDetection) {
        let mut confidence: f64 = 0.0;
        let mut indicators = 0;

        // 1. Check requirements for FastAPI (no version check needed)
        if self.check_requirements_for("fastapi", 0.0) {
            confidence += 0.5;
            indicators += 1;
        }

        // 2. Check pyproject.toml
        if self.check_pyproject_for("fastapi") {
            confidence += 0.3;
            indicators += 1;
        }

        // 3. Check for main.py or app.py with FastAPI code
        let app_files = ["main.py", "app.py", "api.py"];
        for app_file in &app_files {
            let path = self.root.join(app_file);
            if path.exists() {
                // Check if file contains FastAPI code
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.contains("from fastapi") || content.contains("import fastapi")
                        || content.contains("FastAPI()") {
                        confidence += 0.15;
                        indicators += 1;
                        break;
                    }
                }
            }
        }

        // 4. Check for routers directory
        if self.root.join("routers").is_dir() || self.root.join("api").is_dir() {
            confidence += 0.05;
            indicators += 1;
        }

        if indicators > 0 {
            result.add_framework(Framework::FastAPI, confidence.min(1.0));
        }
    }

    // Helper methods

    /// Find files recursively with given name.
    fn find_files_recursive(&self, filename: &str, max_depth: usize) -> bool {
        self.find_files_recursive_impl(&self.root, filename, max_depth, 0)
    }

    fn find_files_recursive_impl(
        &self,
        dir: &PathBuf,
        filename: &str,
        max_depth: usize,
        current_depth: usize,
    ) -> bool {
        if current_depth > max_depth {
            return false;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Check if this is the file we're looking for
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name == filename {
                        return true;
                    }
                }

                // If it's a directory (not hidden), recurse into it
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if !dir_name.starts_with('.') {
                            if self.find_files_recursive_impl(&path, filename, max_depth, current_depth + 1) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check requirements files for a package.
    fn check_requirements_for(&self, package: &str, min_version: f64) -> bool {
        let req_files = [
            "requirements.txt",
            "requirements/base.txt",
            "requirements/production.txt",
            "dev-requirements.txt",
        ];

        for req_file in &req_files {
            let path = self.root.join(req_file);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.to_lowercase().starts_with(package) {
                            // Found the package, check version if needed
                            if min_version > 0.0 {
                                // Simple version check (could be enhanced)
                                if let Some(version_part) = line.split(">=").nth(1) {
                                    if let Some(version_str) = version_part.split(&['<', '=', ','][..]).next() {
                                        if let Ok(version) = version_str.trim().parse::<f64>() {
                                            return version >= min_version;
                                        }
                                    }
                                }
                            }
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Check pyproject.toml for a dependency.
    fn check_pyproject_for(&self, package: &str) -> bool {
        let pyproject = self.root.join("pyproject.toml");
        if pyproject.exists() {
            if let Ok(content) = std::fs::read_to_string(&pyproject) {
                // Simple check - could be enhanced with proper TOML parsing
                return content.to_lowercase().contains(&format!("\"{}\"", package))
                    || content.to_lowercase().contains(&format!("'{}'", package))
                    || content.to_lowercase().contains(&format!("{} = ", package));
            }
        }
        false
    }
}

// ============================================================================
// Framework-Specific Type Providers
// ============================================================================

/// Provides framework-specific type information.
pub trait FrameworkTypeProvider {
    /// Get types for a symbol.
    fn get_type(&self, symbol: &str, context: &TypeContext) -> Option<Type>;

    /// Get attribute types for an object.
    fn get_attribute_type(&self, base_type: &Type, attr: &str) -> Option<Type>;

    /// Get method signatures.
    fn get_method_signature(&self, base_type: &Type, method: &str) -> Option<MethodType>;

    /// Framework name.
    fn framework_name(&self) -> &str;
}

/// Method type with parameters and return.
#[derive(Debug, Clone)]
pub struct MethodType {
    /// Parameter types
    pub params: Vec<(String, Type)>,
    /// Return type
    pub return_type: Type,
    /// Is async
    pub is_async: bool,
}

// ============================================================================
// Django Support
// ============================================================================

/// Django type provider.
pub struct DjangoTypeProvider {
    /// Model definitions
    models: HashMap<String, DjangoModel>,
}

/// Django model definition.
#[derive(Debug, Clone)]
pub struct DjangoModel {
    /// Model name
    pub name: String,
    /// Fields
    pub fields: HashMap<String, DjangoField>,
    /// Related models
    pub relations: Vec<DjangoRelation>,
}

/// Django field.
#[derive(Debug, Clone)]
pub struct DjangoField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: DjangoFieldType,
    /// Is nullable
    pub null: bool,
    /// Has default
    pub has_default: bool,
}

/// Django field type.
#[derive(Debug, Clone)]
pub enum DjangoFieldType {
    CharField,
    TextField,
    IntegerField,
    FloatField,
    BooleanField,
    DateField,
    DateTimeField,
    ForeignKey(String),
    OneToOneField(String),
    ManyToManyField(String),
    Custom(String),
}

/// Django relation.
#[derive(Debug, Clone)]
pub struct DjangoRelation {
    /// Relation name
    pub name: String,
    /// Related model
    pub related_model: String,
    /// Relation type
    pub relation_type: DjangoRelationType,
}

/// Django relation type.
#[derive(Debug, Clone)]
pub enum DjangoRelationType {
    ForeignKey,
    OneToOne,
    ManyToMany,
}

impl DjangoTypeProvider {
    /// Create a new provider.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Register a model.
    pub fn register_model(&mut self, model: DjangoModel) {
        self.models.insert(model.name.clone(), model);
    }

    /// Get model info.
    pub fn get_model(&self, name: &str) -> Option<&DjangoModel> {
        self.models.get(name)
    }
}

impl Default for DjangoTypeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkTypeProvider for DjangoTypeProvider {
    fn get_type(&self, symbol: &str, _context: &TypeContext) -> Option<Type> {
        // Check if symbol is a model
        if self.models.contains_key(symbol) {
            Some(Type::Instance {
                name: symbol.to_string(),
                module: Some("models".to_string()),
                type_args: Vec::new(),
            })
        } else {
            None
        }
    }

    fn get_attribute_type(&self, base_type: &Type, attr: &str) -> Option<Type> {
        if let Type::Instance { name, .. } = base_type {
            if let Some(model) = self.models.get(name) {
                if let Some(field) = model.fields.get(attr) {
                    return Some(self.field_type_to_type(&field.field_type));
                }
            }
        }
        None
    }

    fn get_method_signature(&self, _base_type: &Type, _method: &str) -> Option<MethodType> {
        None
    }

    fn framework_name(&self) -> &str {
        "Django"
    }
}

impl DjangoTypeProvider {
    fn field_type_to_type(&self, field_type: &DjangoFieldType) -> Type {
        match field_type {
            DjangoFieldType::CharField | DjangoFieldType::TextField => Type::Str,
            DjangoFieldType::IntegerField => Type::Int,
            DjangoFieldType::FloatField => Type::Float,
            DjangoFieldType::BooleanField => Type::Bool,
            DjangoFieldType::DateField | DjangoFieldType::DateTimeField => Type::Instance {
                name: "datetime".to_string(),
                module: Some("datetime".to_string()),
                type_args: Vec::new(),
            },
            DjangoFieldType::ForeignKey(model)
            | DjangoFieldType::OneToOneField(model) => Type::Instance {
                name: model.clone(),
                module: None,
                type_args: Vec::new(),
            },
            DjangoFieldType::ManyToManyField(model) => Type::List(Box::new(Type::Instance {
                name: model.clone(),
                module: None,
                type_args: Vec::new(),
            })),
            DjangoFieldType::Custom(name) => Type::Instance {
                name: name.clone(),
                module: None,
                type_args: Vec::new(),
            },
        }
    }
}

// ============================================================================
// FastAPI Support
// ============================================================================

/// FastAPI type provider.
pub struct FastAPITypeProvider {
    /// Registered endpoints
    endpoints: HashMap<String, FastAPIEndpoint>,
    /// Dependency types
    dependencies: HashMap<String, Type>,
}

/// FastAPI endpoint.
#[derive(Debug, Clone)]
pub struct FastAPIEndpoint {
    /// Path
    pub path: String,
    /// HTTP methods
    pub methods: Vec<String>,
    /// Request body type
    pub request_body: Option<Type>,
    /// Response type
    pub response_type: Type,
    /// Dependencies
    pub dependencies: Vec<String>,
}

impl FastAPITypeProvider {
    /// Create a new provider.
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Register an endpoint.
    pub fn register_endpoint(&mut self, name: String, endpoint: FastAPIEndpoint) {
        self.endpoints.insert(name, endpoint);
    }

    /// Register a dependency.
    pub fn register_dependency(&mut self, name: String, ty: Type) {
        self.dependencies.insert(name, ty);
    }
}

impl Default for FastAPITypeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkTypeProvider for FastAPITypeProvider {
    fn get_type(&self, symbol: &str, _context: &TypeContext) -> Option<Type> {
        self.dependencies.get(symbol).cloned()
    }

    fn get_attribute_type(&self, _base_type: &Type, _attr: &str) -> Option<Type> {
        None
    }

    fn get_method_signature(&self, _base_type: &Type, _method: &str) -> Option<MethodType> {
        None
    }

    fn framework_name(&self) -> &str {
        "FastAPI"
    }
}

// ============================================================================
// Pydantic Support
// ============================================================================

/// Pydantic type provider.
pub struct PydanticTypeProvider {
    /// Registered models
    models: HashMap<String, PydanticModel>,
}

/// Pydantic model.
#[derive(Debug, Clone)]
pub struct PydanticModel {
    /// Model name
    pub name: String,
    /// Fields
    pub fields: HashMap<String, PydanticField>,
    /// Validators
    pub validators: Vec<String>,
    /// Config class
    pub config: Option<PydanticConfig>,
}

/// Pydantic field.
#[derive(Debug, Clone)]
pub struct PydanticField {
    /// Field name
    pub name: String,
    /// Field type
    pub ty: Type,
    /// Default value
    pub default: Option<String>,
    /// Field validators
    pub validators: Vec<String>,
    /// Alias
    pub alias: Option<String>,
}

/// Pydantic config.
#[derive(Debug, Clone)]
pub struct PydanticConfig {
    /// Allow extra fields
    pub extra: PydanticExtra,
    /// Validate assignment
    pub validate_assignment: bool,
    /// Use enum values
    pub use_enum_values: bool,
}

/// Pydantic extra handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PydanticExtra {
    Allow,
    Forbid,
    Ignore,
}

impl PydanticTypeProvider {
    /// Create a new provider.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Register a model.
    pub fn register_model(&mut self, model: PydanticModel) {
        self.models.insert(model.name.clone(), model);
    }

    /// Get model info.
    pub fn get_model(&self, name: &str) -> Option<&PydanticModel> {
        self.models.get(name)
    }
}

impl Default for PydanticTypeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkTypeProvider for PydanticTypeProvider {
    fn get_type(&self, symbol: &str, _context: &TypeContext) -> Option<Type> {
        if self.models.contains_key(symbol) {
            Some(Type::Instance {
                name: symbol.to_string(),
                module: None,
                type_args: Vec::new(),
            })
        } else {
            None
        }
    }

    fn get_attribute_type(&self, base_type: &Type, attr: &str) -> Option<Type> {
        if let Type::Instance { name, .. } = base_type {
            if let Some(model) = self.models.get(name) {
                if let Some(field) = model.fields.get(attr) {
                    return Some(field.ty.clone());
                }
            }
        }
        None
    }

    fn get_method_signature(&self, _base_type: &Type, _method: &str) -> Option<MethodType> {
        None
    }

    fn framework_name(&self) -> &str {
        "Pydantic"
    }
}

// ============================================================================
// Framework Registry
// ============================================================================

/// Registry of framework type providers.
pub struct FrameworkRegistry {
    /// Registered providers
    providers: Vec<Box<dyn FrameworkTypeProvider + Send + Sync>>,
}

impl FrameworkRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register a provider.
    pub fn register(&mut self, provider: Box<dyn FrameworkTypeProvider + Send + Sync>) {
        self.providers.push(provider);
    }

    /// Get type from any provider.
    pub fn get_type(&self, symbol: &str, context: &TypeContext) -> Option<Type> {
        for provider in &self.providers {
            if let Some(ty) = provider.get_type(symbol, context) {
                return Some(ty);
            }
        }
        None
    }

    /// Get attribute type from any provider.
    pub fn get_attribute_type(&self, base_type: &Type, attr: &str) -> Option<Type> {
        for provider in &self.providers {
            if let Some(ty) = provider.get_attribute_type(base_type, attr) {
                return Some(ty);
            }
        }
        None
    }
}

impl Default for FrameworkRegistry {
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

    #[test]
    fn test_framework_detection() {
        let detector = FrameworkDetector::new(PathBuf::from("."));
        let result = detector.detect();
        assert!(result.frameworks.is_empty()); // Empty project
    }

    #[test]
    fn test_django_provider() {
        let mut provider = DjangoTypeProvider::new();

        let mut fields = HashMap::new();
        fields.insert(
            "name".to_string(),
            DjangoField {
                name: "name".to_string(),
                field_type: DjangoFieldType::CharField,
                null: false,
                has_default: false,
            },
        );

        provider.register_model(DjangoModel {
            name: "User".to_string(),
            fields,
            relations: Vec::new(),
        });

        assert!(provider.get_model("User").is_some());
    }

    #[test]
    fn test_pydantic_provider() {
        let mut provider = PydanticTypeProvider::new();

        provider.register_model(PydanticModel {
            name: "UserModel".to_string(),
            fields: HashMap::new(),
            validators: Vec::new(),
            config: None,
        });

        assert!(provider.get_model("UserModel").is_some());
    }
}
