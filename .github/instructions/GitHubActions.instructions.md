---
description: 'Agent directives for GitHub Actions workflows.'
applyTo: '**/*'
---

# Agent Directives — GitHub Actions

Purpose: These directives are written for an automated coding agent that creates, edits, or validates GitHub Actions workflow files in this repository. They document repository-specific expectations (permissions, action pinning, secrets handling) and complement the project's tooling and security posture. Follow project tooling and security guidance first; these directives cover repository-specific decisions.

- **Workflow metadata**: Every workflow must include `name:` and a concise description.
- **Least privilege**: Set explicit `permissions:` for workflows and avoid using broad write scopes or `GITHUB_TOKEN` elevated privileges unless required and documented.
- **Pin actions**: Reference actions by tag or commit SHA (prefer major version tags with minor/patch pinning where reasonable). Avoid floating `@latest`.
- **Secrets**: Do not hard-code secrets in workflows. Use `secrets.` and GitHub repository/org secrets only. Document required secrets in the PR (but never their values).
- **Concurrency & idempotency**: For deploy or long-running jobs, use `concurrency` to prevent overlapping runs when necessary.
- **Minimal triggers**: Use explicit `on:` triggers (push/PR filters) to reduce noise. Avoid broad `on: [push]` without path filters when the workflow targets specific files.
- **Action versions & maintainers**: Prefer well-maintained, popular actions. If adding a new action, include a short rationale in the PR.
- **Check secrets usage in logs**: Ensure workflows avoid printing secrets; use `::add-mask::` or rely on GitHub's built-in masking.

## Exception Handling & Human Review

- **Include a note in the summary for Human Review**: If changes would violate these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
