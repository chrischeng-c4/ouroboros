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
| **Refactoring Engine** | 📋 | **0%** | `refactoring.rs:248-291` | All 7 operations empty (extract_function, extract_method, rename, etc.) |
| **Semantic Search** | ✅ | **100%** | `semantic_search.rs:~1100` | All 7 search types implemented (2026-01-20) |
| **Framework Support** | 📋 | **20%** | `frameworks.rs:77-350` | Detection incomplete, type providers not integrated |

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
| **Other Sprint 2-6 Features** | ~42 | ❌ Critically Low (mostly placeholders) |

### Known Technical Debt
- ⚠️ **189 panic!/unwrap()/expect() calls** - Risk of crashes on edge cases
- ⚠️ **15+ empty functions** returning default values
- ⚠️ **No benchmark data** - Cannot validate performance claims
- ⚠️ **12+ "Placeholder implementation" comments**

---

## Competitive Position (Honest Assessment)

### Current State (2026-01-20)
- **Best Use Case**: Multi-language linting + basic type checking + semantic search in single tool
- **vs Competitors**: Feature maturity 5.0/10 (Pyright/JetBrains: 7-9/10)
- **Unique Strengths**: MCP integration, multi-language architecture, comprehensive semantic search
- **Critical Gaps**: Refactoring (0%), framework support (20%)

### Market Risk
- **Time Window**: 6-12 months before competitors may add multi-language support
- **Credibility Risk**: Over-promising unfinished features damages trust
- **Recommendation**: Focus marketing on mature features (linting, daemon, MCP)

---

## Implementation Roadmap

### Phase 1: Critical Gaps (3-4 months)
1. **Refactoring Engine** (4-6 weeks)
   - Implement all 7 operations in `refactoring.rs`
   - Integrate mutable AST
   - Add import management

2. **Semantic Search** (3-4 weeks)
   - Implement 6 empty search methods
   - Call hierarchy tracking
   - Type hierarchy traversal

3. **Framework Integration** (3-4 weeks)
   - Complete FrameworkDetector
   - Integrate type providers into main checker
   - Django/FastAPI deep inference

### Phase 2: Maturity (2-3 months)
4. Complete incremental analysis
5. Finish deep type inference
6. Raise test coverage to 60%+
7. Establish benchmarks

### Phase 3: Optimization (Ongoing)
8. Performance tuning
9. Error handling improvements
10. Documentation and examples
