---
change: complete-argus-parity-2
date: 2026-01-18
updated: 2026-01-19
---

# Clarifications (Updated Scope - 2026-01-19)

## Scope Expansion

The original proposal (2026-01-18) focused on: dynamic typeshed, error recovery, watch mode, extended LSP, generic inference, and variance checking.

**New requirements (2026-01-19)**: Expand to achieve **100% parity with mypy/pyright** for Python type checking, with integrated **MCP Server** and **Daemon Mode** for LLM usage.

---

## Q1: Feature Priority
- **Question**: 在 mypy/pyright/ruff 的功能中，你希望优先实现哪个方面？
- **Answer**: **Type checking** (优先实现完整的类型检查功能)
- **Rationale**: Focus on matching mypy/pyright's core type checking capabilities (generics, protocols, type inference) as the foundation. Linting and performance optimizations can follow.

## Q2: Type Checking Coverage Goal  
- **Question**: 对于 Type Checking 的覆盖目标是什么？
- **Answer**: **100% parity** (完全匹配 mypy/pyright 的所有功能)
- **Rationale**: Aim for complete feature parity with mypy/pyright, including experimental features. This ensures Argus can handle any Python codebase that these tools can analyze.

## Q3: MCP Server and Daemon Mode
- **Question**: 是否需要在这个阶段同时实现 MCP Server 和 Daemon Mode？  
- **Answer**: **Yes - Include now** (在实现 type checking 的同时构建 MCP server)
- **Rationale**: Build the MCP server integration alongside type checking so LLMs can immediately use Argus. The daemon mode will provide performance benefits for interactive use.

## Q4: Validation Strategy
- **Question**: 如何验证达到 parity 的目标？
- **Answer**: **All three approaches**:
  1. **Benchmark suite**: 使用 mypy/pyright 的测试集，要求通过率 ≥90%
  2. **Real projects**: 在真实 Python 项目（如 Django、FastAPI）上测试，对比诊断结果  
  3. **Performance benchmarks**: 验证性能指标（速度、内存占用）达到或超越 ruff
- **Rationale**: Comprehensive validation ensures both functional correctness and practical usability.

---

## Expanded Scope Summary

### 1. Core Type System (100% parity)
- All PEP 484/526/544/585/586/589/591/593/604/612/613/646/647/655/673/742 features
- Generic types: List[T], Dict[K,V], ParamSpec, TypeVarTuple, Self
- Union, Optional, Literal, TypedDict, Protocol
- Type narrowing: isinstance, TypeGuard, TypeIs, match statements
- Intersection types, Annotated, Final, ClassVar

### 2. Type Inference & Checking
- Bidirectional type checking
- Generic parameter inference
- Lambda and decorator type inference
- Complete error detection (missing returns, unreachable code, etc.)

### 3. Module System
- Full import resolution (absolute, relative, star imports)
- Typeshed integration (download, cache, load)
- Cross-file analysis with incremental updates
- Circular import detection

### 4. MCP Server + Daemon (NEW)
**MCP Tools** (10 tools):
- check, type_at, symbols, hover
- goto_definition, find_references, quick_fix
- analyze_file, analyze_project, infer_types

**Daemon Features**:
- Unix socket server
- In-memory index
- File watcher (debounced)
- Session management
- <100ms latency (warm daemon)

### 5. Validation Suite (NEW)
- mypy test cases (≥90% pass rate)
- Real projects: Django, FastAPI, Flask, Pandas
- Performance benchmarks vs mypy/pyright/ruff

### 6. Configuration
- [tool.argus] in pyproject.toml
- Compatible with common mypy/ruff options

---

## Success Criteria

1. **Functional**: ≥90% of mypy/pyright tests pass
2. **Real-world**: Django/FastAPI analysis with <5% false positives vs mypy
3. **Performance**: Match or beat ruff's speed
4. **LLM**: MCP server <100ms latency (daemon warm)
5. **No regressions**: All existing Argus tests pass

## Timeline Estimate

**Large change**: 4-6 weeks
- Phase 1: Core type system (2 weeks)
- Phase 2: MCP + Daemon (1 week)
- Phase 3: Validation + bugfixes (2 weeks)
- Phase 4: Documentation (1 week)
