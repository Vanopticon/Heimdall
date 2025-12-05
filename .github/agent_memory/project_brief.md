# Heimdall — Project Brief

Heimdall is the central ingestion and processing hub for the Vanopticon suite. It accepts telemetry and vendor feeds, normalizes and enriches them, and routes them into downstream analytics and storage. The project is designed for high-throughput, resilient processing with a graph-centric data model for linkage and correlation.

Key capabilities:

- Ingest logs, sensors, APIs and vendor connectors via HTTP endpoints and upload handlers.
- Normalize, deduplicate and enrich data (ASN/geo, NPI classification) and represent it in a graph for correlation.
- Pluggable pipelines and a lightweight backend API for ingestion and UI.
- Graph backend powered by PostgreSQL + AGE (via `age-schema-client`) and a graph model defined under `sql/v1`.

Primary components:

- `server/` — Node + Express server wrapper around the SvelteKit build, OIDC auth, TLS, and session handling.
- `src/` — SvelteKit app and API endpoints (e.g., `src/routes/api/upload`).
- `src/lib/server/ageClient.ts` — Age schema client wrapper for graph operations.
- `docs/` — Architecture and Data Model docs.

Acceptance / operational notes:

- Uses OIDC for authentication (`openid-client`) and enforces TLS 1.3 in `server/server.ts`.
- Database connection comes from environment variables (supports `VOH_DATABASE_URL` / `DATABASE_URL`).
- Private repository package; `pnpm` + Node toolchain expected for development.

Reference files: `README.md`, `docs/design/Architecture.md`, `docs/design/DataModel.md`, `server/server.ts`, `src/lib/server/ageClient.ts`.
