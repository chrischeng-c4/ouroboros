# Claude Provider Implementation TODO

## Status: IN PROGRESS (Phase 2)

The Claude provider implementation has been started but requires completion.

### What's Done

- ✅ Created claude.rs with basic structure
- ✅ Added ClaudeProvider struct
- ✅ Added NotImplemented error variant to LLMError
- ✅ Exported ClaudeProvider from lib.rs
- ✅ Defined Claude API types (ClaudeMessage, ClaudeContent, ClaudeTool, etc.)
- ✅ Implemented provider_name() and supported_models()
- ✅ Added message conversion logic (Role → Claude format)

### What Needs Fixing

1. **HTTP Client Usage** - Need to use RequestBuilder properly:
   ```rust
   // Current (broken):
   self.client.post("/v1/messages").header(...).json(...)

   // Should be:
   let request = RequestBuilder::new(HttpMethod::Post, "/v1/messages")
       .header("x-api-key", &self.api_key)
       .header("anthropic-version", "2023-06-01")
       .json_value(body);
   self.client.execute_builder(request).await?
   ```

2. **CompletionResponse Construction**:
   - Fix `usage` field: should be `TokenUsage`, not `Option<TokenUsage>`
   - Add `metadata` field: empty HashMap
   ```rust
   Ok(CompletionResponse {
       content,
       finish_reason,
       model: claude_response.model,
       tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
       usage: TokenUsage {  // Not Option!
           prompt_tokens: claude_response.usage.input_tokens as u32,
           completion_tokens: claude_response.usage.output_tokens as u32,
           total_tokens: (claude_response.usage.input_tokens + claude_response.usage.output_tokens) as u32,
       },
       metadata: HashMap::new(),  // Add this field
   })
   ```

3. **Tool Conversion**:
   - `convert_tool()` needs to match the actual tool format used
   - Check what format request.tools uses (likely `Vec<serde_json::Value>`)

4. **Streaming** (Phase 2 later):
   - complete_stream() currently returns NotImplemented
   - Need to handle Claude's SSE (Server-Sent Events) streaming format

### Compilation Errors to Fix

```
error[E0063]: missing field `metadata` in initializer of `CompletionResponse`
error[E0308]: mismatched types - expected `TokenUsage`, found `Option<TokenUsage>`
error[E0599]: no method named `header` found for type (RequestBuilder issue)
```

### Testing Plan

Once fixed, test with:
1. Simple completion request (no tools)
2. Tool-based request
3. Different Claude models (3.5 Sonnet, Opus, Haiku)
4. Error handling (invalid API key, rate limits)

### Next Steps

1. Fix the compilation errors listed above
2. Build and test: `cargo build -p ouroboros-agent-llm`
3. Add Python bindings in `crates/ouroboros/src/agent/py_llm.rs`:
   ```rust
   #[pyclass(name = "Claude")]
   pub struct PyClaude {
       inner: Arc<ClaudeProvider>,
   }
   ```
4. Update integration tests to support Claude
5. Update examples to demonstrate Claude usage

### Reference

- Claude API docs: https://docs.anthropic.com/claude/reference/messages
- Existing OpenAI implementation: `crates/ouroboros-agent-llm/src/openai.rs`
- HTTP client docs: `crates/ouroboros-http/src/client.rs`
