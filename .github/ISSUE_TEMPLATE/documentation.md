---
name: "Documentation"
about: "Improvements, fixes, or additions to documentation."
title: "Docs: "
labels: "documentation, needs-triage"
assignees: ""
---

## Documentation

Provide a concise one-line summary of the documentation change or gap.

## Files / pages affected

List the file paths or documentation pages this change should touch (e.g., `docs/`, `README.md`).

## Suggested change

Describe the proposed change (add examples, fix wording, add diagrams, etc.).

## Acceptance criteria

Describe how reviewers will verify the change is complete.

## Machine-readable metadata

```json
{
  "category": "documentation",
  "description": "Short description of the documentation change (repeat or expand above)",
  "steps": [
    "Identify files to change",
    "Implement docs change",
    "Update examples/tests if needed"
  ],
  "passes": false
}
```

## Long-Term Memory (for automation agents)

If an automated agent is used to implement or track this documentation change, it SHOULD add a memory entry titled `# Documentation: <short-title>` and include the JSON spec above wrapped in a fenced code block.

Agents: use the repository's GitHub MCP for all GitHub operations (create/update issues, open PRs) and the provided memory tools to persist the memory entry. Do NOT use the `gh` CLI.

## Agent actions (recommended checklist)

- [ ] Create or update memory entry under `/memories/` with the JSON spec.
- [ ] Open a branch `chore/docs/<slug>` off `v1.0.0` and open a PR referencing this issue.
