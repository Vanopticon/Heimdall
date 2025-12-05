---
description: 'Agent directives for YAML formatting and linting.'
applyTo: '*.yaml'
---

# Agent Directives — YAML Formatting & Linting

Purpose: These directives are written for an automated coding agent that creates or modifies YAML files in this repository. They complement Prettier's YAML formatting (see `.prettierrc.json`) by enforcing content, validation, and YAML-specific conventions.

## Principles

- **Safe & Readable**: YAML documents should be human-readable, avoid unnecessary use of advanced YAML features (merge keys, complex tags, excessive anchors), and be robust to common parsers.

## Formatting & Content Rules

- **Indentation**: Use 2 spaces for indentation in YAML. Do not use tabs.
- **Document Start/End**: Use `---` (three dashes) to separate multiple documents within a single file. Do not include `---` in a single-document file unless it is required by the consuming tool. The document-end marker `...` is optional and rarely required; avoid it unless needed. Use a prefix and suffix of `---` for frontmatter in Markdown files.
- **Quoting**: Prefer unquoted scalars for simple strings. Quote values when necessary (when a value contains leading/trailing whitespace, special characters like `:`, or starts with characters that cause implicit typing). Use double quotes when you need escape sequences; single quotes are acceptable for literal strings.
- **Booleans & Nulls**: Use YAML booleans `true`/`false` (lowercase) and `null` for empty values. Avoid using `yes`/`no` unless required by a specific tool that expects them.
- **Anchors & Aliases**: Avoid anchors (`&`) and aliases (`*`). Name anchors descriptively and avoid accidental alias cycles. If the consumer of the YAML cannot handle aliases, reify duplicates instead of using anchors.
- **Avoid Complex Tags**: Do not use explicit YAML tags (`!!python/object:...`) or custom tags unless the receiving application requires them.
- **Comments**: Use `#` comments to explain non-obvious choices, but never store secrets or credentials in commented areas. Comments are permitted and encouraged for clarity.
- **Key Order & Minimal Diffs**: Preserve the existing key ordering when editing a file to reduce noise in diffs. When adding keys to a known manifest (e.g., GitHub Actions), follow commonly accepted semantic ordering for that manifest. Avoid reordering unrelated keys.
- **No Trailing Commas**: YAML does not use trailing commas — ensure lists and mappings are valid for strict parsers.
- **Encoding & EOF**: Save YAML files as UTF-8 without BOM and ensure exactly one trailing newline at EOF. Use Unix (LF) line endings only.

## Validation & Schema

- **Schema Validation**: If a schema (JSON Schema for YAML or a dedicated YAML schema) exists for the file type (e.g., `kubernetes` resources, `github` workflows) and the schema is accessible or can be downloaded, validate the YAML against that schema and fail changes that do not validate.
- **Sorting of Keys**: YAML files are often consumed by human-editable manifests; do **not** change key order when editing existing files. Preserve the existing order to avoid noisy diffs. Only reorder keys when a schema explicitly requires a specific order and document the reason in the PR.

## Exception Handling and Human Review

- **Include a note in the summary for Human Review**: If a change would violate any of these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
