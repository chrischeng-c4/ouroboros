# Agent Evaluation - Prompt Template System

## 概述 / Overview

LLM-as-judge 系統現在支援靈活的 prompt template 系統，讓你可以選擇不同的 prompt engineering 技術來優化評估品質。

The LLM-as-judge system now supports a flexible prompt template system, allowing you to choose different prompt engineering techniques to optimize evaluation quality.

## 可用的模板 / Available Templates

### 1. Basic Template (`llm_judge_basic`)

**用途 / Use Case**: 一般評估、快速迭代 / General evaluation, fast iteration
**Temperature**: 0.0 (deterministic)
**適用場景 / Best For**: 初期開發、快速反饋 / Initial development, quick feedback

```rust
let config = LLMJudgeConfig::default()
    .with_template("llm_judge_basic");
```

### 2. Few-Shot Template (`llm_judge_few_shot`)

**用途 / Use Case**: 提高評估一致性 / Improved consistency across evaluations
**Temperature**: 0.0 (deterministic)
**適用場景 / Best For**: 當 `eval_consistency < 0.8` / When eval_consistency < 0.8
**特點 / Features**: 包含 3 個校準範例 / Includes 3 calibration examples

```rust
let config = LLMJudgeConfig::default()
    .with_template("llm_judge_few_shot");
```

**Examples included**:
- Perfect answer (2+2=4)
- Slightly verbose but correct answer
- Failed to provide required information

### 3. Chain-of-Thought Template (`llm_judge_cot`)

**用途 / Use Case**: 可解釋的評估，包含推理過程 / Explainable evaluation with reasoning
**Temperature**: 0.0 (deterministic)
**適用場景 / Best For**: Debugging, understanding failures, accuracy < 0.85
**特點 / Features**: 回傳 step-by-step 推理過程 / Returns step-by-step reasoning in response

```rust
let config = LLMJudgeConfig::default()
    .with_template("llm_judge_cot");
```

**Reasoning structure**:
```json
{
  "reasoning": {
    "step1": "Analysis for each criterion...",
    "step2": "Overall quality assessment...",
    "step3": "Final decision rationale..."
  },
  "scores": { ... },
  "feedback": "Summary of evaluation"
}
```

### 4. Self-Consistency Template (`llm_judge_self_consistency`)

**用途 / Use Case**: 高可靠性評估 / High-reliability evaluation
**Temperature**: 0.7 (sampling)
**適用場景 / Best For**: Critical evaluation, false positive rate > 5%
**特點 / Features**: 使用多次採樣 (n=5) + majority voting / Use with multiple samples + majority voting
**注意 / Note**: Higher cost but more reliable

```rust
let config = LLMJudgeConfig::default()
    .with_template("llm_judge_self_consistency")
    .with_temperature(0.7);

// Run multiple times and use majority voting
let mut evaluations = vec![];
for _ in 0..5 {
    let result = judge.evaluate(input, expected, actual).await?;
    evaluations.push(result);
}
// Implement majority voting logic
```

## 使用方式 / Usage

### Rust API

```rust
use ouroboros_qc::agent_eval::{LLMJudge, LLMJudgeConfig};

// Create configuration with specific template
let config = LLMJudgeConfig::new("gpt-4o-mini", "openai")
    .with_template("llm_judge_few_shot")
    .with_criteria(vec![
        QualityCriterion::new("accuracy", "Is the response correct?"),
        QualityCriterion::new("relevance", "Does it answer the question?"),
    ]);

// Create judge with custom template directory
let judge = LLMJudge::with_template_dir(config, "templates/llm_judge")?;

// Evaluate
let scores = judge.evaluate(input, Some(expected), actual).await?;
```

### Python API

```python
from ouroboros.agent_eval import AgentEvaluator, LLMJudgeConfig

# Configure LLM judge with specific template
llm_judge_config = LLMJudgeConfig(
    model="gpt-4o-mini",
    provider="openai",
    temperature=0.0,
    template_name="llm_judge_few_shot",  # Choose template
    template_version="1.0.0",  # Optional: specify version
)

# Create evaluator
evaluator = AgentEvaluator(
    test_cases=test_cases,
    enable_llm_judge=True,
    llm_judge_config=llm_judge_config,
)

# Evaluate
report = await evaluator.evaluate(agent_fn)
```

