//! Type system for Argus
//!
//! This module provides type inference and checking for Python, TypeScript, and Rust.

mod ty;
mod infer;
mod check;
mod narrow;
mod imports;
mod stubs;

pub use ty::{LiteralValue, Param, ParamKind, Type, TypeVarId};
pub use infer::{ClassInfo, TypeEnv, TypeInferencer};
pub use check::{TypeChecker, TypeError};
pub use narrow::{NarrowingCondition, TypeNarrower};
pub use imports::{Import, ImportResolver, ImportedName, ModuleInfo};
pub use stubs::StubLoader;
