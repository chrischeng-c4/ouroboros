---
change: complete-argus-parity
date: 2026-01-18
---

# Clarifications

## Q1: Scope
- **Question**: Which gaps should we prioritize closing first?
- **Answer**: Full Parity - All gaps: typeshed, LSP, watch mode, error recovery, generics, variance
- **Rationale**: Comprehensive approach to reach production-ready state comparable to mypy/pyright

## Q2: Typeshed Integration
- **Question**: How should typeshed stubs be loaded?
- **Answer**: Dynamic Loading - Download/cache typeshed at runtime, always up-to-date
- **Rationale**: Keeps stubs current without manual updates, reduces binary size

## Q3: Git Workflow
- **Question**: What's your preferred git workflow?
- **Answer**: New branch - Create agentd/complete-argus-parity branch
- **Rationale**: Isolated development for this large change

---

## Gap Analysis Summary

Based on prior analysis, the gaps between Argus and mypy/pyright include:

| Gap | Priority | Impact |
|-----|----------|--------|
| Complete typeshed integration | High | Currently only ~10 bundled stubs |
| Error recovery | High | Syntax errors interrupt analysis |
| Watch mode | Medium | File change monitoring |
| LSP features (rename, references, code actions) | Medium | Better IDE experience |
| Generic class instantiation inference | Medium | Better type inference |
| Variance checking | Low | Covariance/contravariance |

## Current State

- **Tests**: 137 passing
- **Maturity**: ~50-60% (realistic assessment)
- **Strengths**: Core type system, PEP compliance, basic LSP
- **Weaknesses**: Limited stdlib coverage, no error recovery, incomplete LSP
