# Postgres + Apache AGE + pgvector Local Dev Setup

This project uses PostgreSQL + Apache AGE + `pgvector` as the canonical graph + vector store for Heimdall. A development Docker image and `docker-compose.yml` are included to simplify local development and CI.

Quick start

1. Build and start the container (requires Docker):

```bash
docker compose build db
docker compose up -d db
```

2. Connect with any Postgres client on `localhost:5432` using:

- user: `heimdall`
- password: `heimdall`
- database: `heimdall`

3. The container runs an init script (`docker/postgres-age/initdb/01-extensions.sql`) which attempts to create the `vector` and `ag` extensions and a default AGE graph `heimdall_graph`.

Notes & troubleshooting

- Building Apache AGE and `pgvector` in the Dockerfile can be time-consuming on first build.
- If your platform provides a prebuilt Postgres image with AGE/pgvector, update `docker-compose.yml` to reference that image instead of building locally.
- For CI, prefer caching Docker layers or using a prebuilt base image with the required extensions installed.

If you want, I can also add a small Rust `age_client` wrapper and SQL schema files under `sql/v1/` for the canonical graph schema used by the ingest pipeline.
