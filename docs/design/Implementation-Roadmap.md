# Implementation Roadmap — Heimdall

This roadmap breaks the work into milestones and suggested sprint-sized deliverables. Each milestone includes acceptance criteria and test expectations.

## Milestone 0 — Foundation

Timeline: 1–2 sprints

Deliverables:

- Project scaffolding and core configuration module (env-first).
- Streaming ingest API endpoint with minimal CSV/JSON/NDJSON parser.
- Graph schema for core entities: `dump`, `field`, `field_value`, `sighting`, `entity` and provenance metadata implemented in PostgreSQL+Apache AGE (graph schema) and `pgvector` for vector embeddings.
- Unit tests for parser and normalizers.

Acceptance:

- Able to ingest a small NDJSON dump and observe canonical nodes in the Postgres+AGE graph.

## Milestone 1 — Normalization & PII

Timeline: 1–2 sprints

Deliverables:

- Canonicalizers for IP, domain, and common hash formats.
- PII policy engine and envelope encryption implementation (env-injected master key).
- Tests ensuring PII is never stored plaintext.

Acceptance:

- Upload a test dump containing PII; verify stored values are hashed/encrypted or scrubbed.

## Milestone 2 — Enrichment Framework

Timeline: 1–2 sprints

Deliverables:

- Enricher worker framework and adapter interface.
- Provider configuration schema (rate-limits, credentials).
- Sample adapters (geoip, ASN).
- Integration tests with mocked providers.

Acceptance:

- Enrichment nodes/edges appear and are linked with source records.

## Milestone 3 — Sync & Multi-instance

Timeline: 2–3 sprints

Deliverables:

- Append-only change-log and per-record version vectors.
- Sync agent (push/pull) with TLS 1.3 and OIDC-backed control plane authentication.
- Merge resolver with configurable rules per entity type.
- Tests simulating partition and reconciliation.

Acceptance:

- Two Heimdall instances reconcile changes after simulated network partitions.

## Milestone 4 — Observability & Harden

Timeline: 1–2 sprints

Deliverables:

- Structured logging, Prometheus metrics, and OpenTelemetry traces.
- Expanded test coverage (overflow, malformed input, cert errors).
- Operational docs: backup/restore, TLS bootstrapping, key rotation playbook.

Acceptance:

- Metrics and traces available; e2e tests pass.

## Milestone 5 — Ops & Production Prep

Timeline: 1 sprint

Deliverables:

- Deployment runbook for Linux hosts (use Postgres+AGE sidecar or managed Postgres with AGE installed).

Acceptance:

## Cross-Cutting Requirements

- Testing: Unit + integration + e2e tests for all new features.
- Security: PII controls, TLS 1.3 enforcement, OIDC token validation.
- Observability: JSON logs, Prometheus metrics, traces.
- 100% Rust implementation; all CLI/output to `stdout`/`stderr`.
