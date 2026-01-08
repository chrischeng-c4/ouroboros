---
name: openspec:apply
description: Implement an approved OpenSpec change by working through tasks sequentially, updating the checklist as work progresses. Use when the user says "implement it", "proceed", "apply the proposal", "start implementation", or approves a proposal. Requires an existing validated proposal.
user-invocable: true
category: OpenSpec
tags: [openspec, apply, implementation]
---

# OpenSpec Apply Skill

Implement approved change proposals following the OpenSpec workflow.

## When to Use This Skill

Automatically use when the user:
- Approves a proposal: "**implement it**", "**proceed**", "**looks good**"
- Says "**apply the proposal**", "**start implementation**", "**build this**"
- References a change ID: "**implement add-docker-compose**"
- Says "**continue with the implementation**" after proposal creation

**Do NOT use** if:
- Proposal validation failed
- User hasn't explicitly approved the proposal
- Still discussing or refining requirements

---

## Instructions

### Guardrails
- Favor straightforward, minimal implementations; add complexity only when clearly required
- Keep changes tightly scoped to the requested outcome
- Refer to `openspec/AGENTS.md` if you need OpenSpec conventions

### Steps

**Use TodoWrite to track progress through these steps:**

1. **Load the proposal**
   - Read `changes/<id>/proposal.md` to understand scope and rationale
   - Read `changes/<id>/design.md` (if present) for architectural guidance
   - Read `changes/<id>/tasks.md` to see the implementation checklist

2. **Work through tasks sequentially**
   - Complete tasks in order (respect dependencies)
   - Keep edits minimal and focused on the requested change
   - Follow State-First and Definition of Done principles
   - Run tests after each significant change

3. **Update task checklist in real-time**
   - Mark `- [x]` immediately after completing each task
   - Keep `tasks.md` synchronized with actual progress
   - Don't batch updates—update as you go

4. **Validate completion**
   - Ensure all checklist items are marked `[x]`
   - Run `cargo test`, `pnpm test`, `cargo clippy`
   - Verify Definition of Done checklist (dev-docs/workflow/definition-of-done.md)
   - Confirm no MOCK data in production code

5. **Reference additional context**
   - Use `openspec show <id> --json --deltas-only` for proposal details
   - Use `openspec list` to check other related changes

---

## Examples

### Example: User Approval
```
User: "The proposal looks good, proceed with implementation"
```

**Your response:**
1. Load `changes/<id>/proposal.md` and `tasks.md`
2. Create TodoWrite with all tasks from tasks.md
3. Implement tasks sequentially
4. Update both TodoWrite AND tasks.md as you progress
5. Run tests after each task
6. When complete, ask user if ready to deploy (triggers `openspec-archive`)

### Example: Task Progress
```
TodoWrite:
✓ Create docker-compose.yml schema types
✓ Add docker-compose parser in packages/core
⚙️ Add napi-rs bindings for docker-compose commands
☐ Add preload bridge functions
☐ Create React components for docker-compose UI
☐ Write E2E tests
```

Update `tasks.md` to match:
```markdown
- [x] Create docker-compose.yml schema types
- [x] Add docker-compose parser in packages/core
- [x] Add napi-rs bindings for docker-compose commands
- [ ] Add preload bridge functions
...
```

---

## Reference

- State-First Architecture: `dev-docs/architecture/01-state-first.md`
- Definition of Done: `dev-docs/workflow/definition-of-done.md`
- Testing Guide: `dev-docs/workflow/testing-guide.md`
- OpenSpec workflow: `openspec/AGENTS.md`

---

## After Completion

When all tasks are done:
1. Confirm all tests pass
2. Verify Definition of Done checklist
3. Show summary of changes made
4. Ask user: "Is this deployed to production?" (triggers `openspec-archive`)
