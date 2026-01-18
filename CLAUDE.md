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
<!-- agentd:end -->

## Abbreviation
- ob: obouroboros
- obpg: ouroboros-postgres 
- obqc: ouroboros-qc

## Limitaion

No big file; If file lines ≥ 1000, must split; If file lines ≥ 500, consider split.
