# System Patterns

- **ETL pipeline pattern:** modular stages for parsing, canonicalization, enrichment, and routing. Pipeline stages should be pluggable and composable.
- **Streaming-first ingestion:** handlers accept streaming HTTP multipart and raw payloads to avoid large memory spikes.
- **Graph-backed model:** entities and sightings stored in a graph (Postgres+AGE recommended). Use idempotent MERGE-style operations to prevent duplication.
- **Enrichment as async workers:** enrichers run asynchronously and should be pluggable to avoid blocking the ingest path.
- **Authentication patterns:** OIDC for interactive users; OAuth2 client credentials for machine-to-machine integrations.
- **Operational patterns:** TLS 1.3 enforcement, secure session cookies, rate limiting, and comprehensive logging.
- **Testing patterns:** unit tests for graph operations, integration tests against disposable AGE-enabled Postgres instances for schema drift detection.
