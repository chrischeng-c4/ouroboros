# Change: complete-argus-parity-2

<meta>
  <purpose>PRD - Product Requirements Document</purpose>
  <constraint>Describes WHAT and WHY, not HOW</constraint>
</meta>

## Summary

Complete the Argus Python type checker to reach feature parity with industry-standard tools like mypy and pyright. This includes dynamic typeshed integration, robust error recovery, watch mode, extended LSP features, generic class instantiation inference, and variance checking.

## Why

Currently, Argus provides basic type checking and linting, but lacks several critical features required for production-level Python development. Bundled stubs are limited, AST analysis is brittle on incomplete code (common during editing), and advanced type system features like generics and variance are not fully supported. These gaps prevent Argus from being a full replacement for mypy/pyright in large, complex codebases.

## What Changes

- **Dynamic Typeshed Integration**: Implement a system to automatically download, cache, and load typeshed stubs at runtime. Bundled stubs are retained as a fallback when network is unavailable, with precedence order: Local .pyi > Downloaded Typeshed > Bundled stubs.
- **Error Recovery**: Enhance the AST analysis to gracefully handle and recover from incomplete or invalid syntax, ensuring that analysis continues for the rest of the file.
- **Watch Mode**: Add a file system observer using the `notify` crate to support automatic re-analysis upon file changes.
- **Extended LSP Features**: Implement `textDocument/rename`, `textDocument/references`, and a broader range of `textDocument/codeAction` (beyond basic quick fixes).
- **Generic Class Instantiation Inference**: Improve type inference to correctly handle generic class instantiations, including nested generics and complex bounds.
- **Variance Checking**: Implement covariance, contravariance, and invariance checking for generic types to ensure type safety in complex hierarchies.

## Impact

- Affected specs:
  - `specs/dynamic-typeshed.md`
  - `specs/error-recovery.md`
  - `specs/watch-mode.md`
  - `specs/advanced-lsp.md`
  - `specs/generic-inference.md`
  - `specs/variance-checking.md`
- Affected code:
  - `crates/argus/Cargo.toml`
  - `crates/argus/src/lib.rs`
  - `crates/argus/src/core/config.rs`
  - `crates/argus/src/types/typeshed.rs`
  - `crates/argus/src/types/stubs.rs`
  - `crates/argus/src/types/ty.rs`
  - `crates/argus/src/types/class_info.rs`
  - `crates/argus/src/types/infer.rs`
  - `crates/argus/src/types/modules.rs`
  - `crates/argus/src/types/check.rs`
  - `crates/argus/src/types/config.rs`
  - `crates/argus/src/syntax/parser.rs`
  - `crates/argus/src/semantic/mod.rs`
  - `crates/argus/src/lsp/server.rs`
  - `crates/argus/src/lsp/workspace.rs`
  - `crates/argus/src/lsp/code_actions.rs`
  - `crates/argus/src/lsp/tests.rs` (new file)
  - `crates/argus/src/semantic/tests.rs` (new file)
- Breaking changes: No.