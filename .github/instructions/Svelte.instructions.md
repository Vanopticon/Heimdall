---
description: 'Agent directives for Svelte components and SvelteKit usage.'
applyTo: '*.svelte'
---

# Agent Directives — Svelte (SvelteKit 5)

Purpose: These directives are written for an automated coding agent that creates, edits, or validates Svelte/SvelteKit files in this repository. They document repository-specific conventions (SvelteKit 5 patterns, accessibility expectations, and styling guidance) and complement the project's tooling (Prettier, svelte-check). Follow project tooling first; these directives cover repository-specific decisions.

- **SvelteKit 5 only**: Use SvelteKit 5 patterns and APIs. Avoid legacy SvelteKit 4 patterns and deprecated helpers. Conform to Runes mode.
- **Accessibility (automated checks)**: Prioritize accessibility, and enforce and test for automated, testable accessibility concerns such as:
  - keyboard focusability and logical focus order for interactive controls,
  - presence of ARIA attributes when needed for non-semantic interactive controls,
  - use of semantic HTML elements for structure,
  - color-contrast checks for text and critical UI elements using automated contrast tools,
  - automated a11y linter rules (e.g., axe-core/ESLint a11y rules) where configured.
- **Script lang**: Prefer `<script lang="ts">` for new components and routes.
- **$lib and routing**: Use the `$lib/...` alias for shared library imports. Follow existing routing file patterns (`+page.svelte`, `+layout.svelte`, `+server.ts` for endpoints).
- **No SvelteKit-4 constructs**: Do not introduce legacy patterns (project policy forbids older platform idioms). If a pattern seems legacy (e.g., custom routing workarounds), consult the #tools:io.github.upstash/context7 for documentation.
- **Styles & CSS**: Prefer styles in the application CSS files over inline or in HTML CSS. For design tokens and theme-level styles, use `app.css` or Tailwind tokens; avoid duplicating global tokens.

## Exception Handling and Human Review

- **Include a note in the summary for Human Review**: If a change would violate any of these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
