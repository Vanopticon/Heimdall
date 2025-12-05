---
description: 'Agent directives for CSS and Tailwind usage.'
applyTo: '*.css'
---

# Agent Directives — CSS / Tailwind

- **Use Tailwind where possible**: Prefer Tailwind utility classes for layout and simple styling. Add bespoke component CSS in `app.css` or component-scoped styles only when utilities are insufficient.
- **Design tokens**: Use CSS variables or Tailwind config tokens for colors, spacing, and typographic scales. Only add new tokens if necessary. Add new tokens to `tailwind.config` rather than scatter magic values in CSS.
- **Tabs for indentation**: Repository-wide convention uses tabs — keep tabs for indentation in CSS files to match the codebase.
- **Scoped vs global**: Keep component-specific CSS scoped to the component; avoid adding global selectors unless modifying theme or base styles.
- **No !important**: Avoid `!important` except when overriding external libraries; document any use in the PR.
- **Minimize specificity**: Prefer low-specificity selectors and class-based styling to keep overrides predictable.

## Exception Handling & Human Review

- **Include a note in the summary for Human Review**: If changes would violate these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
