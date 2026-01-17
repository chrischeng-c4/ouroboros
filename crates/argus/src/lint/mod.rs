//! Language-specific checkers

mod python;
mod rust_checker;
mod typescript;

use crate::syntax::{Language, ParsedFile};
use crate::diagnostic::Diagnostic;
use crate::LintConfig;
use std::collections::HashMap;

pub use python::PythonChecker;
pub use rust_checker::RustChecker;
pub use typescript::TypeScriptChecker;

/// Trait for language-specific checkers
pub trait Checker: Send + Sync {
    fn language(&self) -> Language;
    fn check(&self, file: &ParsedFile, config: &LintConfig) -> Vec<Diagnostic>;
    fn available_rules(&self) -> Vec<&'static str>;
}

/// Registry of all checkers
pub struct CheckerRegistry {
    checkers: HashMap<Language, Box<dyn Checker>>,
}

impl CheckerRegistry {
    pub fn new() -> Self {
        let mut checkers: HashMap<Language, Box<dyn Checker>> = HashMap::new();

        checkers.insert(Language::Python, Box::new(PythonChecker::new()));
        checkers.insert(Language::TypeScript, Box::new(TypeScriptChecker::new()));
        checkers.insert(Language::Rust, Box::new(RustChecker::new()));

        Self { checkers }
    }

    pub fn get(&self, language: Language) -> Option<&dyn Checker> {
        self.checkers.get(&language).map(|c| c.as_ref())
    }
}

impl Default for CheckerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
