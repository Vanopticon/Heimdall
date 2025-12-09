# CI Gating Rules & Coverage Thresholds

This document defines the CI pipeline gating rules, coverage thresholds, and testing requirements for Heimdall.

## Overview

All code changes must pass automated CI checks before merging. The CI pipeline enforces code quality, security, and test coverage standards.

## Current CI Pipeline

Location: `.github/workflows/test.yml`

### Pipeline Stages

1. **Build** — Compile all workspace crates
2. **Lint** — Run `rustfmt` and `clippy`
3. **Test** — Run unit tests (default feature set)
4. **Integration** — Run integration tests (gated by environment variable)

## Test Execution Strategy

### Unit Tests

- **Trigger**: Every push and pull request
- **Command**: `cargo test`
- **Default Features**: `unit-tests` feature enabled by default
- **Coverage Target**: 70%+ for core modules
- **Gating**: Must pass all unit tests

### Integration Tests

- **Trigger**: Pull requests to `main` or `v1.0.0` branches
- **Command**: `cargo test --features integration-tests`
- **Requirements**:
	- Docker available for ephemeral Postgres+AGE instances
	- Environment variable `RUN_DOCKER_INTEGRATION_TESTS=1` must be set
- **Gating**: Must pass all integration tests
- **Cleanup**: Containers must be stopped after tests

### E2E Tests

- **Trigger**: Pull requests to main branches
- **Command**: `cargo test --features all-tests`
- **Requirements**: Full stack including database
- **Gating**: Must pass all e2e tests

### Security Tests

- **Trigger**: Every pull request
- **Requirements**:
	- CodeQL analysis (if configured)
	- Input sanitization tests
	- TLS configuration tests
	- Authentication/authorization tests
- **Gating**: No high-severity security issues

## Coverage Thresholds

### Minimum Requirements

| Module Type | Coverage Target | Enforcement |
|-------------|-----------------|-------------|
| Core modules (`age_client`, `persist`) | 70% | Recommended |
| Security modules (`tls_utils`, `health`) | 80% | Recommended |
| Ingest modules | 70% | Recommended |
| Utilities & helpers | 60% | Recommended |
| Config modules | 70% | Recommended |

**Note**: Coverage enforcement is currently **recommended** but not blocking. Future updates will make these thresholds **mandatory** for merges.

### Coverage Reporting

- Coverage reports generated using `cargo tarpaulin` (to be integrated)
- Reports uploaded to PR comments for visibility
- Trend tracking to prevent coverage regressions

## Feature Flags for Testing

The `Cargo.toml` defines test feature flags:

```toml
[features]
default = ["unit-tests"]
all-tests = ["devops-tests", "ingest-tests", "integration-tests", "unit-tests"]
devops-tests = []
ingest-tests = []
integration-tests = []
unit-tests = []
```

### Usage

```bash
# Run only unit tests (default)
cargo test

# Run integration tests
cargo test --features integration-tests

# Run all tests
cargo test --features all-tests

# Run specific test group
cargo test --features ingest-tests
```

## Test Organization Standards

### Unit Tests

- **Location**: `#[cfg(test)]` modules within source files
- **Naming**: `mod tests { ... }` or `mod <module>_tests { ... }`
- **Scope**: Test individual functions/structs in isolation
- **Dependencies**: Mock external dependencies where possible

### Integration Tests

- **Location**: `tests/` directory at repository root
- **Naming**: `integration_*.rs` pattern
- **Scope**: Test component interactions
- **Dependencies**: Use Docker for ephemeral databases

### E2E Tests

- **Location**: `tests/` directory at repository root
- **Naming**: `e2e_*.rs` pattern
- **Scope**: Test complete workflows from API to database
- **Dependencies**: Full stack with database and services

## CI Workflow Updates (Planned)

### Phase 1: Current State
- ✓ Basic unit test execution
- ✓ Integration tests with Docker (manual trigger)
- ✓ Rust compilation and basic linting

### Phase 2: Enhanced Coverage
- [ ] Add `cargo tarpaulin` for coverage reports
- [ ] Add coverage threshold enforcement
- [ ] Add coverage trend tracking
- [ ] Publish coverage badges to README

### Phase 3: Advanced Checks
- [ ] Add mutation testing for critical modules
- [ ] Add fuzz testing for parsers
- [ ] Add performance regression tests
- [ ] Add security scanning (Clippy security lints)

## Gating Policy

### Merge Requirements

All pull requests must:

1. **Pass all unit tests** — No test failures allowed
2. **Pass all integration tests** — If Docker tests are enabled
3. **Pass linting** — `cargo fmt` and `cargo clippy` must pass
4. **Build successfully** — All workspace crates must compile
5. **No new warnings** — Compiler warnings must be addressed
6. **Security checks pass** — No high-severity issues

### Exceptions

- **Documentation-only changes**: May skip integration tests
- **Test additions**: Coverage increases are always welcome
- **Emergency fixes**: May be merged with reduced checks (requires justification)

## Running Tests Locally

### Quick Test Run

```bash
# Run unit tests only
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Full Test Suite

```bash
# Run all tests including integration
export RUN_DOCKER_INTEGRATION_TESTS=1
cargo test --features all-tests
```

### Pre-Push Checklist

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features -- -D warnings

# Run unit tests
cargo test

# Run integration tests (if applicable)
export RUN_DOCKER_INTEGRATION_TESTS=1
cargo test --features integration-tests
```

## Test Failure Response

### On Failure

1. **Review test output** — Understand what failed and why
2. **Fix the issue** — Update code to pass tests
3. **Run locally** — Verify fix before pushing
4. **Don't skip tests** — Skipping tests masks issues

### Flaky Tests

- **Report immediately** — Open an issue for flaky tests
- **Investigate root cause** — Race conditions, timing issues, resource conflicts
- **Fix or remove** — Flaky tests reduce confidence in CI
- **Don't increase timeouts blindly** — Fix the underlying issue

## Future Enhancements

### Coverage Tool Integration

```yaml
# Planned CI step
- name: Generate coverage report
  run: |
    cargo install cargo-tarpaulin
    cargo tarpaulin --out Xml --output-dir ./coverage

- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
  with:
    files: ./coverage/cobertura.xml
```

### Mutation Testing

```bash
# Planned mutation testing
cargo install cargo-mutants
cargo mutants --features all-tests
```

### Benchmarking

```bash
# Planned performance tests
cargo bench
```

## References

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Feature Flags](https://doc.rust-lang.org/cargo/reference/features.html)
- [Integration Testing with Docker](https://docs.docker.com/engine/api/)
- `docs/design/Implementation-Roadmap.md` — Testing expectations per milestone
- `docs/design/TEST-COVERAGE-ANALYSIS.md` — Module coverage tracking
