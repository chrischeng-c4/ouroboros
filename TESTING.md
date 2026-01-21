# ouroboros-agent Testing Guide

## Quick Start / 快速開始

### 1. Set OpenAI API Key / 設定 OpenAI API Key

```bash
export OPENAI_API_KEY="sk-..."
```

### 2. Build Python Bindings / 編譯 Python 綁定

```bash
uv run --with maturin maturin develop
```

**Expected Output / 預期輸出**:
```
✏️  Setting installed package as editable
🛠  Installed ouroboros-kit-0.1.0
```

### 3. Run Integration Tests / 執行整合測試

```bash
uv run python python/examples/agent/integration_test.py
```

---

## Test Suite Overview / 測試套件概覽

The integration test validates all agent framework functionality:

整合測試驗證所有 agent 框架功能：

### Test 1: Module Imports / 模組導入
- ✅ Import Agent, OpenAI, Tool, ToolRegistry, get_global_registry
- Validates Python bindings are working

### Test 2: OpenAI Provider / OpenAI 提供者
- ✅ Create OpenAI provider with API key
- ✅ Check supported models
- Validates LLM integration

### Test 3: Basic Agent Execution / 基本 Agent 執行
- ✅ Create agent with system prompt
- ✅ Send simple query to OpenAI
- ✅ Get response with token usage
- Validates end-to-end agent execution

### Test 4: Tool Creation & Registration / 工具建立與註冊
- ✅ Create async tool
- ✅ Register in ToolRegistry
- ✅ Verify registration
- Validates tool structure

### Test 5: Tool Execution / 工具執行
- ✅ Execute sync Python function
- ✅ Execute async Python function
- ✅ Handle complex return values
- **Validates Phase 2 critical feature** (Python function tool wrapping)

### Test 6: Advanced Queries / 進階查詢
- ✅ Multiple agent queries
- ✅ Different models (gpt-3.5-turbo)
- ✅ Parameter variations
- Validates production usage patterns

---

## Expected Output / 預期輸出

```
🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀
  ouroboros.agent Integration Test Suite
  Testing Phase 1 (MVP) + Phase 2 (Tool Execution)
🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀 🚀

ℹ API key: sk-proj-uY...iZQA

======================================================================
  Test 1: Module Imports
======================================================================
✓ Imported Agent
✓ Imported OpenAI
✓ Imported Tool
✓ Imported ToolRegistry
✓ Imported get_global_registry

======================================================================
  Test 2: OpenAI Provider
======================================================================
✓ Created OpenAI provider: openai
✓ Supports 8 models
ℹ Models: gpt-4, gpt-3.5-turbo, gpt-4-turbo, gpt-4o, gpt-4o-mini...

======================================================================
  Test 3: Basic Agent Execution
======================================================================
✓ Created agent: test_agent
ℹ Agent ID: test_agent
ℹ Max turns: 10
ℹ Tool timeout: 30s
ℹ Sending query: 'What is 2+2? Answer in 3 words or less.'
✓ Got response in 1.23s
ℹ Content: 2 + 2 = 4
ℹ Model: gpt-4
ℹ Finish reason: stop
ℹ Tokens: 23 (prompt: 15, completion: 8)

======================================================================
  Test 4: Tool Creation & Registration
======================================================================
✓ Created tool: calculate
ℹ Description: Evaluate a mathematical expression
ℹ Parameters: 1
✓ Registered tool (registry count: 1)
✓ Tool found in registry

======================================================================
  Test 5: Tool Execution
======================================================================
✓ Sync tool executed: Hello, Alice!
✓ Async tool executed: 56
✓ Complex data tool executed

======================================================================
  Test 6: Advanced Queries
======================================================================
ℹ Query: Name a programming language
✓ Response (0.87s): Python
ℹ Tokens used: 12
ℹ Query: What is the capital of France? One word answer.
✓ Response (0.65s): Paris
ℹ Tokens used: 10
ℹ Query: Calculate 15 * 3. Just give the number.
✓ Response (0.71s): 45
ℹ Tokens used: 11

======================================================================
  Test Summary
======================================================================
✓ PASS - Module Imports
✓ PASS - OpenAI Provider
✓ PASS - Basic Agent
✓ PASS - Tool Creation
✓ PASS - Tool Execution
✓ PASS - Advanced Queries

Total: 6/6 passed (100.0%)
Duration: 4.52s

🎉 All tests passed! Agent framework is working correctly.
✅ Phase 1 (MVP): Complete
✅ Phase 2 (Tool Execution): Complete
```

---

## Individual Examples / 個別範例

### Simple Agent Example

```bash
uv run python python/examples/agent/simple_agent.py
```

**What it does / 功能**:
- Creates OpenAI provider
- Creates agent with system prompt
- Runs 3 example queries with different models/parameters
- Shows response metadata (tokens, model, finish reason)

### Tool Agent Example

