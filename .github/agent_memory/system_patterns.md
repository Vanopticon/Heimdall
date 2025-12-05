# System Patterns

Heimdall's architecture follows modular, pluggable pipeline patterns with a graph-centric data model. Key patterns observed in the codebase:

- Central ingestion API: HTTP endpoints (SvelteKit endpoints + `server/` static/SSR handler) accept uploads and raw payloads. Example: `src/routes/api/upload/+server.ts`.
- Graph-first enrichment: Ingested entities are normalized and stored in a Postgres+AGE graph (see `src/lib/server/ageClient.ts` and SQL schema in `sql/v1`).
- Defensive DB wrapper: `ageClient.ts` wraps `age-schema-client`, providing both higher-level operations and raw cypher fallbacks for compatibility.
- Security-first server wrapper: `server/server.ts` applies Helmet CSP, enforces TLS 1.3, uses secure cookie/session settings, OIDC for auth, and rate-limiting.
- SvelteKit frontend served by Node adapter: Build output placed under `build/` and served via Express static middleware + SvelteKit `handler`.

Operational notes:

- Schema versioning: `HEIMDALL_SCHEMA` is defined in `ageClient.ts` and references SQL in `sql/v1`.
- Idempotent graph ops: create/merge patterns (e.g., `MERGE` in Cypher) used to avoid duplicates.
- Upload handling: streaming to OS temp directory to avoid large memory usage.

Design trade-offs:

- Using Express as a wrapper around SvelteKit keeps deployment flexible but introduces an extra layer to maintain (auth, TLS).
- The code defers to `age-schema-client` when available but includes raw SQL fallbacks for robustness.
