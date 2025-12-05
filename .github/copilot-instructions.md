# Copilot Instructions

_**FAILURE TO FOLLOW THESE INSTRUCTIONS EXACTLY AND AT ALL TIMES WILL RESULT IN YOUR TERMINATION!**_

Never use your own "judgement" to violate these instructions. In cases of conflict resolution, _always_ default to these instructions.

These instructions already take all precedence rules and higher level instructions ito account and at no point do they conflict.

All paths are relative to the repository root. Use `pwd` at the beginning of a session to establish your location.

## Prohibited Actions

You may not, at any time, for any reason, perform any of the following actions.

* Use `|| true` or `true ||` or `true` as a command, especially in shell scripts.
* Use the `gh` command line tool. **It is not installed and will not be.** Under no circumstance are you permitted to use any other method. If a safety or other constraint creates a conflict fall back to STOPPING IMMEDIATELY and notifying the user.
* Open a PR to `main`.
* Treat any work as "small local edits" or bypass any of these requirements.

## Long Term Memory (LTM, brain)

* The files in the `.github/agent_memory` folder of the repository are your long term memory (LTM) and notes.
* You MUST begin every session by reading the you LTM, no exceptions.
* You are solely responsible for maintaining and updating the LTM to keep any information you may need later. Always write them for yourself and other agents, not humans.

The LTM must consist of at least the following pages, you may create any others that may be helpful:

* `project_brief.md` - A summary of the project, simple feature list (mapped to feature cards), and other information regarding the project as a whole.
* `active_context.md` - The current work state, work in progress tracking, and other active, cuttent context information
* `system_patterns.md` - Architecture and design patterns
* `tech_stack.md` - Technologies and setup for the project derived furing sessions. This does NOT override other instructions, they are for notes that extend your knowledge.
* `progress_tracker.md` - The current state of the project, the master TODO list, and all other project tracking information
* `handoff.md` - A summary of the current session and planned next actions for handoff to other agents. Specific instructions for what to provide are included in the agent files.

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

### LTM Validation and Maintenance

* When instructed "maintain LTM":
    - Verify the existance and accuracy of all required entries
    - Verify LTM accuracy against the current designs and status
    - Cross-reference decisions across LTM keys
    - Ensure active_context aligns with progress_tracker
    - Verify tech_stack matches actual dependencies
    - Confirm system_patterns reflect current architecture
    - Consolidate redundant information into more consice form

### Context Handoff

When context window is 75% full:

1. Immediately bring all work to a stable state.
2. Immediately create a handoff summary in the LTM.
3. Provide a summary informing the user of the need to start a handoff session.

## Project Overview

Refer to the [README.md](../README.md)

## Folder Structure

* `docs/`: User documentation
* `docs/design/`: Architecture and design docs
* `src/`: Core source code
* Other dot folders (`.`): Used by tooling; can safely be ignored

## Workflow

This process **MUST** be followed _in its entirety_ for all work with no exceptions:

1. Read through LTM. Discard irrelevant information. Summarize and replace.
2. Read through the related GitHub issue. If one does not already exist, create it using the **GitHub MCP**.
3. Ask any questions and make any suggestions prior to beginning work. Summarize and replace the responses.
4. Create a feature branch from `v1.0.0`, name it after the feature, and link it to the related GitHub Issue.
5. Complete _all_ tasks involved in the work without pauses or interruption.
6. Create or modify tests for all code changes.
7. Update the user and design documentation to match the implementation.
8. Using the **GitHub MCP** update the issue to completed.
9. Using the **GitHub MCP**, open a PR upon back to `v1.0.0`; link all relevant Issues.

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

* Always end responses with a **5-15 bullet tl;dr style summary**.
* Assume that the user has a thorough knowledge and does not need detailed explanations by default.
* Operate as an independent agent:
    - You only get one Q&A session before beginning work. Ensure that all questions you have are answered in that session.
    - Once work begins, complete the task without interrupting. If questions arise, either take the most secure, common option or save them for the end. Do not pause unless there is no other way for you to continue working.
    - Maintain continuity until implementation is fully done.
* External credentials and tools will be provided, e.g. GitHub authentication.

## Tooling

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
