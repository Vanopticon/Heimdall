# Architecture — Heimdall

## Purpose

Heimdall is the ETL, normalization, enrichment, and correlation hub for telemetry. It ingests bulk data dumps, canonicalizes and scrubs sensitive fields (no PII in plaintext), enriches records via configurable external providers, and persists a canonical graph model for queries and downstream analysis. Multiple Heimdall instances replicate and synchronize continuously to provide resilience and geographic distribution.

## High-level Components

- Ingest API / Bulk Upload Handler — streamed handlers for large dumps (CSV/JSON/NDJSON/vendor formats). Keeps memory usage bounded.
- Parser & Normalizer Pipeline — adapters for input formats, canonicalizers for IP, domain, hashes, timestamps, and canonical-key generation for idempotent writes.
- PII Policy Layer — per-field rules (scrub, one-way hash, two-way envelope encryption). Defaults to highest classification.
-- Canonical Graph Store — PostgreSQL + Apache AGE + `pgvector` stores nodes, edges, provenance, and vector embeddings.
- Enrichment Workers — pluggable adapters that call external APIs (configurable providers, rate-limiters, retry/backoff, circuit-breaker).
- Sync Agent — change-log replication and reconciliation between Heimdall peers over TLS 1.3 with OIDC-authenticated control plane. Uses per-record version vectors for conflict detection and merge rules for reconciliation.
- API / Query Layer — internal REST/gRPC API for integrations and a public ingest API. Enforces RBAC using external OIDC claims.
- Observability — structured JSON logs to stdout/stderr, Prometheus metrics endpoint, OpenTelemetry traces.

## Data Flow

1. Client uploads bulk dump (streamed).
2. Parser emits records to normalization pipeline.
3. Apply PII policy to fields (scrub/hash/encrypt).
4. Generate canonical keys and merge into the graph store (Postgres+AGE) with provenance metadata and a change-log entry.
5. Enrichment workers consume changefeed, call configured providers, and persist enrichment nodes/edges.
6. Sync agent ships change-log entries to peer Heimdall instances; peers reconcile using version vectors and merge rules.
7. Query/API layer returns correlated results, honoring RBAC and field-level classification.

## Security & Compliance

- All network communication: TLS 1.3 only.
- Authentication/authorization: external OIDC/OAuth2 provider (tokens validated locally).
- PII handling: never stored plaintext. Two-way encryption uses envelope encryption with an environment-provided master key (KMS/Vault planned later).
- Audit trail: every write includes actor (OIDC sub), request id, and timestamp. Logs are structured and auditable.
- Default classification: data treated as highly classified until annotated otherwise.

## Storage & Replication

- Primary store: PostgreSQL + Apache AGE + `pgvector` (graph and vector store). Run as a Docker sidecar for local dev and CI; production deployments may use managed Postgres with AGE installed.
   	+ Replication: peer-to-peer change-log replication with per-record version vectors.
   	+ Backups: periodic Postgres backups (pg_dump/pg_basebackup) including graph metadata; nothing in Heimdall is single-source-of-truth that cannot be re-derived from raw dumps and enrichments.

## Operational Considerations

- Config: environment-first; secrets via env for now.
- Logs: JSON to stdout for aggregation by host-level log collector.
- Telemetry: Prometheus metrics + OpenTelemetry traces (collector endpoint configurable).
- TLS certs: provided by deployment; no self-signed certificates permitted.

## Non-functional Requirements

- Implementation language: Rust (100%).
- Logging: all output to `stdout`/`stderr` in structured JSON.
- Testing: unit, integration (ephemeral Postgres+AGE containers via Docker Compose or testcontainers), and e2e sync tests.

## Decisions & Trade-offs

-- PostgreSQL + Apache AGE chosen to satisfy the requirement of a graph-capable internal data store, with `pgvector` for vector embeddings. Operational trade-offs: database extensions require build/install steps; using Docker sidecars simplifies local development and CI.

- Sync via change-log + version vectors (eventual consistency) chosen over a fully CRDT-based approach for implementation simplicity and deterministic conflict-resolution rules.
- Envelope encryption using environment-injected master key chosen for initial simplicity and operational compatibility; swap to KMS/Vault planned for Milestone 5.

## Next Steps

