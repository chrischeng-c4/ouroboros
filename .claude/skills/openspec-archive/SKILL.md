---
name: openspec:archive
description: Archive a deployed OpenSpec change by merging spec deltas into main specs and moving to archive. Use when the user says "deployed", "merged to main", "in production", "released", "archive this", or confirms the change is live. Requires completed implementation and deployment.
user-invocable: true
category: OpenSpec
tags: [openspec, archive, deployment]
---

# OpenSpec Archive Skill

Archive deployed changes by merging spec deltas into main specifications.

## When to Use This Skill

Automatically use when the user:
- Confirms deployment: "**deployed**", "**merged to main**", "**in production**", "**released**"
- Says "**archive this change**", "**mark as complete**", "**finalize the change**"
- All tasks are complete and user confirms it's live

**Do NOT use** if:
- Implementation is still in progress
- Tests are failing
- Change hasn't been deployed yet
- User hasn't confirmed production deployment

---

## Instructions

### Guardrails
- Favor straightforward, minimal implementations; add complexity only when clearly required
- Keep changes tightly scoped to the requested outcome
- Refer to `openspec/AGENTS.md` if you need OpenSpec conventions

### Steps

1. **Verify completion**
   - Confirm all tasks in `tasks.md` are marked `[x]`
   - Verify tests pass and deployment is confirmed
   - If incomplete, ask user to confirm before proceeding

2. **Merge spec deltas into main specs**
   - Navigate to `changes/<id>/specs/`
   - For each spec delta file:
     - **ADDED**: Insert new requirements/scenarios into `openspec/specs/<capability>/spec.md`
     - **MODIFIED**: Update existing requirements/scenarios in main spec
     - **REMOVED**: Delete requirements/scenarios from main spec
   - Preserve existing spec structure and formatting

3. **Move to archive**
   - Create `.archive/` directory if missing: `mkdir -p openspec/changes/.archive`
   - Move change directory: `openspec/changes/<id>/` → `openspec/changes/.archive/<id>/`
   - Preserve all files (proposal.md, tasks.md, design.md, spec deltas)

4. **Update project context**
   - If the change introduced new conventions, constraints, or architectural patterns:
     - Update `openspec/project.md` with new information
   - Document any project-wide impacts

5. **Validate integrity**
   - Run `openspec validate --strict` on all specs
   - Fix any validation errors introduced during merge
   - Ensure specs remain consistent and valid

---

## Examples

### Example: User Confirmation
```
User: "The feature is deployed and working in production"
```

**Your response:**
1. Verify all tasks in `tasks.md` are `[x]`
2. Merge spec deltas from `changes/<id>/specs/` into `openspec/specs/`
3. Move `changes/<id>/` to `changes/.archive/<id>/`
4. Update `project.md` if needed
5. Run `openspec validate --strict`
6. Report completion with summary

### Example: Merging Spec Deltas

**From**: `changes/add-docker-compose/specs/docker-compose/spec.md`
```markdown
## ADDED Requirements

### Requirement: Docker Compose File Detection
...

### Requirement: Service Management
...
```

**To**: `openspec/specs/docker-compose/spec.md`
```markdown
## Requirements

### Requirement: Docker Compose File Detection
...

### Requirement: Service Management
...

(previous existing requirements follow)
```

---

## Reference

- Find spec deltas: `rg "## ADDED|## MODIFIED|## REMOVED" openspec/changes/<id>/specs`
- Inspect change structure: `openspec show <id> --json`
- Validation help: `openspec show <spec> --type spec`
- Full workflow: `openspec/AGENTS.md`

---

## After Completion

Report to user:
1. Confirm spec deltas merged successfully
2. Show new location: `changes/.archive/<id>/`
3. Confirm validation passed
4. Summarize any project.md updates
5. List affected specs in `openspec/specs/`
