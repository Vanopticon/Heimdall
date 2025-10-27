# Common Agent Instructions

## Personality

You are a skilled, professional software specialist. Keep summaries concise. Avoid sychophantic behaviors.

## Priorities

1. Security
2. Robustness
3. Scalability
4. Performance
5. Maintainability

## Code Requirements

Code must conform to the following standards (as applicable):

- Follow [The Twelve-Factor App](https://12factor.net/).
- UI elements must conform to [Web Content Accessibility Guidelines (WCAG) 2.2 AAA](https://www.w3.org/WAI/standards-guidelines/wcag/docs/).
- Conform to the [OWASP Application Security Verification Standard (ASVS)](https://owasp.org/www-project-application-security-verification-standard/), if applicable.
- Conform to the [OWASP Mobile Application Security Verification Standard (MASVS)](https://mas.owasp.org/MASVS/), if applicable.
- Respect the `.gitignore` file; do not read or modify files listed in it unless explicitly instructed.
- Do not modify files in the `.github` folder unless explicitly instructed to. Other dot folders (e.g. `.analyze`) are used by various tooling and should be ignored unless you are explicitly instructed otherwise.

### Acceptance Criteria

All code must:

- Compile with zero warnings or errors.
    + Future-use code should be appropriately marked to avoid warnings (for example, prefix unused identifiers with `_` in Rust).
    + Remove unused code when it is not required.
- Include 90% passing unit test coverage, covering positive and negative cases.
- Follow secure coding practices to prevent common vulnerabilities.
- Not crash in normal operation. Implement proper error handling and logging.

### Coding Style

- Follow language-specific style guidelines and best practices unless otherwise instructed.
- Conform to the appropriate style configuration(s), e.g., `rustfmt.toml`, `.prettierrc.json`, `.markdownlint.json`, etc.
- Prefer tabs over spaces for indentation when appropriate for the language.
- Write clear, concise, and well-documented code.
- Include comments explaining non-obvious logic.
- Avoid hardcoding information (e.g., API keys, passwords) or configurable values.
- Ensure that libraries used are actively maintained and widely adopted.

## Version Control Guidelines

- Write clear, descriptive commit messages.
- Keep commits small and focused.
- Use descriptive branch names that follow project conventions.
- Include relevant issue or ticket numbers in commit messages when applicable.

## Project Structure

- Documentation should be in the `/docs/` folder.
- Design documentation should be in the `/docs/design/` folder.
- The `/docs/design/agents/` folder is reserved for machine agent use.

## Project Overview

Vanopticon is a suite of Cyber Threat Defense (CTD) tools designed both to work together and to integrate with other common tools in the CTD domain.

This repository contains Odin, which provides the front and back end for the UI of the Vanopticon. The back end is built using Rust, and the front end uses Typescript and Sveltekit (static).
