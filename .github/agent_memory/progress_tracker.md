# Progress Tracker — Heimdall

Last updated: 2025-12-07
Author: GitHub Copilot (architect session)

## Purpose

This file captures the step-by-step implementation plan derived from the architecture and feature cards in `docs/design/`. It is intended as the single-source-of-truth for sprint planning, status, and next actions.

## High-level Roadmap (linked)

- Architecture overview: `docs/design/Architecture.md`
- Implementation roadmap: `docs/design/Implementation-Roadmap.md`
- Feature cards: `docs/design/features/` (ING-001, SYNC-001, API-001, SEC-001, OBS-001)

## Master Plan (steps mapped to milestones)

Milestone 0 — Foundation

- Task 0.1: Project scaffolding and core config module (env-first).
    + Deliverable: `src/config` module, CI skeleton. Link: `docs/design/Implementation-Roadmap.md`
    + Owner: You + Copilot
    + Status: completed

- Task 0.2: Streaming ingest endpoint (CSV/JSON/NDJSON minimal parser).
    + Deliverable: `src/ingest` streaming API and minimal parsers. Link: `docs/design/features/ING-001-Bulk-Dump-Normalization.md`
    + Owner: You + Copilot
    + Status: in-progress

- Task 0.3: Graph schema (dump, field, field_value, sighting, entity) and basic client wrapper for Postgres+AGE.
    + Deliverable: `sql/` or `pg/age/schema` definitions + `src/lib/age_client.rs`. Link: `docs/design/Architecture.md`
    + Owner: You + Copilot
    + Status: not-started

Milestone 1 — Normalization & PII

- Task 1.1: Implement canonicalizers (IP, domain, hash).
    + Deliverable: `src/lib/normalizers/*` with unit tests. Link: `docs/design/Implementation-Roadmap.md`
    + Status: not-started

- Task 1.2: PII policy engine and envelope encryption helpers.
    + Deliverable: `src/lib/pii_policy.rs`, encryption helper using vetted Rust crate (to be selected). Link: `docs/design/features/SEC-001-PII-Policy-Field-Encryption.md`
    + Status: not-started

Milestone 2 — Enrichment Framework

- Task 2.1: Enricher adapter interface and worker pool.
    + Deliverable: `src/enrich` worker, adapter trait, sample adapters (ASN, geoip). Link: `docs/design/features/API-001-External-API-Caller-Enrichment.md`
    + Status: not-started

- Task 2.2: Provider config (rate-limits, credentials), retry/backoff and circuit-breaker.
    + Deliverable: runtime config and resilient HTTP client helpers. Link: `docs/design/Architecture.md`
    + Status: not-started

Milestone 3 — Sync & Multi-instance

- Task 3.1: Implement change-log writer and local append-only log.
    + Deliverable: `src/sync/change_log.rs`, log storage and API. Link: `docs/design/features/SYNC-001-Multi-Heimdall-Synchronization.md`
    + Status: not-started

- Task 3.2: Sync agent (push/pull) with TLS 1.3 and OIDC auth.
    + Deliverable: `src/sync/agent.rs`, peer config, and test harness simulating partitions. Link: `docs/design/Implementation-Roadmap.md`
    + Status: not-started

- Task 3.3: Merge resolver and per-entity merge rules.
    + Deliverable: `src/sync/merge.rs` with configurable rules. Link: `docs/design/Architecture.md`
    + Status: not-started

Milestone 4 — Observability & Testing

- Task 4.1: Structured logging, metrics, and tracing.
    + Deliverable: logging & metrics modules, Prometheus endpoint, OpenTelemetry spans. Link: `docs/design/features/OBS-001-Observability-Testing.md`
    + Status: not-started

-- Task 4.2: Integration & e2e test harness (ephemeral Postgres+AGE containers).
    + Deliverable: `test/integration/*`, `docker-compose.yml` and CI job definitions. Link: `docs/design/Implementation-Roadmap.md`
    + Status: not-started

Milestone 5 — Ops & Production Prep

- Task 5.1: Secret management integration plan & key rotation playbook.
    + Deliverable: `docs/ops/key-rotation.md` and `docs/ops/secrets.md`. Link: `docs/design/Implementation-Roadmap.md`
    + Status: not-started

- Task 5.2: Deployment runbook for Linux hosts (Postgres+AGE sidecar guidance).
    + Deliverable: `docs/ops/deployment-runbook.md`. Link: `docs/design/Architecture.md`
    + Status: not-started

## Cross-cutting tasks

- Test coverage: unit, integration, e2e and security edge cases. (See `docs/design/Implementation-Roadmap.md`)
- Security audit checklist: TLS, token validation, PII verification. (See `docs/design/Architecture.md`)
- Crate selection: propose concrete Rust crates for HTTP server, Postgres client (`sqlx`/`tokio-postgres`), AGE helpers (SQL/Cypher), crypto, metrics/tracing — create `docs/design/choices.md` for discussion. **Status: completed**

## Short-term Next Actions (this week)

