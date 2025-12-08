---
name: "Bug report"
about: "Report reproducible bugs: include steps, logs, and environment details."
title: "Bug: "
labels: "bug, needs-triage"
assignees: ""
---

## Bug

Provide a concise one-line summary of the bug.

## Steps to reproduce (required)

1.
2.

## Expected behavior

Describe the behavior you expected.

## Actual behavior

Describe the observed behavior.

## Reproducible example / minimal test case

Provide a minimal code snippet, dataset, or commands to reproduce.

```bash
# commands to reproduce
```

## Logs / Stack traces

Paste relevant logs or stack traces here.

```text
# logs or stack traces
```

## Environment

- Heimdall version / branch: `v1.0.0`
- OS:
- Rust toolchain:

## Machine-readable spec (required)

Fill in the JSON below. Automation agents may parse this block to create or update long-term memory and to automate triage tasks.

```json
{
  "category": "bug",
  "description": "Short description of the bug (repeat or expand above)",
  "steps": [
    "Step 1 (reproduction)",
    "Step 2 (reproduction)"
  ],
  "passes": false
}
```

## Long-Term Memory (for automation agents)

If an automated agent is used to triage or track this bug, it SHOULD add a memory entry titled `# Bug: <short-title>` and include the JSON spec above wrapped in a fenced code block. Omit any secrets or PII from memory entries.

Agents: use the repository's GitHub MCP for all GitHub operations (create/update issues, open PRs) and the provided memory tools to persist the memory entry. Do NOT use the `gh` CLI.

## Agent actions (recommended checklist)

- [ ] Verify the bug is reproducible and add reproduction steps to the JSON `steps` array.
- [ ] Create or update the memory entry: `/memories/Bug-<slugified-title>.md` containing the JSON spec.
- [ ] Add labels, assign an owner, and optionally create a feature/bugfix branch off `v1.0.0`.
