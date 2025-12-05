---
description: 'Agent directives for JavaScript files (Node/ESM).'
applyTo: '*.js'
---

# Agent Directives — JavaScript (Node / ESM)

Purpose: These directives are written for an automated coding agent that creates, edits, or validates JavaScript files in this repository. They describe repository-specific requirements (Node ESM usage, runtime safety) and complement the project's tooling (Prettier). Follow project tooling first; these directives document repository-specific decisions.

Only include rules that differ from defaults or are repository-specific.

- **ESM only**: The repository uses ESM (`"type": "module"` in `package.json`). Use `import`/`export` and avoid `require()` or `module.exports`.
- **Prefer TypeScript for new code**: New server-side code should be authored in TypeScript where feasible; add `.js` only for tiny scripts or tooling wrappers that do not require type safety.
- **No global mutable state**: Avoid module-level mutable singletons. Use explicit initialization functions (follow the `initializeDataSource()` pattern for DB initialization).
- **Error handling**: Use explicit `try/catch` at boundaries and avoid swallowing errors silently. For HTTP handlers follow existing patterns with `Response` objects or appropriate framework error handling.
- **Dependencies & imports**: Import only what you need. Use project aliases (e.g., `$lib/...`) where configured.
- **No `eval` or dynamic code**: Do not use `eval()`, `new Function()`, or unsafe dynamic require patterns.
- **Formatting & linting**: Run `pnpm format` and ensure `pnpm lint` (prettier check) passes.

## Exception Handling & Human Review

- **Include a note in the summary for Human Review**: If changes would violate these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
