---
change: validation-custom-validators
date: 2026-01-19
issue: "#20"
---

# Clarifications

## Q1: Implementation Layer
- **Question**: Custom validators 要在哪一層實作？
- **Answer**: Rust only (Recommended)
- **Rationale**: 只在 Rust 實作 trait，Python 透過 PyO3 呼叫。保持效能優勢，避免跨語言複雜度。

## Q2: Validator Types
- **Question**: 要支援哪些 validator 類型？
- **Answer**: Both @field_validator and @model_validator
- **Rationale**: 同時支援單一欄位驗證和跨欄位/整個 model 的驗證，完整對標 Pydantic 功能。

## Q3: Async Support
- **Question**: 是否需要 async validator 支援？
- **Answer**: Yes (Recommended)
- **Rationale**: 支援 async def validator，可做 DB lookup 等非同步操作。

## Q4: Git Workflow
- **Question**: Git workflow 偏好？
- **Answer**: New branch
- **Rationale**: 建立 agentd/validation-custom-validators 分支進行開發。

---

# Summary

實作 Pydantic 風格的 custom validators：
- **Rust trait-based interface**: 可擴展的驗證器系統
- **@field_validator**: 單一欄位驗證，支援 mode='before'/'after'
- **@model_validator**: 跨欄位驗證，支援 mode='before'/'after'
- **Async support**: 支援非同步驗證函數
- **Python integration**: 透過 PyO3 提供 decorator API
