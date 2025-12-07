# Feature: OBS-001 -> Observability & Testing Suite

## Related Items

- See: `docs/design/Architecture.md`, `docs/design/Implementation-Roadmap.md`

## Story

As a developer, I want comprehensive metrics, structured logs, traces, and tests so the system is monitorable and reliable.

## Overview

- Structured JSON logging to `stdout`/`stderr`.
- Prometheus metrics (ingest rate, queue length, sync lag, enrich failures).
- OpenTelemetry traces for request flows (ingest → enrichment → sync).
-- Test harness: unit tests, integration tests using ephemeral Postgres+AGE containers, and e2e tests for sync/enrichment (mock providers).

## Implementation Steps

1. Add structured logging and basic metrics in core modules.
2. Instrument critical code paths with tracing spans.
3. Create test harness that spins up ephemeral Postgres+AGE containers for integration tests (via `docker-compose` or `testcontainers`).
4. Implement e2e scenarios for ingestion → enrichment → sync.

## Acceptance Criteria

- Metrics and traces are available and critical test scenarios pass in CI/local runs.
