---
change: agent-framework
date: 2026-01-19
---

# Clarifications

## Q1: 使用場景
- **Question**: 這個 agent framework 的主要使用場景是什麼？
- **Answer**: 通用型 agent 框架 + workflow 導向 + type-safe AI 應用（三者兼具）
- **Rationale**: 需要同時滿足多種場景需求，類似 LangChain 的通用性、LangGraph 的 workflow 能力，以及 PydanticAI 的型別安全特性

## Q2: 核心功能
- **Question**: 希望支援哪些核心功能？
- **Answer**: 全部選擇
  - Tool/Function calling：Agent 可以調用外部工具和函數
  - Multi-agent 協作：多個 agent 之間的協調和通訊
  - Memory 管理：對話歷史、長期記憶、知識庫整合
  - Workflow orchestration：複雜的任務流程編排和狀態機
- **Rationale**: 需要完整的 agent 框架能力

## Q3: LLM Provider 支援
- **Question**: 希望支援哪些 LLM provider？
- **Answer**:
  - OpenAI (GPT-4/GPT-3.5)
  - Anthropic Claude (Claude 3.5 Sonnet/Opus)
  - 多 provider 支援（統一介面）
- **Rationale**: 支援主流 LLM provider 並提供統一介面，方便切換和比較

## Q4: 架構設計偏好
- **Question**: 架構設計偏好？
- **Answer**: 功能完整
- **Rationale**: 提供豐富的內建功能和整合，減少使用者需要自行實作的部分

## Q5: 整合需求
- **Question**: 是否需要與現有 ouroboros 系統整合？
- **Answer**: 深度整合
- **Rationale**: 與 ouroboros 系統緊密結合，充分利用現有的基礎設施和能力

## Q6: 開發優先順序
- **Question**: 希望的開發優先順序？
- **Answer**: Core + Tool calling 優先
- **Rationale**: 先建立核心架構和 tool calling 機制，這是 agent 框架的基礎，再逐步擴展其他功能

## Q7: API 設計風格
- **Question**: 希望的 API 設計風格？
- **Answer**: Functional（函數式風格）
- **Rationale**: 簡潔易用，符合 Python 的現代開發習慣

## Q8: 專案設置
- **Question**: 專案名稱和放置位置？
- **Answer**: ouroboros-agent
- **Rationale**: 作為 ouroboros 家族的一部分，保持命名一致性

## Git Workflow
- **Question**: 希望使用哪種 Git workflow？
- **Answer**: 新分支（git checkout -b agentd/agent-framework）
- **Rationale**: 在獨立分支上進行開發，便於管理和 code review
