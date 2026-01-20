# CLAUDE.md - Implementation Essentials

<!-- agentd:start -->
## Agentd: Spec-Driven Development

**IMPORTANT**: Do NOT make direct code changes. Use the SDD workflow below.

| Skill | Purpose |
|-------|---------|
| `/agentd:plan` | Planning workflow (proposal → challenge) |
| `/agentd:impl` | Implementation workflow |
| `/agentd:archive` | Archive completed change |

Start with: `/agentd:plan <id> "<description>"`
<!-- agentd:end -->

## Abbreviation
- ob: obouroboros
- obpg: ouroboros-postgres 
- obqc: ouroboros-qc

## Limitaion

No big file; If file lines ≥ 1000, must split; If file lines ≥ 500, consider split.

## 競品分析 (Competitors)

### Type Checkers & Static Analysis
- **Pyright** (Microsoft) - Fast Python type checker, powers VS Code Pylance
- **mypy** - Standard Python static type checker
- **Pyre** (Meta) - Performant type checker with security focus
- **Pytype** (Google) - Type inference without annotations

### Linters & Code Quality
- **Ruff** - Extremely fast Python linter (Rust-based)
- **pylint** - Comprehensive Python code analyzer
- **flake8** - Style guide enforcement
- **ESLint** - JavaScript/TypeScript linting standard

### Language Servers & Code Intelligence
**LSP (Language Server Protocol) Based:**
- **Pylance** (Microsoft) - Python LSP for VS Code
- **Jedi** - Python autocompletion/static analysis
- **typescript-language-server** - TypeScript LSP
- **rust-analyzer** - Rust LSP

**PSI (Program Structure Interface) Based:**
- **IntelliJ Platform** (JetBrains) - PSI for all JetBrains IDEs
  - PyCharm (Python), WebStorm (JS/TS), IntelliJ IDEA (Java/Kotlin), RustRover (Rust)
  - Deep semantic analysis with incremental parsing
  - Rich AST manipulation and refactoring support

### Multi-Language Analysis
- **SonarQube** - Multi-language code quality platform
- **CodeQL** (GitHub) - Semantic code analysis engine
- **Semgrep** - Fast, customizable static analysis

### Key Differentiators for Argus
1. **Unified Multi-Language**: Python, TypeScript, Rust in single tool
2. **Hybrid Architecture**: Combines LSP protocol compatibility with PSI-like semantic analysis
   - LSP for editor integration (VS Code, etc.)
   - PSI-inspired mutable AST for advanced refactoring
3. **Daemon Architecture**: Persistent analysis with incremental updates
4. **Deep Type Inference**: Cross-file analysis without full annotations
5. **MCP Integration**: Native LLM tool integration via Model Context Protocol
6. **Framework-Aware**: Django, FastAPI, Pydantic specialized support

---

## Feature Implementation Status

**Legend**: ✅ Production-Ready | ⚠️ Partial/In Progress | 📋 Planned/Placeholder

### Core Features (Mature)
| Feature | Status | Completion | Notes |
|---------|--------|------------|-------|
| **Basic Linting** | ✅ | ~80% | Python, TypeScript, Rust lint rules |
| **Type Checking (Python)** | ✅ | ~70% | Core type system solid (~10k LOC) |
| **LSP Integration** | ✅ | ~85% | Server implementation mature (306 test lines) |
| **MCP Protocol** | ✅ | ~80% | Native LLM tool integration working |
| **Daemon Architecture** | ✅ | ~75% | Long-running server with file watching |

### Sprint 2-6 Features (Gaps Identified)

#### 🔴 CRITICAL Priority (P0)
| Feature | Status | Completion | Files | Issue |
|---------|--------|------------|-------|-------|
| **Refactoring Engine** | ✅ | **100%** | `refactoring.rs:~1000` | All 7 operations implemented (2026-01-20) - extract_variable, extract_function, extract_method, rename, inline, change_signature, move |
| **Semantic Search** | ✅ | **100%** | `semantic_search.rs:~1100` | All 7 search types implemented (2026-01-20) |
| **Framework Support** | ✅ | **90%** | `frameworks.rs:~580` | Detection complete, type providers integrated (2026-01-20) |

#### 🟡 HIGH Priority (P1)
| Feature | Status | Completion | Files | Issue |
|---------|--------|------------|-------|-------|
| **Incremental Analysis** | ⚠️ | **40%** | `incremental.rs:~351` | Infrastructure exists, `analyze_file()` placeholder |
| **Deep Type Inference** | ⚠️ | **60%** | `deep_inference.rs:212` | Protocol conformance hardcoded, limited cross-file propagation |
| **Code Generation** | 📋 | **25%** | `codegen.rs:512` | Type stub generation mostly dead code |

