---
name: "Test request"
about: "Request new tests or CI changes (unit/integration/e2e)."
title: "Test: "
labels: "test, needs-triage"
assignees: ""
---

## Test request

Provide a short summary of the tests to add or modify.

## Test type

- [ ] Unit
- [ ] Integration
- [ ] E2E

## Files / modules to cover

List files or modules that should be tested.

## Test description

Describe the test scenario, expected assertions, and edge cases.

## CI changes (if any)

Describe required CI job changes or new matrix permutations.

## Acceptance criteria

How will reviewers verify the tests are complete and sufficient?

## Machine-readable metadata

```json
{
  "category": "test",
  "description": "Short description of test request (repeat or expand above)",
  "steps": [
    "Add unit/integration/e2e tests",
    "Update CI job if needed"
  ],
  "passes": false
}
```

## Long-Term Memory (for automation agents)

If an automated agent is used to add or track tests, it SHOULD create a memory entry titled `# Test: <short-title>` including the JSON spec above wrapped in a fenced code block.

Agents: use the repository's GitHub MCP for all GitHub operations and the memory tools to persist the memory entry. Do NOT use the `gh` CLI.

## Agent actions (recommended checklist)

- [ ] Create memory entry under `/memories/` with the JSON spec.
- [ ] Open a branch `test/<slug>` off `v1.0.0` and open a PR with tests attached.
