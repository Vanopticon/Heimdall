# Heimdall — Central Ingestion, ETL & Data-Scrubbing Hub

Heimdall (named for the keeper of the Rainbow Bridge) is the ETL, data-scrubbing, normalization, enrichment, and tool-bridging component of the Vanopticon suite. It provides secure, resilient, and auditable ingestion pipelines that normalize and correlate telemetry into a canonical graph model for downstream analysis, detection, and response.

## Purpose

- **Ingest** telemetry from vendors, sensors, APIs, and batch uploads.
- **Transform & scrub** incoming data to canonical forms, removing PII where required and normalizing entity types (IP, domains, hashes, emails, etc.).
- **Enrich** data with ASN, geolocation, NPI classification, and other contextual metadata.
- **Correlate** entities and sightings in a graph store to enable threat linkage, deduplication, and provenance tracking.

## Architecture Overview

- **Backend (this repository)**: Rust-based service implemented with `tokio` and `axum`. The CLI and runtime entrypoint is `src/main.rs` (see the `Run`, `StartDb`, and `StopDb` subcommands).
- **Graph & vector store**: PostgreSQL + Apache AGE + `pgvector` stores nodes, edges, provenance, and vector embeddings. Use the included `docker/` and `scripts/` helpers for local development.
- **Pipelines & APIs**: Streaming ingest handlers and parsing adapters live in `src/ingest/` (NDJSON/CSV adapters and HTTP handlers). Enrichers, sync agents, and operational helpers are under `src/` as `src/enrich/`, `src/sync/`, and `src/devops/`.
- **Security & Observability**: TLS 1.3, OIDC token validation, structured JSON logs, Prometheus metrics, and OTLP/OpenTelemetry tracing (design details in `docs/design/`).

Refer to `docs/design/Architecture.md` and `docs/design/DataModel.md` for diagrams and the canonical graph model.

## Data Model (high level)

- Graph-centered: canonical nodes include `Dump`, `Field`, `FieldValue`, `Entity`, `IP`, `NPI_Category`, and `Sighting` to capture occurrences and provenance.
- Deduplication and cross-field sightings are used to surface linkages between otherwise separate records.

See `sql/v1/001-create_graph.sql` for the authoritative schema and indices.

## Getting Started (Development)

Prerequisites:

- Rust toolchain (rustup + cargo) — recommended stable channel.
- Docker and `docker-compose` (for Postgres+AGE local dev), or the helper scripts in `scripts/`.

Quick dev run (local, minimal):

```bash
# Build the workspace
cargo build --workspace

# Start the dev Postgres+AGE (preferred: use the built-in helper)
# This will run the docker-compose-based dev DB used for integration testing.
cargo run -- StartDb --timeout 120

# Run the application (default runtime)
cargo run -- run

# Stop the dev DB
cargo run -- StopDb

# Run tests
cargo test
```

If you prefer script helpers, you can use the provided scripts:

```bash
scripts/start-postgres.sh
scripts/stop-postgres.sh
```

Notes:

- The runtime entrypoint is `src/main.rs`. The CLI exposes dev DB helpers as subcommands; see `--help` for details.
- Database connectivity is taken from `HMD_DATABASE_URL` or `PG*` environment variables. The logical graph name inside AGE is controlled by `AGE_GRAPH`.

## Configuration (important environment variables)

- `HMD_HOST`, `HMD_PORT` — host and port the server binds to.
- `HMD_TLS_KEY`, `HMD_TLS_CERT` — TLS key and certificate paths (required for TLS deployments).
- `HMD_OAUTH_DISCOVERY_URL`, `HMD_OAUTH_AUTH_ID`, `HMD_OAUTH_AUTH_SECRET`, `HMD_OAUTH_SCOPES` — OIDC configuration used for interactive user login.
- `HMD_DATABASE_URL` or `PGHOST` / `PGDATABASE` / `PGUSER` / `PGPASSWORD` — database connection.
- `AGE_GRAPH` — logical graph name inside the AGE-enabled database.

Keep secrets out of source control and use a secrets manager in production.

## Security & Operations

- TLS 1.3 is recommended and enforced in production configurations.
- Authentication/authorization: external OIDC/OAuth2 provider; tokens are validated locally.
- PII handling: policy-driven field-level rules (scrub, one-way hash, envelope encryption). Two-way decryption operations must be audited.

Operational checklist for deployment:

1. Provision a Postgres instance with AGE and `pgvector` (use `docker/postgres-age/` for local dev).
2. Apply SQL schema in `sql/v1/`.
3. Provide TLS certs and OIDC client credentials via environment variables or a secrets manager.
4. Configure logging, monitoring (Prometheus), and backups for the graph database.

## Tests and Quality

- Unit, integration, and e2e tests are implemented with Rust test harnesses. Integration tests run against an ephemeral Postgres+AGE instance (Docker Compose).
- Run `cargo test` for unit/integration tests. See `docs/design/Implementation-Roadmap.md` for testing expectations.

## Contributing

- Follow `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md` for PRs and code reviews.
- Use a feature branch off of `v1.0.0` and open a PR against that branch (do not open PRs directly to `main`).

## Roadmap / Suggested Features

- Add configurable enrichment workers so enrichers can run asynchronously.
- Add a plugin system for vendor connector transforms and signature parsers.
- Add end-to-end tests that run against a disposable AGE-enabled Postgres instance in CI to catch schema drift.
- Improve observability: structured logs and metrics (Prometheus) for ingestion throughput, error rates, and enrichment latency.

## Persistence and Metrics

- Persistence: Incoming raw uploads are written to a temporary file for
	offline analysis when using the bulk upload endpoint. Raw payloads are
	NOT written directly to the graph database. Only normalized and
	sanitized records (canonical values) are enqueued to the background
	batcher and persisted into Postgres+AGE via MERGE statements.

- Batching & safety: The service batches persistence jobs to reduce DB
	round-trips. Property keys are sanitized to a safe identifier form
	(alphanumeric and underscore) and string values are JSON-serialized
	when embedded into Cypher to reduce injection surface. Further
	hardening (parameterized UNWIND batches) is recommended for
	high-security deployments.

- Metrics: A lightweight Prometheus-compatible `/metrics` endpoint is
	exposed by the dev server that reports simple persistence metrics
	(jobs submitted, batch flushes, failures, and cumulative batch
	latency). This is intentionally minimal to avoid adding another
	runtime dependency; integrate a Prometheus client if you need richer
	metric types and labels.

## Where to Look Next

- `src/main.rs` — CLI and runtime entrypoint.
- `src/age_client.rs` — Postgres+AGE helper client.
- `src/ingest/` — ingest handlers and parsers (NDJSON/CSV, streaming helpers).
- `src/devops/` — dev DB helpers and docker helpers.
- `sql/v1` — SQL schema for the canonical graph model.

---

License: see `LICENSE.md` in the repository root.