## 決策矩陣 / Decision Matrix

何時使用哪個模板？/ When to use which template?

| 情況 / Scenario | 推薦模板 / Recommended Template | 原因 / Reason |
|-----------------|--------------------------------|---------------|
| 快速開發與迭代 / Fast development & iteration | `llm_judge_basic` | 最快、最簡單 / Fastest, simplest |
| 評估結果不一致 / Inconsistent evaluation results | `llm_judge_few_shot` | 提供校準範例 / Provides calibration |
| 需要理解為什麼失敗 / Need to understand why it fails | `llm_judge_cot` | 提供推理過程 / Provides reasoning |
| 關鍵評估、高可靠性需求 / Critical evaluation, high reliability | `llm_judge_self_consistency` | 多次採樣投票 / Multiple sampling + voting |
| 偵錯與分析 / Debugging & analysis | `llm_judge_cot` | 可解釋性 / Explainability |
| 評估準確率 < 85% / Evaluation accuracy < 85% | `llm_judge_cot` or `llm_judge_few_shot` | 改善準確性 / Improve accuracy |
| False positive rate > 5% | `llm_judge_self_consistency` | 更嚴格驗證 / Stricter validation |

## 自訂模板 / Custom Templates

你可以建立自己的 YAML 模板 / You can create your own YAML templates:

```yaml
# templates/llm_judge/my_custom_template.yaml
name: my_custom_template
version: "1.0.0"
description: "Custom evaluation template for specific domain"

system_role: "You are an expert evaluator specialized in [domain]..."

# Optional: Few-shot examples
examples:
  - input: "Example input"
    output: |
      {
        "scores": {"criterion1": 1.0},
        "feedback": "Perfect example"
      }
    explanation: "Why this is a good example"

sections:
  - title: "Input"
    content: "{{input}}"

  - title: "Expected Output"
    content: "{{expected}}"
    optional: true
    condition: "has_expected"

  - title: "Actual Output"
    content: "{{actual}}"

  - title: "Evaluation Criteria"
    content: |
      {{criteria}}

  - title: "Instructions"
    content: |
      Your custom evaluation instructions here.

      Evaluate each criterion:
      1. First step...
      2. Second step...

      Respond in JSON format:
      ```json
      {
        "scores": {
          {{criteria_keys}}
        },
        "feedback": "Brief explanation"
      }
      ```

metadata:
  technique: "custom"
  temperature: "0.0"
  use_case: "specialized_domain_evaluation"
  author: "Your Name"
  tags: ["domain-specific", "custom"]
```

### 變數替換 / Variable Substitution

模板支援以下變數 / Templates support the following variables:

- `{{input}}` - 原始使用者輸入 / Original user input
- `{{expected}}` - 預期輸出（可選）/ Expected output (optional)
- `{{actual}}` - 實際 agent 輸出 / Actual agent output
- `{{criteria}}` - 格式化的評估標準 / Formatted evaluation criteria
- `{{criteria_keys}}` - JSON schema 的標準 keys / Criterion keys for JSON schema
- `{{has_expected}}` - 是否有預期輸出 / Whether expected output exists

### 條件區段 / Conditional Sections

```yaml
sections:
  - title: "Expected Output"
    content: "{{expected}}"
    optional: true
    condition: "has_expected"  # Only shown if has_expected is set
```

## Template 載入順序 / Template Loading Priority

LLMJudge 按以下順序嘗試載入模板 / LLMJudge tries to load templates in this order:

1. 指定的自訂目錄（如果使用 `with_template_dir()`）/ Custom directory if using `with_template_dir()`
2. `./templates/llm_judge/` (相對於當前目錄) / Relative to current directory
3. `crates/ouroboros-qc/templates/llm_judge/` (用於測試) / For testing
4. 後退至 legacy hardcoded prompt / Fallback to legacy hardcoded prompt

## 效能考量 / Performance Considerations

### Cost

