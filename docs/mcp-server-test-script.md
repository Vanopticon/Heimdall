# MCP Server Test Script

This document provides a comprehensive testing script for verifying all available MCP servers.

## Purpose

This script is designed to systematically test all Model Context Protocol (MCP) servers available in the GitHub Copilot environment to ensure they are properly configured and functional.

## Prerequisites

- Access to a GitHub Copilot Coding Agent environment
- Repository cloned and accessible
- Sufficient permissions to execute commands and create files

## Test Execution Steps

### 1. GitHub MCP Server Tests

#### 1.1 Repository Operations

```
Test: list_branches
Parameters: owner, repo
Expected: List of branches with names and SHAs
Success Criteria: Returns array of branch objects

Test: list_tags
Parameters: owner, repo
Expected: List of tags (may be empty)
Success Criteria: Returns array (empty or populated)

Test: list_commits
Parameters: owner, repo, perPage
Expected: List of recent commits
Success Criteria: Returns array of commit objects
```

#### 1.2 File Operations

```
Test: get_file_contents
Parameters: owner, repo, path
Expected: File content from repository
Success Criteria: Returns file content or error if not found
```

#### 1.3 Issue and PR Operations

```
Test: list_issues
Parameters: owner, repo
Expected: List of issues
Success Criteria: Returns issues array

Test: list_pull_requests
Parameters: owner, repo, state
Expected: List of pull requests
Success Criteria: Returns PR array
```

#### 1.4 Workflow Operations

```
Test: list_workflows
Parameters: owner, repo
Expected: List of GitHub Actions workflows
Success Criteria: Returns workflows array with workflow details
```

#### 1.5 Release Operations

```
Test: list_releases
Parameters: owner, repo
Expected: List of releases
Success Criteria: Returns releases array (may be empty)
```

#### 1.6 Search Operations

```
Test: search_repositories
Parameters: query, perPage
Expected: Search results matching query
Success Criteria: Returns search results object

Test: search_code
Parameters: query, perPage
Expected: Code search results
Success Criteria: Returns code search results object
```

### 2. Playwright Browser MCP Server Tests

**Note:** Browser operations may be restricted in secure environments.

#### 2.1 Navigation

```
Test: browser_navigate
Parameters: url
Expected: Navigate to URL
Success Criteria: Page loads or returns security error (expected in restricted env)
```

#### 2.2 Page Interaction

```
Test: browser_snapshot
Expected: Accessibility snapshot of current page
Success Criteria: Returns page structure (requires navigation)

Test: browser_take_screenshot
Expected: Screenshot of current page
Success Criteria: Returns screenshot image (requires navigation)
```

### 3. Bash/Shell MCP Server Tests

#### 3.1 Synchronous Execution

```
Test: bash (sync mode)
Parameters: command, mode: "sync", initial_wait
Expected: Command executes and returns output
Success Criteria: Command completes with exit code 0

Example: echo "test" && date && whoami
```

#### 3.2 Asynchronous Execution

```
Test: bash (async mode)
Parameters: command, mode: "async", sessionId
Expected: Command starts in background, returns sessionId
Success Criteria: Returns sessionId for async session

Example: for i in 1 2 3; do echo "Count: $i"; sleep 1; done
```

#### 3.3 Session Management

```
Test: read_bash
Parameters: sessionId, delay
Expected: Retrieves output from async command
Success Criteria: Returns command output

Test: list_bash
Expected: Lists all active bash sessions
Success Criteria: Returns list of sessions with status
```

### 4. File System MCP Server Tests

#### 4.1 Read Operations

```
Test: view (file)
Parameters: path
Expected: File content with line numbers
Success Criteria: Returns file content

Test: view (directory)
Parameters: path (ending with /)
Expected: Directory listing
Success Criteria: Returns list of files and subdirectories
```

#### 4.2 Write Operations

```
Test: create
Parameters: path, file_text
Expected: New file created with content
Success Criteria: File created successfully

Test: edit
Parameters: path, old_str, new_str
Expected: File content updated
Success Criteria: File edited successfully
```

### 5. GitHub Advisory Database MCP Server Tests

```
Test: gh-advisory-database
Parameters: dependencies (array of {ecosystem, name, version})
Expected: List of vulnerabilities for given dependencies
Success Criteria: Returns vulnerability report

Example: Check npm package "express" version "4.17.0"
```

### 6. Code Review MCP Server Tests

```
Test: code_review
Parameters: prTitle, prDescription
Expected: Automated code review of current changes
Success Criteria: Returns review comments and suggestions

Note: Should be run after making code changes
```

### 7. CodeQL Checker MCP Server Tests

```
Test: codeql_checker
Expected: Security scan of code changes
Success Criteria: Returns security analysis results

Note: Should be run after making code changes
```

## Test Results Template

For each test, document:

```
Server: [MCP Server Name]
Test: [Test Name]
Tool: [Tool Function Name]
Status: [PASS/FAIL/BLOCKED/SKIP]
Notes: [Additional observations]
Output: [Sample output or error message]
```

## Success Criteria Summary

A test is considered **PASSED** if:
- The tool executes without errors, OR
- The tool returns expected error (e.g., empty results for empty repo), OR
- The tool is blocked by security policy but the error is expected

A test is considered **FAILED** if:
- The tool is not available
- The tool returns unexpected errors
- The tool configuration is incorrect

## Automation Recommendations

This test script can be automated by:
1. Creating a test runner that invokes each tool systematically
2. Capturing outputs and comparing against expected results
3. Generating a test report with pass/fail status
4. Running tests as part of CI/CD pipeline validation

## Maintenance

This test script should be updated when:
- New MCP servers are added
- Existing MCP server APIs change
- New testing requirements are identified
- Security policies affecting MCP servers change

## References

- [Model Context Protocol Specification](https://spec.modelcontextprotocol.io/)
- [GitHub API Documentation](https://docs.github.com/en/rest)
- [Playwright Documentation](https://playwright.dev/)

---

Last Updated: 2025-12-05
Version: 1.0
