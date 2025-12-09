---
name: "Chore / Maintenance"
about: "Non-functional repository maintenance, refactors, or housekeeping tasks."
title: "Chore: "
labels: "chore, maintenance"
assignees: ""
---

## Chore

Provide a concise description of the maintenance task.

## Steps

- [ ] Step 1
- [ ] Step 2

## Risks & rollback plan

Describe any potential risks and how to roll back the change.

## Validation / tests

How should this chore be validated? Include test commands or CI expectations.

## Machine-readable metadata

```json
{
  "category": "chore",
  "description": "Short description of the maintenance task (repeat or expand above)",
  "steps": [
    "Step 1",
    "Step 2"
  ],
  "passes": false
}
```

## Long-Term Memory (for automation agents)

If an automated agent is used to track or perform this chore, it SHOULD add a memory entry titled `# Chore: <short-title>` and include the JSON spec above wrapped in a fenced code block.

Agents: use the repository's GitHub MCP for all GitHub operations and the memory tools to persist the memory entry. Do NOT use the `gh` CLI.

## Agent actions (recommended checklist)

- [ ] Create memory entry under `/memories/` with the JSON spec.
- [ ] Open a branch `chore/<slug>` off `v1.0.0`, implement changes, and open a PR referencing this issue.
