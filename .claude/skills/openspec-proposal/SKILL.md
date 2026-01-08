---
name: openspec:proposal
description: Generate OpenSpec proposals using Gemini CLI. Gemini explores codebase and creates files directly via WriteFile tool. Cost-efficient with 1M token context.
user-invocable: true
category: OpenSpec
tags: [openspec, proposal, gemini]
---

# OpenSpec Proposal Skill (Gemini-Powered)

Generate specification-driven change proposals using Gemini CLI in headless mode.

## When to Use This Skill

Automatically use when the user:
- Requests a **new feature** or **capability**
- Asks to **add/modify requirements**
- Says "**create a proposal**", "**spec this**", "**plan a feature**"
- Mentions **architecture changes** or **new patterns**
- Describes work that's **non-trivial** (>500 LOC, >5 files, complex logic)

**Do NOT use** for:
- Simple bug fixes (< 100 LOC, single file)
- Documentation-only changes
- Quick tweaks or refactoring

---

## Instructions

### Guardrails
- This skill uses **Gemini CLI** to generate proposals (cost-efficient, large context)
- Gemini will **read code and generate spec files**, but **NOT write implementation code**
- System prompt is in `/GEMINI.md` (OpenSpec Instructions section)
- Keep changes tightly scoped to the requested outcome
- Identify vague or ambiguous details and ask follow-up questions before calling Gemini

### Steps

1. **Preparation**
   - Ask user for `change-id` if not provided (must be verb-led kebab-case: `add-*`, `update-*`, `refactor-*`)
   - Clarify the user's request - ensure you understand requirements
   - The helper script will validate that the change-id doesn't already exist

2. **Call Gemini via helper script**
   Use the helper script to generate the proposal:

   ```bash
   .claude/skills/openspec-proposal/scripts/generate-proposal.sh "<change-id>" "<user-request>"
   ```

   The script will:
   - Validate arguments and check for existing change-id
   - Call Gemini CLI: `gemini /openspec:proposal "<user-request>" -y`
   - Read `GEMINI.md` for system prompt (OpenSpec Instructions section)
   - Explore codebase with context aggregation
   - Create all proposal files directly in `openspec/changes/<change-id>/` using WriteFile tool
   - Run `openspec validate <change-id> --strict`
   - Output structured summary
   - Provide clear error messages if any step fails

3. **Verify and present results**
   Gemini's summary includes all key information. You should:

   a. **Show Gemini's summary** (already formatted)

   b. **Verify files exist**:
   ```bash
   ls -la "openspec/changes/<change-id>/"
   ```

   c. **If validation passed**: Proceed to step 5

   d. **If validation failed**:
      - Show validation errors to user
      - Debug with: `openspec show <change-id> --json --deltas-only`
      - Common issues:
        - Scenario format: Must be `#### Scenario:` (4 hashtags)
        - MODIFIED requirements: Must include full text
        - Missing scenarios: Every requirement needs ≥1 scenario
      - Offer to manually edit files OR re-run Gemini with clarifications

4. **Manual fixes (if needed)**
   If validation fails, you can:
   - Read the problematic file
   - Identify the issue
   - Ask user: "Should I fix this manually or regenerate with Gemini?"
   - If fixing manually: Update the specific file using Edit tool
   - Re-run validation: `openspec validate <change-id> --strict`

5. **Present final summary to user**
   Format:
   ```
   ✅ Proposal Created: <change-id>

   📄 Files:
   - proposal.md (why, what, impact)
   - tasks.md (X implementation tasks)
   - design.md (architecture decisions) [if applicable]
   - specs/<capability>/spec.md (Y requirements, Z scenarios)

   ✅ Validation: PASSED

   📊 Summary:
   - Requirements: A ADDED, B MODIFIED, C REMOVED
   - Affected capabilities: <list>
   - Affected code: <file list>

   ⏭️ Next Steps:
   1. Review the proposal: cat openspec/changes/<change-id>/proposal.md
   2. Inspect details: openspec show <change-id> --json --deltas-only
   3. When approved, say "implement it" to trigger openspec-apply skill
   ```

---

## Examples

### Example: User Request
```
User: "I want to add Docker Compose support to the project management feature"
```

**Your response:**
1. Ask clarifying questions about requirements
2. Create `openspec/changes/add-docker-compose/`
3. Draft spec deltas for new Docker Compose capabilities
4. Create ordered task list
5. Validate with `openspec validate add-docker-compose --strict`
6. Present proposal for approval

### Example: Proposal Structure
```
openspec/changes/add-docker-compose/
├── proposal.md          # Why, what, scope
├── tasks.md             # [ ] Task checklist
├── design.md            # Architecture decisions (if needed)
└── specs/
    └── docker-compose/
        └── spec.md      # ## ADDED Requirements
```

---

## Reference

- Search existing requirements: `rg -n "Requirement:|Scenario:" openspec/specs`
- Explore codebase: `rg <keyword>`, `ls`, or direct file reads
- Validation help: `openspec show <spec> --type spec`
- Full workflow: See `openspec/AGENTS.md`

---

## After Completion

Present the proposal to the user:
1. Summarize the change
2. List spec deltas created
3. Show task count and highlights
4. Confirm validation passed
5. Ask: "Should I proceed with implementation?" (triggers `openspec-apply` skill)