1. Create `src/config` skeleton and CI job placeholder (Task 0.1).
2. Add minimal streaming ingest endpoint that accepts NDJSON and writes canonical records to a dev Postgres+AGE instance (Task 0.2 + 0.3).
3. Critical: Application must be able to start/stop the Postgres+AGE dev container itself (`heimdall start-db` / `heimdall stop-db`). Implemented via `src/devops`.
3. Draft `docs/design/choices.md` with recommended Rust crates to finalize before implementation.
3. `docs/design/choices.md` created and finalised per team choices (see file).

## Notes

Last updated: 2025-12-07

- Owner fields default to the two-person team: You (repository owner) and Copilot (assisting). Update ownership if needed.
- All paths are relative to repository root.
- This file is part of the LTM and should be updated as tasks progress.

# Progress Tracker

- **Goal:** Maintain an up-to-date tracker for active work, blockers, and acceptance criteria.

- **Initial items:**
    + Initialize LTM files — status: completed (this session)
    + Add tests for `src/lib/server/ageClient.rs` — status: not-started
    + Add tests for `src/routes/api/upload` streaming handling — status: not-started
    + Validate OIDC integration in staging — status: not-started

- **Acceptance Criteria:**
    + Unit tests cover positive, negative, and security cases for changed units.
    + E2E tests cover normal user flows and common error conditions.
    + All CI tests pass before merging feature branches.

- **Notes:** Update this tracker as tasks progress; reference issue numbers when present.

- **Notes:** Update this tracker as tasks progress; reference issue numbers when present.

## Session log

  Author: GitHub Copilot (assistant)
  Summary: Bootstrapped the project configuration module (Task 0.1).

  Actions taken:

   	+ Read all LTM files in `.github/agent_memory` to establish context.
   	+ Added a minimal configuration module skeleton at `src/config/mod.rs` exposing `Settings` and a `load()` helper that reads an optional `config` file and `HMD_`-prefixed environment variables.
   	+ Exported the `config` module from the crate root (`src/lib.rs`).
 Date: 2025-12-07
 Author: GitHub Copilot (assistant)
 Summary: LTM read, todo list created, attempted streaming handler implementation; reverted to committed state and ran tests.

 Actions taken:

- Read all LTM files in `.github/agent_memory` to re-establish project context (confirmed existing LTM files: `project_brief.md`, `active_context.md`, `system_patterns.md`, `tech_stack.md`, `progress_tracker.md`, `handoff.md`).
- Created a structured todo list for short-term work (see repository `manage_todo_list` entries). Marked "Implement streaming ingest handler" as in-progress.
- Attempted to implement streaming ingest logic in `src/ingest/handler.rs` (convert `bulk_dump_upload` and `ndjson_upload` to chunked streaming write to temp file / line-by-line processing). During iteration the implementation used different body streaming approaches and introduced compilation issues; to maintain repository stability I reverted `src/ingest/handler.rs` to the last committed version.
- Ran the full `cargo test` suite to validate repository remains functional after the revert. All unit and gated integration tests passed locally.

 Notes & next steps:

- Streaming ingest rewrite remains a short-term goal. The safest approach moving forward is to implement a streaming writer using the request body APIs offered by `axum`/`hyper` in a manner compatible with the project's axum version (use `Body::data()` via the `http_body::Body` trait or convert the underlying hyper body to a stream). This requires careful trait imports and compatibility with the project's dependencies (avoid introducing new heavy crates unless necessary).
- Proposed immediate action: implement streaming behavior incrementally for `bulk_dump_upload` only (write chunk-by-chunk while preserving a small peek buffer for detection). Add unit tests that exercise chunked inputs. Once stable, update `ndjson_upload` to use the same pattern.
- Update the todo list statuses after the streaming implementation is completed.

- Date: 2025-12-07
  Author: GitHub Copilot (assistant)
  Summary: Implemented normalization primitives for Task 0.2 (NDJSON parser).

  Actions taken:

   	+ Implemented a permissive NDJSON normalizer at `src/ingest/ndjson.rs` that accepts NDJSON records (objects, arrays, or simple CSV-like lines) and canonicalizes field values for `domain`, `ip`, `hash`, and `email`.
    + Exposed the NDJSON normalizer via `src/ingest/mod.rs` and added unit tests validating canonicalization for domain/ip/hash/email and array/CSV fallbacks.
    + Added a minimal HTTP handler (`src/ingest/handler.rs`) that accepts an NDJSON payload as a raw body and returns the canonicalized records as JSON (streaming implementation to follow).
   	+ Ran the library unit tests and fixed issues; all library tests pass (8 passed; 0 failed).

  Next steps (short-term):

  	1. Add an async streaming HTTP handler that accepts NDJSON/CSV uploads and pipes parsed records to the pipeline (implement in `src/ingest/handler.rs` or integrate with `run()` server flow).
  	2. Wire minimal ingestion path into `run()` so `heimdall run` can start a local HTTP ingest endpoint for development.
  	3. Create a feature branch for `ingest/ndjson` and open a tracking issue/PR against `v1.0.0`.
