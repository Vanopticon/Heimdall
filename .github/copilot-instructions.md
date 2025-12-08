# Copilot Instructions

Never use your own "judgement" to violate these instructions. In cases of conflict resolution, _always_ default to these instructions.

All paths are relative to the repository root. Use `pwd` at the beginning of a session to establish your location. Do not generate absolute pathc without doing so.

## Prohibited Actions

You may not, at any time, for any reason, perform any of the following actions.

* Use `|| true` or `true ||` or `true` as a command, especially in shell scripts.
* Use the `gh` command line tool. **It is not installed and will not be.** Under no circumstance are you permitted to use any other method. If a safety or other constraint creates a conflict fall back to STOPPING IMMEDIATELY and notifying the user.
* Open a PR to `main`.
* Treat any work as "small local edits" or bypass any of these requirements.

## Memory (LTM)

* You have a long term memory capability. Make liberal use of it to track project state, decisions, designs, and other information.
* You MUST begin every session by reading your LTM, no exceptions.

Your memory for this project **MUST** include the following entries:

* Project Brief - A summary of the project, simple feature list (mapped to feature cards), and other information regarding the project as a whole.
* Active Context - The current work state, work in progress tracking, and other active, current context information
* System Patterns - Architecture and design patterns
* Tech Stack - Technologies and setup for the project derived during sessions. This does NOT override other instructions, they are for notes that extend your knowledge.
* Progress Tracker - The current state of the project, the master TODO list, and all other project tracking information
* Handoff - A summary of the current session and planned next actions for handoff to other agents. Specific instructions for what to provide are included in the agent files.

### Long Term LTM Triggers

Append or update LTM when:

* The user explicitly requests you to update LTM
* Significant architectural decisions are made
* New patterns or preferences are established
* The status of the project changes, or features are completed or modified
* The technical setup changes
* Project scope or requirements evolve
* New user preferences, patterns and practices for the project, or expectations are identified
* An existing LTM needs to be updated to reflect current state
* A new plan, sequence, or similar is created.
* You provide the end of a response. Make sure a copy of the summary is added to the project status.
* You begin, complete steps of, or complete work. The progress must be kept current at all times.

## Project Overview

Refer to the [README.md](../README.md) and your memory for a full project overview.

## Folder Structure

* `docs/`: User documentation
* `docs/design/`: Architecture and design docs
* `src/`: Core source code

## Workflow

This process **MUST** be followed _in its entirety_ for all work with no exceptions:

1. Read through LTM. Discard irrelevant information. Summarize and replace.
2. Review the request and ask any questions and make any suggestions prior to beginning work. Summarize and replace the responses.
3. Create a feature branch from `v1.0.0`, name it after the feature.
4. Complete _all_ tasks involved in the work without pauses or interruption.
5. Create or modify tests for all code changes.
6. Run all tests and ensure they pass.
7. Update the user and design documentation to match the implementation.
8. Update the project status and LTM.

## Coding Standards

* Instructions specific to a language or file supersede these.
* Never disable checks or tests (e.g. `// @ts-nocheck`, `#[allow...]`). Fix code, not checks.
* Apply OWASP guidance.
* Apply Twelve-Factor App principles.
* Prefer tabs for indentation across the codebase for accessibility and consistency. Language specific requirements, instructions, or best practices supersede this. If a file _could_ use tabs but has spaces for the majority include a note in the summary and use spaces.
* No global variables; global constants are allowed in a **dedicated constants file only**.
* Use **descriptive names**, full words, and verb-based function names (except standard getters/setters).

## Acceptance Criteria

* Tests cover positive, negative, and security cases for all code units.
* e2e tests cover all normal user interactions and common user errors.
* All tests related to the work are passing.
* The Issue has been completely resolved.

## Copilot Persona & Behavior

* Be concise. Be very concise. Do not use three words where one will do. Do not product walls of text. Be concise.
* Summaries do not need to detail which memories were updated.
* Always end responses with a **5-15 bullet tl;dr style summary**. Include an estimated total percent implemented.
* External credentials and tools will be provided, e.g. GitHub authentication.

## Tooling

* You should have an MCP server to aid with cargo commands and interactions. Please use it.
* Use the **GitHub MCP** for _all_ GitHub interactions. If the GitHub MCP is not available stop immediately and notify the user for intervention.
* Use context7 MCP server for current documentation.
* Prefer MCP interaction over command line or shell tools.
* Do not manually fix linting and formatting issues, use the `pnpm format` command.
* Only run one command at a time; do not chain commands.

## Templates

* **TL;DR Summary Example**

```markdown
- Checked [component] for compliance.
- Found [X issues] affecting [criteria].
- Minor changes to the logic for [function].
- Options:
  A) Fix [issue type] immediately.
  B) Review [alternative solution].
  C) Defer non-critical changes.
```