#### 🟢 MEDIUM Priority (P2)
| Feature | Status | Completion | Files | Issue |
|---------|--------|------------|-------|-------|
| **Mutable AST** | ⚠️ | **50%** | `mutable_ast.rs:594` | Tree diff algorithm incomplete |
| **Multi-Language Depth** | ⚠️ | **50%** | Various | TypeScript/Rust support shallower than Python |

### Test Coverage Status
| Module Category | Test Lines | Status |
|-----------------|------------|--------|
| **Core Features** | ~1,350 | ✅ Sufficient (infer, check, narrow, server) |
| **Semantic Search (P0)** | ~400 | ✅ Comprehensive (7 unit + 4 integration tests) |
| **Refactoring Engine (P0)** | ~1,287 | ✅ Comprehensive (54 integration tests covering all operations) |
| **Framework Support (P0)** | ~320 | ✅ Good (18 integration tests for Django/FastAPI/Pydantic) |
| **Other Sprint 2-6 Features** | ~42 | ❌ Critically Low (mostly placeholders) |

### Known Technical Debt
- ⚠️ **189 panic!/unwrap()/expect() calls** - Risk of crashes on edge cases
- ⚠️ **15+ empty functions** returning default values
- ⚠️ **No benchmark data** - Cannot validate performance claims
- ⚠️ **12+ "Placeholder implementation" comments**

---

## Competitive Position (Honest Assessment)

### Current State (2026-01-20)
- **Best Use Case**: Multi-language linting + type checking + semantic search + refactoring in single tool
- **vs Competitors**: Feature maturity 7.0/10 (Pyright: 7/10, JetBrains: 8-9/10)
- **Unique Strengths**: MCP integration, multi-language architecture, comprehensive semantic search, multi-language refactoring
- **Strengths**: All P0 features implemented (Semantic Search ✅, Framework Support ✅, Refactoring ✅)
- **Remaining Gaps**: Some P1/P2 features (incremental analysis depth, code generation)

### Market Risk
- **Time Window**: 6-12 months before competitors may add multi-language support
- **Credibility Risk**: Over-promising unfinished features damages trust
- **Recommendation**: Focus marketing on mature features (linting, daemon, MCP)

---

## Implementation Roadmap

### ✅ Completed Phases

**Phase 1: Semantic Search (100%)** - Completed 2026-01-20
- ✅ All 7 search types implemented (usages, definitions, type signature, implementations, call hierarchy, type hierarchy, patterns)
- ✅ Symbol indexing and reference tracking
- ✅ Call graph and type hierarchy traversal
- ✅ 400+ lines of comprehensive tests

**Phase 2: Framework Support (90%)** - Completed 2026-01-20
- ✅ Framework detection (Django, FastAPI, Flask, Pydantic)
- ✅ Type providers for Django QuerySet, FastAPI routes, Pydantic models
- ✅ Integration with type inference engine
- ✅ 320+ lines of integration tests

**Phase 3: Refactoring Engine (100%)** - Completed 2026-01-20
- ✅ M3.1: AST integration and caching (11 tests)
- ✅ M3.2: Extract variable/function operations (14 tests)
- ✅ M3.3: Rename symbol operation (14 tests)
- ✅ M3.4: Advanced refactoring (inline, move, change signature, extract method) (15 tests)
- ✅ 1,287+ lines of comprehensive tests
- ✅ Multi-language support (Python, TypeScript, Rust)

**Total P0 Completion: ~72.5%** (up from initial 14-20%)

### 🚧 Phase 4: Integration & Polish (Current)

**Goals**:
1. Integration testing across phases
   - Semantic search + refactoring workflows
   - Framework-aware refactoring
   - Real-world project testing

2. Documentation & Examples
   - API documentation
   - Usage tutorials
   - LSP integration guide

3. Performance & Stability
   - Benchmark establishment
   - Error handling improvements
   - Memory optimization

### 🔮 Future Phases (P1/P2)

**Phase 5: P1 Features** (2-3 months)
- Complete incremental analysis depth
- Finish deep type inference enhancements
- Code generation maturity
- LSP code actions integration

**Phase 6: Optimization** (Ongoing)
- Performance tuning
- Multi-language depth (TypeScript/Rust parity with Python)
- Mutable AST diff algorithm completion
