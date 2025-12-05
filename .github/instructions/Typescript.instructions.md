---
description: 'Agent directives for Typescript formatting and linting.'
applyTo: '*.ts'
---

# Agent Directives — Typescript Formatting & Linting

Purpose: These directives are written for an automated coding agent that creates, edits, or refactors TypeScript code in this repository. The `.prettierrc.json` and `tsconfig.json` files in the repository are definitive and override these instructions.

## Prose

Use Markdown instructions for any in-code documentation or comments. Write clear, concise comments that explain the "why" behind complex logic. Avoid redundant comments that restate obvious code behavior.

## Formatting

- **Stay Strict**: Never disable linter/TypeScript checks.
- **Prefer Explicit Types**: Prefer explicit function return types and typed variables for exported/public APIs. Use `unknown` rather than `any` when accepting untyped input and narrow before use.
- **Avoid `any`**: Do not use the _any_ type; prefer `undefined`, `unknown`, or specific types. If `any` is necessary, document the justification in the code comments and PR description.
- **Keep `as` Casts Minimal**: Use type assertions sparingly; prefer safe narrowing and typed helpers.
- **No Global Side-Effects**: Do not add mutable global state. Keep module initialization idempotent; use exported functions for initialization (the repo uses `initializeDataSource()` pattern for DB setup).
- **SvelteKit Patterns**: Conform to SvelteKit 5 patterns. Use #tools:io.github.upstash/context7 for documentation integration.
- **Error Handling**: Use Error objects for exceptional conditions and handle errors at appropriate boundaries. Avoid returning raw error-like values; ensure production code does not allow unhandled exceptions to crash the process.
- **Tabs for Indentation**: The repository uses tabs for indentation. Keep indentation consistent with existing file.
- **Line Endings & EOF**: Use only Unix (LF) line endings. Ensure a single trailing newline at EOF and no trailing spaces.
- **Export Style**: Follow existing export patterns. Use named exports for helpers and prefer default exports.
- **Promise Handling**: Use `async`/`await` consistently; catch and handle errors at boundaries. Avoid unhandled Promise rejections.

## Exception Handling & Human Review

- **Include a note in the summary for Human Review**: If changes would violate these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
