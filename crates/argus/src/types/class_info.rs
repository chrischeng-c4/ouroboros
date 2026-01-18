//! Class information for type inference

use std::collections::HashMap;

use super::ty::Type;

/// Information about a class definition
#[derive(Debug, Clone, Default)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Base classes
    pub bases: Vec<String>,
    /// Instance attributes (name -> type)
    pub attributes: HashMap<String, Type>,
    /// Methods (name -> callable type)
    pub methods: HashMap<String, Type>,
    /// Class variables (name -> type)
    pub class_vars: HashMap<String, Type>,
}

impl ClassInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            bases: Vec::new(),
            attributes: HashMap::new(),
            methods: HashMap::new(),
            class_vars: HashMap::new(),
        }
    }

    /// Get attribute type (checks instance attrs, then methods, then class vars)
    pub fn get_attribute(&self, name: &str) -> Option<&Type> {
        self.attributes
            .get(name)
            .or_else(|| self.methods.get(name))
            .or_else(|| self.class_vars.get(name))
    }
}
