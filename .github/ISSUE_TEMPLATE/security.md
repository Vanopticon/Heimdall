---
name: "Security issue (private)"
about: "Report a security vulnerability — do NOT include secrets publicly."
title: "Security: "
labels: "security, needs-triage"
assignees: ""
---

## Security issue — DO NOT INCLUDE SECRETS

Provide a concise one-line summary of the suspected vulnerability.

## Affected components

List components, modules, or endpoints that may be affected.

## Impact

Describe the potential impact and severity.

## Steps to reproduce (if safe)

If it is safe to share a reproducible demo, provide steps. Otherwise, indicate that details will be shared privately.

## Proof-of-concept / sensitive attachments

Do NOT paste keys, credentials, or PII. Use GitHub Security Advisories or contact repository owners privately for sensitive PoC attachments.

## Contact / private disclosure

Prefer: GitHub Security Advisories or private contact with repository maintainers.

## Machine-readable metadata

```json
{
  "category": "security",
  "description": "Short description of the security issue (non-sensitive summary)",
  "steps": [
    "Optional safe reproduction step"
  ],
  "private": true,
  "passes": false
}
```

## Long-Term Memory (for automation agents)

Security reports may contain sensitive details and should avoid storing secrets in long-term memory. Agents SHOULD add a memory entry titled `# Security: <short-title>` containing a minimal, non-sensitive JSON summary of the report (as above) and a reference to secure disclosure channels (e.g., Security Advisory ID or private contact). Do NOT include PoC secrets or private keys in memory.

Agents: use the repository's GitHub MCP for all GitHub operations and the memory tools to persist only non-sensitive metadata. Use private disclosure channels for sensitive attachments; do NOT use the `gh` CLI.

## Agent actions (recommended checklist)

- [ ] Ensure no secrets are written to the issue or memory entry.
- [ ] Create a minimal memory entry under `/memories/` containing only the JSON summary.
- [ ] Notify repository owners via the approved private channel (Security Advisory or email) for sensitive PoCs.
