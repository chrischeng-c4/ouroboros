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
mod env;
mod deep_inference;
mod mutable_ast;
mod refactoring;
mod codegen;
mod semantic_search;
mod incremental;
mod frameworks;

pub use ty::{LiteralValue, Param, ParamKind, Type, TypeVarId, Variance};
pub use class_info::{ClassInfo, GenericParam};
pub use type_env::TypeEnv;
pub use infer::{TypeInferencer, TypeVarInfo};
pub use check::{TypeChecker, TypeError, SemanticModelBuilder, build_semantic_model};
pub use narrow::{NarrowingCondition, TypeNarrower};
pub use imports::{Import, ImportResolver, ImportedName, ModuleInfo, ModuleIndexEntry, ModuleLoadState};
pub use stubs::StubLoader;
pub use typeshed::{TypeshedCache, TypeshedConfig};
pub use modules::{ModuleGraph, ModuleNode};
pub use project::{ProjectAnalyzer, ProjectConfig};
pub use cache::{AnalysisCache, CacheEntry, ContentHash};
pub use config::{ArgusConfig, EffectiveConfig, OverrideConfig, PythonEnvConfig};
pub use model::{
    LiteralInfo, ParamInfo, ScopeId, ScopeInfo, SemanticModel, SemanticSymbolKind,
    SymbolData, SymbolId, SymbolReference, TypeInfo, TypedRange,
};
pub use env::{
    detect_python_environment, detect_with_config, detect_all_venvs,
    find_site_packages, is_venv_directory, get_venv_python_version,
    DetectedEnv, EnvInfo, VenvType,
};
pub use deep_inference::{
    TypeContext, TypeBinding, TypeVarInfo as DeepTypeVarInfo, ProtocolDef, MethodSignature,
    GenericKey, DeepTypeInferencer, FileAnalysis, ImportInfo, ImportGraph,
    TypeTraceStep, DeepInferenceResult, CrossFileRef,
    infer_type_deep, trace_type_chain,
};
pub use mutable_ast::{
    NodeId, NodeRef, MutableNode, Span, NodeMetadata,
    MutableAst, AstEdit, TreeDiff,
};
pub use refactoring::{
    RefactorRequest, RefactorKind, RefactorOptions, SignatureChanges,
    RefactorResult, TextEdit, ImportChange, RefactorDiagnostic, DiagnosticLevel,
    RefactoringEngine,
};
pub use codegen::{
    CodeGenRequest, CodeGenKind, DocstringStyle, TestFramework, CodeGenOptions,
    CodeGenResult, CodeGenerator,
};
pub use semantic_search::{
    SearchQuery, SearchKind, CallDirection, TypeHierarchyDirection, SearchScope,
    SearchResult, SearchMatch, MatchKind, MatchContext, SearchStats,
    SemanticSearchEngine, SymbolLocation, TypeLocation,
};
pub use incremental::{
    FileChange, ChangeKind, ChangeTracker, DependencyGraph,
    IncrementalConfig, AnalysisResult, IncrementalAnalyzer, CachedAnalysis,
};
pub use frameworks::{
    Framework, FrameworkDetection, FrameworkDetector, FrameworkTypeProvider, MethodType,
    DjangoTypeProvider, DjangoModel, DjangoField, DjangoFieldType, DjangoRelation, DjangoRelationType,
    FastAPITypeProvider, FastAPIEndpoint,
    PydanticTypeProvider, PydanticModel, PydanticField, PydanticConfig, PydanticExtra,
    FrameworkRegistry,
};
