//! Type system for Argus
//!
//! This module provides type inference and checking for Python, TypeScript, and Rust.

mod ty;
mod annotation;
mod builtins;
mod class_info;
mod type_env;
mod infer;
mod check;
mod narrow;
mod imports;
mod stubs;
mod typeshed;
mod modules;
mod project;
mod cache;
mod config;
mod model;

pub use ty::{LiteralValue, Param, ParamKind, Type, TypeVarId, Variance};
pub use class_info::{ClassInfo, GenericParam};
pub use type_env::TypeEnv;
pub use infer::{TypeInferencer, TypeVarInfo};
pub use check::{TypeChecker, TypeError, SemanticModelBuilder, build_semantic_model};
pub use narrow::{NarrowingCondition, TypeNarrower};
pub use imports::{Import, ImportResolver, ImportedName, ModuleInfo};
pub use stubs::StubLoader;
pub use typeshed::{TypeshedCache, TypeshedConfig};
pub use modules::{ModuleGraph, ModuleNode};
pub use project::{ProjectAnalyzer, ProjectConfig};
pub use cache::{AnalysisCache, CacheEntry, ContentHash};
pub use config::{ArgusConfig, EffectiveConfig, OverrideConfig};
pub use model::{
    LiteralInfo, ParamInfo, ScopeId, ScopeInfo, SemanticModel, SemanticSymbolKind,
    SymbolData, SymbolId, SymbolReference, TypeInfo, TypedRange,
};