| Template | API Calls | Tokens (approx) | Cost (gpt-4o-mini) |
|----------|-----------|-----------------|---------------------|
| Basic | 1 | 500-800 | $0.0001-0.0002 |
| Few-Shot | 1 | 800-1200 | $0.0002-0.0003 |
| CoT | 1 | 600-1000 | $0.0001-0.0003 |
| Self-Consistency | 5-7 | 3000-5000 | $0.0006-0.0012 |

### Latency

| Template | Latency (P95) |
|----------|---------------|
| Basic | ~500ms |
| Few-Shot | ~600ms |
| CoT | ~700ms |
| Self-Consistency | ~3500ms (5x sampling) |

## 範例 / Examples

完整範例請參考 / See complete examples at:
- `examples/agent_eval_llm_judge_templates.py` - Python usage examples
- `crates/ouroboros-qc/tests/integration/agent_eval_templates_test.rs` - Rust integration tests

## 版本管理 / Version Management

Template registry 支援版本管理 / Template registry supports version management:

```rust
// Use specific version
let config = LLMJudgeConfig::default()
    .with_template_version("llm_judge_basic", "1.0.0");

// Use latest version (default)
let config = LLMJudgeConfig::default()
    .with_template("llm_judge_basic");

// List available templates
let mut registry = PromptRegistry::new();
registry.load_from_directory("templates/llm_judge")?;
let templates = registry.list_templates();
for template_name in templates {
    let versions = registry.list_versions(&template_name);
    println!("{}: {:?}", template_name, versions);
}
```

## 最佳實踐 / Best Practices

### 1. 開發階段 / Development Phase

```python
# Start with basic template for fast iteration
llm_judge_config = LLMJudgeConfig(
    model="gpt-4o-mini",  # Cheap & fast
    template_name="llm_judge_basic",
    temperature=0.0,
)
```

### 2. 測試階段 / Testing Phase

```python
# Use few-shot for consistency
llm_judge_config = LLMJudgeConfig(
    model="gpt-4o",  # More accurate
    template_name="llm_judge_few_shot",
    temperature=0.0,
)
```

### 3. 生產環境 / Production

```python
# For critical paths, use self-consistency
llm_judge_config = LLMJudgeConfig(
    model="gpt-4o",
    template_name="llm_judge_self_consistency",
    temperature=0.7,  # Enable sampling
)

# Run multiple evaluations
results = []
for _ in range(5):
    result = await evaluator.evaluate(agent_fn)
    results.append(result)

# Majority voting
final_score = majority_vote(results)
```

### 4. 偵錯與分析 / Debugging & Analysis

```python
# Use CoT for explainability
llm_judge_config = LLMJudgeConfig(
    model="gpt-4o",
    template_name="llm_judge_cot",
    temperature=0.0,
)

result = await evaluator.evaluate(agent_fn)
print(result.quality_scores.feedback)  # See reasoning
```

## 疑難排解 / Troubleshooting

### Template Not Found

```python
# Error: Template 'llm_judge_basic' not found
```

**解決方案 / Solution**:
1. 確認模板目錄存在 / Verify template directory exists: `templates/llm_judge/`
2. 確認 YAML 檔案存在 / Verify YAML files exist
3. 使用自訂路徑 / Use custom path:
```rust
let judge = LLMJudge::with_template_dir(config, "/path/to/templates")?;
```

### Variable Not Substituted

```python
# Prompt contains {{variable}} instead of actual value
```

**解決方案 / Solution**:
1. 確認變數名稱正確 / Verify variable name is correct
2. 檢查 context 是否設定變數 / Check if context sets the variable
3. 查看 `PromptContext::set()` 呼叫 / Review `PromptContext::set()` calls

### Low Evaluation Consistency

```python
# Same input gets different scores each time
```

**解決方案 / Solution**:
1. 設定 `temperature=0.0` / Set `temperature=0.0`
2. 使用 few-shot template / Use few-shot template
3. 考慮 self-consistency template / Consider self-consistency template

## 參考資料 / References

- [Prompt Engineering Guide](https://www.promptingguide.ai/)
- [Chain-of-Thought Prompting](https://arxiv.org/abs/2201.11903)
- [Self-Consistency](https://arxiv.org/abs/2203.11171)
- [Few-Shot Learning](https://arxiv.org/abs/2005.14165)
