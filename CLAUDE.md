# CLAUDE.md - Implementation Essentials

<!-- agentd:start -->
## Agentd: Spec-Driven Development

**IMPORTANT**: Do NOT make direct code changes. Use the SDD workflow below.

| Skill | Purpose |
|-------|---------|
| `/agentd:proposal` | Generate proposal with Gemini |
| `/agentd:challenge` | Review proposal with Codex |
| `/agentd:reproposal` | Refine based on feedback |
| `/agentd:implement` | Implement the change |
| `/agentd:review` | Run tests and code review |
| `/agentd:fix` | Fix issues from review |
| `/agentd:archive` | Archive completed change |

Start with: `/agentd:proposal <id> "<description>"`
<!-- agentd:end -->


## Abbreviation
- ob: obouroboros
- obpg: ouroboros-postgres 
- obqc: ouroboros-qc

## Limitaion

No big file; If file lines ≥ 1000, must split; If file lines ≥ 500, consider split.
