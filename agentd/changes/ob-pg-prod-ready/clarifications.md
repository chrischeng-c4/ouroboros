---
change: ob-pg-prod-ready
date: 2026-01-19
---

# Clarifications

## Q1: Scope
- **Question**: 這次要處理的範圍是什麼？
- **Answer**: Full Roadmap (P0 + P1 + P2)
- **Rationale**: 完整準備 ob-pg 用於小型/次要專案的生產部署，涵蓋所有優先級項目

## Q2: Test Fix Strategy
- **Question**: 2 個失敗測試 (returning/DECIMAL) 的處理方式？
- **Answer**: Fix Implementation
- **Rationale**: 修改 Rust 實作讓測試通過，確保行為符合預期

## Q3: Panic Audit Strategy
- **Question**: unwrap()/expect() 審計策略？
- **Answer**: Critical Path Only
- **Rationale**: 只處理 CRUD/連線/交易等關鍵路徑，平衡工作量與風險

## Q4: Git Workflow
- **Question**: Git 工作流程偏好？
- **Answer**: New Branch (agentd/ob-pg-prod-ready)
- **Rationale**: 隔離變更，便於審查和回滾

---

# Context

## Current State (from prior analysis)

### Known Issues
1. **Test Failures (2/11)**:
   - `test_execute_insert_with_returning`: Returns row count instead of list
   - `test_execute_aggregate_query`: DECIMAL/NUMERIC type conversion issue

2. **Unimplemented Features**:
   - `loading.py:530` - Deferred column loading
   - `loading.py:672` - JOIN building pattern
   - `loading.py:777` - Subquery strategy
   - `query_ext.py:331` - `any_()` raises NotImplementedError
   - `query_ext.py:377` - `has()` raises NotImplementedError
   - `transactions.py:244` - read_only/deferrable support

3. **Panic Risks**:
   - 506 unwrap()/expect() in query builder
   - 158 in main codebase (critical path audit needed)

4. **Missing Documentation**:
   - No load testing results
   - No operational runbook
   - No monitoring guide

## Priority Matrix

| Priority | Items |
|----------|-------|
| P0 (Critical) | Fix 2 test failures, Critical path panic audit, Connection error handling |
| P1 (High) | Implement any_()/has(), read_only transaction, Basic ops docs |
| P2 (Nice to Have) | Prepared statement caching, Slow query logging, Deferred column loading |