- Create feature cards and implementation roadmap (in `docs/design/features/` and `docs/design/Implementation-Roadmap.md`).
-- Select concrete Rust crates for HTTP server, Postgres client (`tokio-postgres`/`sqlx`), AGE helpers (e.g., `age-schema-client` or raw SQL execution), crypto primitives, metrics, and tracing.

---
## System Diagram (Mermaid)

```mermaid
graph LR
	Client[Client / Integrations]
	IngestAPI[Ingest API / Bulk Handler]
	Parser[Parser & Normalizer]
	PII[PII Policy Layer]
	Age[Postgres + Apache AGE + pgvector]
	Enrich[Enrichment Workers]
	Sync[Sync Agent / Change-log Replication]
	API[API / Query Layer]
	DevDB[Dev Postgres+AGE (docker-compose)]

	Client -->|HTTP / Streaming Upload| IngestAPI
	IngestAPI --> Parser
	Parser --> PII
	PII -->|MERGE / Upsert| Age
	Age -->|changefeed| Enrich
	Enrich -->|enrichment writes| Age
	Age <-->|change-log (TLS)| Sync
	API -->|queries| Age
	DevDB --> Age
```

## Component ↔ Code Mapping

- **CLI / Runtime**: `src/main.rs` — parses CLI, starts devops helpers and runtime.
- **Ingest API / Handlers**: `src/ingest/handler.rs` — HTTP handlers and streaming endpoints (bulk upload, NDJSON).
- **Parser & Normalizers**: `src/ingest/bulk_normalizer.rs`, `src/ingest/ndjson.rs` — canonicalizers and lightweight normalization helpers.
- **Storage / AGE Client**: `src/age_client.rs` — minimal wrapper for executing Cypher against Postgres+AGE.
- **Devops / Local DB Helpers**: `src/devops/docker_manager.rs` — start/stop dev Postgres+AGE for integration testing.
- **Configuration**: `src/config/mod.rs` — env-first configuration loader.
- **Enrichment & Sync**: `src/enrich/` and `src/sync/` (planned) — worker pool, adapters, and sync agent implementations.

## Immediate Recommendations (short-term)

1. **Harden AGE client**: `AgeClient::merge_entity` currently constructs Cypher strings directly. Refactor to use parameterization or strict validation/escaping, and consider prepared statements where possible to avoid injection or malformed payloads.
2. **Add persistence helpers**: Implement a `src/persistence.rs` (or `src/age_persistence/`) module exposing `persist_row` / `persist_dump` helpers that encapsulate the MERGE semantics, batching, and co-occurrence upserts described in `DataModel.md`.
3. **Batch co-occurrence updates**: Materialize pairwise increments in batched upserts (or use a staging table + periodic aggregation) to avoid O(n^2) write storms for wide rows.
4. **PII & Key Management**: Plan for KMS/Vault integration for envelope encryption rather than env-injected master keys. Add a key-rotation and audit plan to `docs/design/` and `/memories`.
5. **Tests & Integration**: Add integration tests that exercise ingest → persistence → query flows using the existing `docker/` dev DB; include negative/malformed inputs and memory/backpressure scenarios.

## Acceptance Criteria (for Architecture Review)

- `docs/design/Architecture.md` contains a system diagram and a component-to-code mapping.
- A machine-readable feature spec exists at `docs/design/features/ARCH-001-Architecture-Review.feature.json` describing steps and acceptance criteria.
- `docs/design/Implementation-Roadmap.md` includes tasks and expected tests for implementing the recommendations.
- A memory entry (`/memories/Feature-ARCH-Architecture-Review.md`) documents decisions and next steps.

## Migration & Rollback Notes

- Persisted data in Postgres+AGE should remain readable before/after refactors; any change that alters canonical keys or row-hash computation MUST be accompanied by a migration plan that re-ingests or backfills derived nodes.
- For critical changes (e.g., canonical-key version bump), provide a compatibility layer that tags old nodes with `canonical_key:vN` and new nodes with `canonical_key:vN+1` during migration.

## Security Considerations

- Avoid string-concatenated Cypher from untrusted input — parameterize or strictly validate JSON -> Cypher map generation.
- Ensure envelope encryption keys are not stored in plaintext in env or repos; prefer KMS/Vault with short-lived credentials.
- Enrichment provider credentials must be stored encrypted and accessed by worker processes with minimal privileges.

---

File generated/updated by architecture-review automation.
