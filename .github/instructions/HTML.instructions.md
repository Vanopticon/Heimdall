---
description: 'Agent directives for HTML templates and static pages.'
applyTo: '*.html'
---

# Agent Directives — HTML

Purpose: These directives are written for an automated coding agent that creates, edits, or validates HTML templates and static pages in this repository. They document repository-specific requirements (head/meta needs, accessibility, and formatting expectations) and complement the project's tooling (Prettier). Follow project tooling first; these directives cover repository-specific decisions.

- **Document language**: Ensure the top-level `<html>` element includes a `lang` attribute. The default language is `en` unless otherwise specified.
- **Charset & viewport**: Include `<meta charset="utf-8">` and a responsive viewport meta tag in the document head.
- **Avoid `rel=preload`/`preconnect`**: Do not use preloading constructs as they are not well supported across current browsers.
- **No inline secrets or credentials**: Never embed secrets, API keys, or credentials directly in HTML.
- **Avoid inline scripts/styles when possible**: Prefer external files so Prettier can manage formatting; small inline critical CSS is acceptable when necessary and should be documented.
- **Accessibility**: Provide descriptive `alt` attributes for images, use semantic HTML elements, and ensure interactive elements are keyboard-accessible. Use ARIA attributes where necessary.

## Exception Handling & Human Review

- **Include a note in the summary for Human Review**: If changes would violate these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
