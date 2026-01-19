---
change: enhanced-type-inference
date: 2026-01-19
---

# Clarifications

## Q1: Python Path 配置優先級

- **Question**: Python path 配置的來源優先級應該如何排序？
- **Answer**: 配置優先 (pyproject.toml > PYTHONPATH 環境變數 > 自動檢測)
- **Rationale**: 明確配置最優先，適合精確控制。這對 LLM 通過 MCP 修改配置特別重要，確保配置文件是唯一可信來源。

## Q2: 虛擬環境檢測策略

- **Question**: 虛擬環境檢測策略？
- **Answer**: 多方式檢測 (檢查 VIRTUAL_ENV、.venv/、venv/、poetry.lock、pipenv)
- **Rationale**: 覆蓋常見場景，支持多種 Python 項目管理工具。自動檢測對用戶友好，減少配置負擔。

## Q3: pyproject.toml 配置格式兼容性

- **Question**: pyproject.toml 配置格式應該兼容哪些工具？
- **Answer**: 自定義格式 (只用 [tool.argus])
- **Rationale**: **核心理念變化** - 目標是給 LLM 使用，理論上應該讓 LLM 透過 MCP 來設定 pyproject.toml。因此不需要遷就現有工具格式，而是設計最適合 LLM 理解和修改的配置結構。
- **Key Insight**: 添加 MCP 工具讓 LLM 可以直接修改配置文件，而不是要求人工編輯。

## Q4: 跨文件類型解析範圍

- **Question**: 跨文件類型解析的範圍？
- **Answer**: 項目內 + 所有依賴 (包括 site-packages 的完整解析)
- **Rationale**: 功能最全，讓 LLM 可以完整理解項目的類型依賴。性能問題可以通過增量緩存和按需加載解決。

## Additional Context

### 目標定位
- **主要用戶**: LLM (通過 MCP/CLI)
- **使用場景**: 代碼理解、重構、類型檢查
- **關鍵需求**:
  1. 精確的類型信息
  2. 可編程的配置接口
  3. 完整的依賴解析

### MCP 工具需求
建議添加以下 MCP 工具：
- `argus_set_python_paths` - 設置 Python 搜索路徑
- `argus_detect_environment` - 自動檢測環境並更新配置
- `argus_configure_project` - 修改 pyproject.toml [tool.argus] 配置
- `argus_list_modules` - 列出可解析的模組

### Implementation Priority
1. **P0** (必須): Python path 配置和虛擬環境檢測
2. **P0** (必須): 跨文件 import 解析
3. **P1** (高優先): MCP 配置工具
4. **P2** (中優先): site-packages 完整解析
