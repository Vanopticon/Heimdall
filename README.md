
# Heimdall — Central Ingestion, ETL & Data-Scrubbing Hub

Heimdall (named for the keeper of the Rainbow Bridge) is the ETL, data scrubbing, normalization, enrichment, and tool-bridging component of the Vanopticon suite. It provides secure, resilient, and auditable ingestion pipelines that normalize and correlate telemetry into a graph model for downstream analysis, detection, and response.

## Purpose

- **Ingest** telemetry from vendors, sensors, APIs, and batch uploads.
- **Transform & scrub** incoming data to canonical forms, removing PII where required and normalizing entity types (IP, domains, hashes, emails, etc.).
- **Enrich** data with ASN, geolocation, NPI classification, and other
	contextual metadata.
- **Correlate** entities and sightings in a graph store to enable threat
	linkage, deduplication, and provenance tracking.

## Key Features

- **Zero NPI Output**: All output has been scrubbed clean of NPI, or the NPI has been one way encoded to allow it to still be processed securely.
- **Flexible ingestion**: HTTP APIs and upload handlers accepting raw payloads	and multipart uploads. Streaming writes avoid large memory spikes.
- **Graph-backed model**: Uses PostgreSQL + AGE to represent entities, their	relationships, and sighting provenance for high-fidelity correlation.
- **Idempotent operations**: Merge/`MERGE`-style graph operations prevent	duplication and make re-ingest safe.
- **Pluggable pipeline stages**: Parsing, canonicalization, enrichment, and	routing stages designed to be extended with vendor connectors or enrichers.
- **Secure-by-default runtime**: OIDC-based authentication, strict	Content-Security-Policy, TLS 1.3 enforcement, secure session cookies,	and rate limiting.
- **Operational tooling**: Clear environment-driven configuration, logging,	and error handling for production readiness.

## Architecture Overview

- **Frontend / UI**: Built with SvelteKit and shipped via the Node adapter (SSR
	capable). Frontend is served from the build output by the Express wrapper.
- **Application Server**: An Express wrapper (`server/server.ts`) provides TLS, authentication support for both interactive users and machine clients (OIDC via `openid-client` for user login; OAuth2 Client Credentials for machine-to-machine integrations), session management, and serves the SvelteKit handler.
- **Graph & vector store**: A database with graph and vector capabilities stores the Heimdall data model. Postgres+AGE is a common and recommended choice for development; when using Postgres+AGE the project uses `age-schema-client` (wrapped in `src/lib/server/ageClient.ts`) for schema-aware operations and raw Cypher fallbacks. Other graph/vector-capable stores can be used in production deployments.
- **Pipelines & APIs**: Ingest endpoints (e.g. `src/routes/api/upload/+server.ts`)
	and future pipeline workers normalize and enrich incoming telemetry.

Refer to `docs/design/Architecture.md` and `docs/design/DataModel.md` for
diagrams and the canonical graph model.

## Data Model (high level)

- Graph-centered: nodes for `dumps`, `fields`, `field_value`, `ip`, `NPI_Category`,
	and `sightings` to capture occurrences and provenance.
- Support for deduplication and cross-field sightings to surface linkages
	between otherwise separate records.

See `sql/v1/001-create_graph.sql` for the authoritative schema and indices.

## Getting Started (Development)

Prerequisites:

- Node.js (recommended LTS) and `pnpm` installed.
- A database with graph and vector capabilities (Postgres+AGE recommended for integration tests / dev).

Quick dev run (local, minimal):

```bash
pnpm install
pnpm build
# Run the server (reads TLS cert and key from env, or place certs locally)
NODE_ENV=development VOH_HOST=127.0.0.1 VOH_PORT=443 \
	VOH_TLS_KEY=/path/to/tls.key VOH_TLS_CERT=/path/to/tls.crt \
	VOH_OAUTH_DISCOVERY_URL="https://example/.well-known/openid-configuration" \
	VOH_OAUTH_AUTH_ID=client_id VOH_OAUTH_AUTH_SECRET=client_secret \
	pnpm dev
```

Notes:

- `server/server.ts` enforces a set of required environment variables. At
	startup the server will validate that the OIDC discovery URL is reachable
	and will exit on misconfiguration.
- Database connectivity is taken from `VOH_DATABASE_URL` or the normal
	`PG*` environment variables and falls back to sane defaults in the code but
	should be configured for production.

## Configuration (important environment variables)

- `VOH_HOST`, `VOH_PORT` — host and port the server binds to.
- `VOH_TLS_KEY`, `VOH_TLS_CERT` — TLS key and certificate paths (required).
-- `VOH_OAUTH_DISCOVERY_URL`, `VOH_OAUTH_AUTH_ID`, `VOH_OAUTH_AUTH_SECRET`,
	`VOH_OAUTH_SCOPES` — OIDC configuration used for interactive user login (browser-based login flows). Machine-to-machine (service-to-service) integrations should use the OAuth2 Client Credentials (client credentials) flow and obtain bearer tokens from your identity provider; client credentials and service tokens should be managed securely by your deployment/secret management solution.
- `VOH_DATABASE_URL` or `PGHOST` / `PGDATABASE` / `PGUSER` / `PGPASSWORD` —
	database connection information.
- `AGE_GRAPH` — logical graph name inside the AGE-enabled database.

Keep secrets out of source control and use a secrets manager for production.

## Security & Operations

- TLS 1.3 is required by the default server; verify certificate management for
	automated renewals (ACME / cert-manager) in production.
- OIDC is used for authentication; ensure client credentials and redirect
	URIs are registered and rotated as appropriate.
- Rate limiting and request size limits are configured to reduce abuse.

Operational checklist for deployment:

1. Provision a database with the required graph and vector capabilities (Postgres+AGE recommended) and run the SQL schema in `sql/v1` or apply equivalent schema/transformations for the chosen store.
2. Configure secrets and TLS certs in a secure store or mounted path.
3. Validate OIDC discovery and client credentials in a staging environment.
4. Configure logging, monitoring, and backups for the graph database.

## Tests and Quality

- Unit and integration tests should be added for:
   	+ `src/lib/server/ageClient.ts` graph operations (mock or test DB).
   	+ `src/routes/api/upload` streaming handling and error paths.
   	+ OIDC-related flows should be validated in integration tests.

This repository includes `vitest` and Svelte testing tooling; extend tests
before adding new ingestion logic.

## Contributing

- Follow `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md` for PRs and code reviews.
- Use a feature branch off of `v1.0.0` and open a PR to that branch (do not
	open PRs directly to `main`).

## Roadmap / Suggested Features

- Add configurable enrichment workers so enrichers can run asynchronously.
- Add a plugin system for vendor connector transforms and signature parsers.
- Add end-to-end tests that run against a disposable AGE-enabled Postgres
	instance in CI to catch schema drift.
- Improve observability: structured logs and metrics (Prometheus) for
	ingestion throughput, error rates, and enrichment latency.

## Where to Look Next

- `server/` — Express wrapper and runtime configuration.
- `src/lib/server/ageClient.ts` — graph client wrapper and examples of Cypher
	operations used by the app.
- `src/routes/api/upload/+server.ts` — streaming upload endpoint.
- `sql/v1` — SQL schema for the graph model.

---

License: see `LICENSE.md` in the repository root.
