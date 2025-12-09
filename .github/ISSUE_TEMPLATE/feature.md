---
name: "Feature — Implementation & Memory Entry"
about: "Structured feature issue template for humans and automation agents. Includes a machine-readable JSON spec and instructions for updating long-term memory."
title: "Feature - "
labels: "feature, needs-triage"
assignees: ""
---

## Feature

## Short description

Provide a concise one-line summary of the feature (what it delivers and who benefits).

## Category

Select one of: `functional`, `non-functional`, `technical-debt`.

## Implementation steps (human-readable)

- [ ] Define the feature in the project design docs (`docs/design/features/`).
- [ ] Add or update tests (unit, integration, e2e) as required.
- [ ] Implement code changes in a feature branch off `v1.0.0` and open a PR targeting `v1.0.0`.
- [ ] Link PR(s) to this issue and add notes for human review.

## Acceptance criteria

Describe how reviewers and CI will consider this feature complete (concise bullet points). Examples:

- Idempotent behavior documented and tested
- PII controls enforced by tests
- Integration tests passing against ephemeral Postgres+AGE

## Machine-readable feature spec (required)

Fill in the JSON below. Agents may parse this block to create or update long-term memory and to automate follow-up tasks.

```json
{
  "category": "functional",
  "description": "Short description of the feature (repeat or expand above)",
  "steps": [
    "Step 1 (high-level)",
    "Step 2 (high-level)"
  ],
  "passes": false
}
```

## Long-Term Memory (for automation agents)

If an automated agent is used to implement or track this feature, it MUST add a memory entry titled `# Feature: <short-title>` and include the JSON feature spec above wrapped in a fenced code block (as shown). Example memory entry:

Example memory entry (create a memory file titled `# Feature: Example Feature Title` and include the JSON below):

```json
{ /* the JSON spec above */ }
```

Agents: use the repository's GitHub MCP for all GitHub operations (create/update issues, open PRs) and the provided memory tools to persist the feature entry. Do NOT use `gh` or other CLI tools that are not permitted by repository policy.

## Agent actions (recommended checklist)

- [ ] Verify there is no existing memory entry for this feature; if none, create it.
- [ ] Create or update this issue with any extracted metadata (labels, milestone).
- [ ] Create a feature branch: `feature/<slugified-title>` off `v1.0.0`.
- [ ] Open a PR targeting `v1.0.0` and add this issue number to the PR description.

## Notes & references

- Design docs: `docs/design/Implementation-Roadmap.md`
- Feature directory: `docs/design/features/`
- Memory guideline: `.github/instructions/Feature.instructions.md`

---

_End of template._
