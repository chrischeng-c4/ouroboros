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
