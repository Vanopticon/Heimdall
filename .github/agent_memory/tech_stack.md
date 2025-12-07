# Tech Stack

- **Language & edition:** Rust, `edition = "2024"`
- **Runtime:** `tokio` (async runtime)
- **HTTP / Web framework:** `axum` (with `http2`, `macros`, `multipart` features)
- **CLI & config:** `clap`, `config`
- **Logging:** `fern`, `log`, `serde`-enabled logging
- **Serialization:** `serde`, `serde_json`
- **Database / store:** `Postgres` + `Apache AGE` + `pgvector` recommended for graph and vector storage. Use the included `docker-compose.yml` for local dev.
- **Middleware / tower:** `tower`, `tower-http`
- **Testing & tooling:** `vitest` and Svelte testing tooling referenced in README for frontend; Rust tests use built-in test tooling.

- **Notes:** See `Cargo.toml` for exact dependency versions and features.
