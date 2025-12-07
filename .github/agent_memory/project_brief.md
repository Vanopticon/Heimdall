# Project Brief

- **Name:** Heimdall (vanopticon_heimdall)
- **Purpose:** Central ingestion, ETL, data-scrubbing, normalization, enrichment, and tool-bridging hub for the Vanopticon suite.
- **High-level goals:** Ingest telemetry from vendors/sensors/APIs, transform and scrub to canonical forms (remove or encode NPI), enrich (ASN, geolocation, NPI classification), correlate entities and sightings into a graph model, and provide resilient, auditable pipelines.
- **Key components:**
   	+ HTTP ingest endpoints (streaming/multipart)
   	+ ETL pipeline stages (parsing, canonicalization, enrichment)
   	+ Graph & vector-backed store (Postgres+AGE recommended; `age-schema-client` used in codebase)
   	+ Authentication: OIDC for users, OAuth2 client credentials for M2M
   	+ Server wrapper: Express/SvelteKit in original JavaScript description; Rust codebase uses Axum + Tokio
- **Important files / locations:**
   	+ `README.md` — project overview and run instructions
   	+ `Cargo.toml` — Rust deps and edition
   	+ `src/` — Rust source (entrypoints `main.rs`, `lib.rs`)
   	+ `sql/v1/` — SQL schema and migrations (graph schema)
   	+ `server/` — runtime wrapper (mentioned in README)
- **Owner / Repo:** Vanopticon / Heimdall
- **Current branch:** `v1.0.0`

This brief is intended to provide a quick context snapshot for future agents and contributors.
