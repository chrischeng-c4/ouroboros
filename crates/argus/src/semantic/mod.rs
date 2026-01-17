//! Advanced code analysis

pub mod scope;
pub mod symbols;

pub use scope::{ScopeAnalyzer, Scope, ScopeKind};
pub use scope::{Symbol as ScopeSymbol, SymbolKind as ScopeSymbolKind};
pub use symbols::{Symbol, SymbolId, SymbolKind, SymbolTable, SymbolTableBuilder, TypeInfo};
