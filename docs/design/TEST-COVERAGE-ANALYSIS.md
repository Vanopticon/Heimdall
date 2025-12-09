# Test Coverage Analysis

This document tracks test coverage across all Heimdall modules and identifies areas requiring additional testing.

## Current Test Status

### Unit Tests

| Module | Coverage | Status | Priority | Owner |
|--------|----------|--------|----------|-------|
| `src/config/mod.rs` | Partial | ✓ Has basic tests | Medium | Core Team |
| `src/ingest/handler.rs` | Partial | ✓ Has detection tests | Medium | Core Team |
| `src/ingest/ndjson.rs` | Good | ✓ Has normalization tests | Low | Core Team |
| `src/ingest/bulk_normalizer.rs` | Partial | ✓ Has basic tests | Medium | Core Team |
| `src/devops/rate_limiter.rs` | Good | ✓ Has comprehensive tests | Low | Core Team |
| `src/tls_utils.rs` | Partial | ✓ Has basic tests | Medium | Core Team |
| `src/age_client.rs` | **None** | ❌ No unit tests | **High** | Core Team |
| `src/persist/mod.rs` | **None** | ❌ No unit tests | **High** | Core Team |
| `src/health.rs` | **None** | ❌ No unit tests | Medium | Core Team |
| `src/state.rs` | **None** | ❌ No unit tests | Low | Core Team |
| `src/devops/docker_manager.rs` | **None** | ❌ No unit tests | Medium | Core Team |

### Integration Tests

| Test File | Purpose | Status | Notes |
|-----------|---------|--------|-------|
| `tests/integration_dev_db.rs` | ✓ Tests dev DB lifecycle | Complete | Requires `RUN_DOCKER_INTEGRATION_TESTS=1` |
| `tests/integration_e2e.rs` | ✓ Tests end-to-end NDJSON ingest | Complete | Requires `RUN_DOCKER_INTEGRATION_TESTS=1` |
| `tests/integration_tls.rs` | ✓ Tests TLS certificate validation | Complete | Security-focused |

### Test Coverage Gaps

#### High Priority (Security & Data Integrity)

1. **`src/age_client.rs`** — No unit tests for:
	- `sanitize_prop_key()` function — critical for injection prevention
	- `sanitize_label()` function — critical for injection prevention
	- `merge_entity()` error handling
	- Edge cases: empty properties, special characters, null values
	- **Security**: Test against Cypher injection attempts

2. **`src/persist/mod.rs`** — No unit tests for:
	- `submit_job()` function and metrics tracking
	- `metrics_text()` output format
	- Batcher logic (buffering, flushing)
	- Error handling when repository fails
	- **Security**: Test batch size limits and resource exhaustion

#### Medium Priority (Core Functionality)

3. **`src/health.rs`** — No unit tests for:
	- Health check endpoint logic
	- Error response formatting
	- Database connectivity failure scenarios

4. **`src/tls_utils.rs`** — Expand tests for:
	- Certificate chain validation
	- TLS version enforcement (TLS 1.3 only)
	- **Security**: Test weak cipher rejection

5. **`src/devops/docker_manager.rs`** — No unit tests for:
	- Docker container lifecycle
	- Error handling for missing Docker daemon
	- Timeout handling

6. **`src/ingest/handler.rs`** — Expand tests for:
	- Multipart upload handling
	- Error responses for invalid inputs
	- Rate limiting integration
	- **Security**: Test file upload size limits and malicious content

7. **`src/config/mod.rs`** — Expand tests for:
	- Environment variable parsing edge cases
	- Invalid configuration handling
	- **Security**: Test secret masking in logs

#### Low Priority (Simple/Stateless)

8. **`src/state.rs`** — Simple struct, low test priority but could add:
	- Clone operation validation
	- State initialization tests

## Test Requirements by Category

### Positive Test Cases
- Happy path for all public API functions
- Valid input handling
- Expected state transitions

### Negative Test Cases
- Invalid input handling (malformed, missing, excessive)
- Boundary conditions (empty, null, max size)
- Error propagation and recovery

### Security Test Cases
- Input sanitization (injection attacks)
- Resource exhaustion (DoS scenarios)
- Authentication/authorization checks
- PII handling compliance
- TLS configuration enforcement

### Integration Test Cases
- End-to-end data flows (ingest → normalize → persist)
- Database interactions with ephemeral Postgres+AGE
- Multi-component interactions
- Error handling across component boundaries

## Testing Expectations

### Per Module

- **Unit Tests**: All public functions and critical internal logic
- **Coverage Target**: 70%+ line coverage for core modules
- **Test Organization**: Use `#[cfg(test)]` modules within source files
- **Documentation**: Each test should have a clear description

### Integration Tests

- Must use ephemeral test databases (Docker-based Postgres+AGE)
- Must clean up resources after execution
- Should be gated by feature flags or environment variables for CI control
- Should test realistic data flows and error scenarios

### E2E Tests

- Test complete user workflows (upload → process → verify)
- Validate API contracts and response formats
- Test error handling from user perspective

### Security Tests

- Input validation and sanitization
- Authentication and authorization
- TLS configuration and certificate validation
- Resource limits and DoS protection
- PII handling and encryption

## Implementation Plan

### Phase 1: High-Priority Security Tests (Current Sprint)

- [ ] Add unit tests for `age_client.rs` sanitization functions
- [ ] Add security tests for Cypher injection prevention
- [ ] Add unit tests for `persist/mod.rs` metrics and batching
- [ ] Add tests for resource exhaustion scenarios

### Phase 2: Core Module Coverage (Next Sprint)

- [ ] Add tests for `health.rs` endpoint logic
- [ ] Expand `tls_utils.rs` tests for certificate validation
- [ ] Add tests for `devops/docker_manager.rs`
- [ ] Expand `ingest/handler.rs` tests for error handling

### Phase 3: Integration & E2E (Following Sprint)

- [ ] Add more integration tests for multi-component flows
- [ ] Add e2e tests for bulk upload scenarios
- [ ] Add tests for enrichment pipeline (when implemented)
- [ ] Add tests for sync operations (when implemented)

## CI Integration

See `docs/design/CI-GATING-RULES.md` for CI configuration and gating thresholds.

## Ownership & Assignments

| Area | Owner | Status |
|------|-------|--------|
| Core persistence & graph | Core Team | In Progress |
| Ingest & normalization | Core Team | Partial |
| TLS & security | Core Team | Partial |
| DevOps & infrastructure | Core Team | Planned |
| Integration tests | Core Team | In Progress |

## Metrics & Tracking

- Current unit test count: 12 tests
- Current integration test count: 3 tests
- Modules with no tests: 5 (high-priority: 2)
- Target: 70%+ coverage for core modules
- Target: 100% of critical security functions tested

## Notes

- Integration tests require `RUN_DOCKER_INTEGRATION_TESTS=1` environment variable
- Docker must be available for integration tests
- Tests should be idempotent and cleanup after themselves
- All new code should include tests as part of the PR
