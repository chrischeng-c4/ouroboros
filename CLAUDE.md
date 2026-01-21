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

### Knowledge Base

System documentation is in `agentd/knowledge/`. Use MCP tools to read:
- `list_knowledge` - List all knowledge files
- `read_knowledge` - Read specific file (e.g., `read_knowledge("00-architecture/index.md")`)
<!-- agentd:end -->

## Abbreviation
- ob: ouroboros
- obpg: ouroboros-postgres
- obqc: ouroboros-qc
- obagent: ouroboros-agent (framework)

## ouroboros-agent Framework

**Competitors**: PydanticAI, LangChain, LangGraph

The ouroboros-agent framework is a high-performance LLM agent framework with deep Rust integration, designed to compete with:
- **PydanticAI**: Type-safe Python agent framework with Pydantic models
- **LangChain**: Popular LLM application framework with extensive integrations
- **LangGraph**: Stateful multi-agent workflow framework from LangChain team

**Key Differentiators**:
- Zero Python Byte Handling (all processing in Rust)
- GIL-free execution for maximum performance
- Deep integration with ouroboros ecosystem (MongoDB, PostgreSQL, HTTP, KV)
- Type-safe Rust core with Pythonic API

## Limitation

No big file; If file lines ≥ 1000, must split; If file lines ≥ 500, consider split.
