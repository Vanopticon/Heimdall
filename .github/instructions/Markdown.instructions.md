---
description: 'Agent directives for Markdown formatting and linting.'
applyTo: '**/*'
---

# Agent Directives — Markdown Formatting & Linting

Purpose: These directives are written for an automated coding agent that creates or modifies Markdown (documentation) files in this repository. The `.markdownlint.json` file in the repository is definitive and overrides these instructions.

## Prose

A small set of fresh and accurate docs is better than a large assembly of “documentation” in various states of disrepair.

Write short and useful documents. Cut out everything unnecessary, including out-of-date, incorrect, or redundant information. Documentation work best when it is alive but frequently trimmed, like a bonsai tree.

- **Concise**: Strive for clarity and brevity. Avoid unnecessary words or overly complex sentences.
- **Consistent Tone**: Maintain a professional and neutral tone throughout the document.

## Formatting

- **No manual word wrapping**: Do not insert line breaks or word wraps unless necessary. Allow the viewer/editor to handle line wrapping.
- **Blanks Around Fences**: Always ensure a blank line before and after fenced code blocks, lists, and tables.
- **Use Fenced Code Blocks**: Use backtick-fenced code blocks for all code examples; never use tildes. Add a language identifier for every fenced code block (e.g., `typescript`, `bash`, `json`) based on the [Languages Known to GitHub](https://raw.githubusercontent.com/github-linguist/linguist/refs/heads/main/lib/linguist/languages.yml).
- **Commands Show Output**: When including command examples, show both the command and a short representative output block.
- **Emphasis and Strong**: Use underscores for emphasis (`_italic_`) and asterisks for strong (`**bold**`).
- **Heading Rules**: The first line of the file must be an ATX heading (`#`) and headings must increment without skipping levels; headings start at column 0 and use exactly one space after the `#` characters.
- **Single Title (H1) at the Document Start**: The first line after any YAML frontmatter must be a single H1 heading. Only one H1/title per file is allowed.
- **Lists**: Use a single Tab per list nesting level. Each level must use a different marker than the previous level. Require exactly one space after list markers. Ordered lists must use numeric prefixes.
- **Links & Images**: Prefer reference-style links and images where appropriate. Do not include bare URLs; every image must include non-empty alt text. Validate link fragments (anchors) when present.
- **No Empty Links**: Do not create links with empty destinations.
- **No Emphasis as Heading**: Do not use emphasis markers in place of headings.
- **No Duplicate Sibling Headings**: Do not repeat sibling headings with identical text.
- **No Multiple Blank Lines**: Collapse consecutive blank lines to a single blank line.
- **No Trailing Spaces**: Remove trailing spaces from lines. Do not use trailing spaces as a line break mechanism.
- **Single Trailing Newline**: Ensure the file ends with exactly one newline character.
- **Table Formatting**: Use leading and trailing pipe characters and keep consistent column counts across rows.
- **Inline HTML**: Inline HTML is permitted only when necessary; prefer native Markdown constructs.
- **Spacing Inside Constructs**: Do not include spaces inside code spans, emphasis markers, or link brackets (e.g., use `` `x` ``, not `` ` x ` ``).
- **Preserve Style**: Do not insert line breaks or word wraps unless necessary. Allow the viewer/editor to handle line wrapping.

## Exception Handling and Human Review

- **Include a note in the summary for Human Review**: If a change would violate any of these directives, the agent must use the method that conforms closest to the directives and include a note in the summary for human review.