```bash
uv run python python/examples/agent/tool_agent.py
```

**What it does / 功能**:
- Creates 3 tools (search, weather, calculator)
- Registers tools in global registry
- **Executes tools directly** (demonstrates Phase 2 tool wrapping)
- Shows tool execution results

---

## Unit Tests / 單元測試

Basic unit tests (no API key required):

```bash
uv run pytest python/tests/agent/test_agent_basic.py -v
```

**Tests / 測試**:
- OpenAI provider creation
- Tool creation with parameters
- ToolRegistry operations (register, unregister, contains, clear)
- Agent configuration

Tool execution tests (validates Phase 2):

```bash
uv run pytest python/tests/agent/test_tool_execution.py -v
```

**Tests / 測試**:
- Sync function tool execution
- Async function tool execution
- String/integer arguments
- Complex dict returns
- Error handling
- Registry integration

---

## Troubleshooting / 故障排除

### Error: OPENAI_API_KEY not set

```bash
export OPENAI_API_KEY="sk-..."
```

### Error: Module 'ouroboros.agent' not found

Rebuild Python bindings:

```bash
uv run --with maturin maturin develop
```

### Error: maturin build failed

Check if pre-existing issues in other crates (postgres, api):

```bash
# Try building just agent crates
cargo build -p ouroboros-agent-core -p ouroboros-agent-llm -p ouroboros-agent-tools
```

Currently disabled in `pyproject.toml` due to compilation errors:
- `postgres`: ExtractedValue::Decimal type mismatch
- `api`: TypeDescriptor missing BSON patterns

### Rate Limiting / API Errors

If you get rate limit errors from OpenAI:
- Use gpt-3.5-turbo (cheaper, higher limits)
- Add delays between requests
- Check your OpenAI API quota

---

## What's Being Tested / 測試內容

### ✅ Phase 1 (MVP) - Complete

| Feature | Status | Test |
|---------|--------|------|
| OpenAI integration | ✅ | Test 2, 3 |
| Basic agent execution | ✅ | Test 3 |
| Tool structure | ✅ | Test 4 |
| Python bindings (PyO3) | ✅ | Test 1 |
| Response metadata | ✅ | Test 3, 6 |

### ✅ Phase 2 (Tool Execution) - Complete

| Feature | Status | Test |
|---------|--------|------|
| Python function wrapping | ✅ | Test 5 |
| Sync function execution | ✅ | Test 5 |
| Async function execution | ✅ | Test 5 |
| Tool registration | ✅ | Test 4 |
| Complex return values | ✅ | Test 5 |
| GIL-free execution | ✅ | Test 5 |

### ❌ Phase 2 (Remaining) - Pending

| Feature | Status | Priority |
|---------|--------|----------|
| Anthropic Claude provider | ❌ | High |
| Streaming responses | ❌ | Critical |
| Human-in-the-loop | ❌ | Critical |
| Persistent memory (MongoDB) | ❌ | High |

---

## Performance Validation / 性能驗證

The integration test measures:
- **Latency**: Response time per query (~1-2s for GPT-4)
- **Token usage**: Tracks prompt/completion/total tokens
- **GIL release**: Tools execute outside Python GIL (async)

Expected performance:
- **Simple queries**: 0.5-1.5s (gpt-3.5-turbo)
- **Complex queries**: 1-3s (gpt-4)
- **Tool execution**: <100ms overhead
- **Memory**: Efficient Arc-based state sharing

---

## Next Steps / 下一步

After successful testing:

1. **Add Anthropic Claude Provider** (Gap #1)
   - Support more LLM providers
   - Reduce vendor lock-in

2. **Implement Streaming** (Gap #3)
   - Real-time token streaming
   - Better UX for long responses

3. **Human-in-the-Loop** (Gap #4)
   - Tool call approval
   - Conditional approval logic

4. **Persistent Memory** (Gap #5)
   - MongoDB backend
   - Long-term conversation history

---

## API Costs / API 成本

Estimated costs for testing (varies by model):

| Model | Cost per 1K tokens | Integration Test | All Examples |
|-------|-------------------|------------------|--------------|
| gpt-4 | ~$0.03 | ~$0.10 | ~$0.20 |
| gpt-3.5-turbo | ~$0.002 | ~$0.01 | ~$0.02 |
| gpt-4-turbo | ~$0.01 | ~$0.03 | ~$0.06 |

**Recommendation / 建議**: Use gpt-3.5-turbo for frequent testing to minimize costs.

---

## Contact / 聯絡

If tests fail or you encounter issues:
1. Check this TESTING.md for troubleshooting
2. Review GAP_ANALYSIS.md for known limitations
3. Check build output for compilation errors

**Status / 狀態**:
- ✅ Phase 1 (MVP): Production-ready
- ✅ Phase 2 (Tool Execution): Production-ready
- 🔄 Phase 2 (Remaining): In progress
